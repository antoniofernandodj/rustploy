use shared::Message;
use std::error::Error;
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    layout::{Layout, Constraint, Direction},
    Terminal,
    style::{Style, Color},
};
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::path::Path;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::net::UnixStream;
use hyper_util::rt::TokioIo;
use hyper::Request;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;

struct App {
    input: String,
    messages: Vec<String>,
}

/// Cliente minimalista para enviar Bincode via UDS usando Hyper
async fn send_bincode_request(
    socket_path: &Path,
    msg: Message,
) -> Result<Message, Box<dyn Error + Send + Sync>> {
    let stream = UnixStream::connect(socket_path).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {:?}", err);
        }
    });

    let bytes = bincode::serialize(&msg)?;
    let body = Full::new(Bytes::from(bytes));
    let req = Request::builder()
        .method("POST")
        .uri("http://localhost/")
        .header("Content-Type", "application/octet-stream")
        .body(body)?;

    let res = sender.send_request(req).await?;
    let body_bytes = res.collect().await?.to_bytes();
    let resp_msg = bincode::deserialize::<Message>(&body_bytes)?;

    Ok(resp_msg)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let socket_path = std::path::PathBuf::from("/tmp/rustploy_echo.sock");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        input: String::new(),
        messages: Vec::new(),
    };

    let mut event_stream = EventStream::new();
    let (tx, mut rx) = mpsc::channel::<String>(32);

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                ].as_ref())
                .split(f.size());

            let input_widget = Paragraph::new(app.input.as_str())
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Input (Hyper-UDS + Bincode)"));
            f.render_widget(input_widget, chunks[0]);

            let messages: Vec<ListItem> = app.messages
                .iter()
                .rev()
                .map(|m| ListItem::new(m.as_str()))
                .collect();
            let messages_widget = List::new(messages)
                .block(Block::default().borders(Borders::ALL).title("Server Responses"));
            f.render_widget(messages_widget, chunks[1]);
        })?;

        tokio::select! {
            Some(event) = event_stream.next() => {
                let event = event?;
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter => {
                                let content = app.input.drain(..).collect::<String>();
                                let msg = Message { content };
                                let tx_clone = tx.clone();
                                let path_clone = socket_path.clone();

                                tokio::spawn(async move {
                                    match send_bincode_request(&path_clone, msg).await {
                                        Ok(resp) => {
                                            let _ = tx_clone.send(resp.content).await;
                                        }
                                        Err(e) => {
                                            let _ = tx_clone.send(format!("Error: {}", e)).await;
                                        }
                                    }
                                });
                            }
                            KeyCode::Char(c) => {
                                app.input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
                            }
                            KeyCode::Esc => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Some(msg_content) = rx.recv() => {
                app.messages.push(msg_content);
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    Ok(())
}
