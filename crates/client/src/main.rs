mod app;
mod events;
mod transport;
mod ui;

use app::App;
use crossterm::{
    event::{Event as TermEvent, EventStream, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::handle_key;
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use shared::{Command, Response};
use std::{io, time::Duration};
use tokio::{sync::mpsc, time::interval};
use transport::DaemonClient;

const TICK_MS: u64 = 100;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket_path = std::env::var("RUSTPLOY_SOCKET")
        .unwrap_or_else(|_| "/run/rustploy/rustploy.sock".to_string());
    let client = DaemonClient::new(&socket_path);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, client, socket_path.clone()).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    client: DaemonClient,
    socket_path: String,
) -> anyhow::Result<()> {
    let mut app = App::new();

    // Load initial data
    load_initial_data(&client, &mut app).await;

    let (event_tx, mut event_rx) = mpsc::channel::<shared::Event>(256);

    // Background: daemon event stream
    let tx = event_tx.clone();
    let sock = socket_path.clone();
    tokio::spawn(async move {
        let client = DaemonClient::new(&sock);
        let _ = client
            .stream(None, move |ev| {
                let _ = tx.blocking_send(ev);
            })
            .await;
    });

    let mut crossterm_events = EventStream::new();
    let mut tick = interval(Duration::from_millis(TICK_MS));

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        tokio::select! {
            Some(term_ev) = crossterm_events.next() => {
                let ev = term_ev?;
                if let TermEvent::Key(key) = ev {
                    if key.kind == KeyEventKind::Press
                        && key.code == KeyCode::Char('q')
                        && matches!(app.screen, app::Screen::Dashboard)
                    {
                        break;
                    }
                    handle_key(&mut app, &client, key);
                }
            }
            Some(daemon_ev) = event_rx.recv() => {
                app.apply_event(daemon_ev);
            }
            _ = tick.tick() => {
                app.tick();
            }
        }
    }

    Ok(())
}

async fn load_initial_data(client: &DaemonClient, app: &mut App) {
    match client.send(Command::ProjectList).await {
        Ok(Response::Projects(projects)) => {
            app.projects = projects;
        }
        _ => {}
    }

    if let Some(project) = app.projects.first() {
        match client
            .send(Command::ServiceList { project_id: project.id.clone() })
            .await
        {
            Ok(Response::Services(services)) => {
                app.services = services;
            }
            _ => {}
        }
    }
}
