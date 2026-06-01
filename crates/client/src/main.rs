mod app;
mod events;
mod models;
mod transport;
mod ui;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use app::{App, CmdContext, PendingCommand};
use crossterm::{
    event::{Event as TermEvent, EventStream, KeyEventKind},
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
    let socket_path = resolve_socket(&[
        std::env::var("RUSTPLOY_SOCKET").ok(),
        Some("/run/rustploy/rustploy.sock".into()),
        fallback_socket(),
    ])
    .await?;
    let client = DaemonClient::new(&socket_path);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, socket_path.clone()).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Ensure initial data load doesn't leave terminal in bad state on error
    let _ = client;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    socket_path: String,
) -> anyhow::Result<()> {
    let mut app = App::new();

    // Load initial data synchronously before entering the loop
    let client = DaemonClient::new(&socket_path);
    if let Ok(Response::Projects(projects)) = client.send(Command::ProjectList).await {
        app.projects = projects;
    }

    // Channel for daemon stream events
    let (event_tx, mut event_rx) = mpsc::channel::<shared::Event>(256);
    // Channel for RPC responses — commands are dispatched as background tasks
    let (resp_tx, mut resp_rx) = mpsc::channel::<(Response, CmdContext)>(64);

    // Daemon event stream task
    {
        let sock = socket_path.clone();
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let client = DaemonClient::new(&sock);
            let _ = client.stream(None, move |ev| { let _ = tx.try_send(ev); }).await;
        });
    }

    let mut crossterm_events = EventStream::new();
    let mut tick = interval(Duration::from_millis(TICK_MS));

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        tokio::select! {
            Some(term_ev) = crossterm_events.next() => {
                let ev = term_ev?;
                if let TermEvent::Key(key) = ev {
                    if key.kind == KeyEventKind::Press
                        && app.can_quit()
                        && key.code == crossterm::event::KeyCode::Char('q')
                    {
                        break;
                    }
                    handle_key(&mut app, key);
                    dispatch_pending(&socket_path, &mut app, resp_tx.clone());
                }
            }
            Some(daemon_ev) = event_rx.recv() => {
                app.apply_event(daemon_ev);
                dispatch_pending(&socket_path, &mut app, resp_tx.clone());
            }
            Some((resp, ctx)) = resp_rx.recv() => {
                app.handle_response(resp, ctx);
                dispatch_pending(&socket_path, &mut app, resp_tx.clone());
            }
            _ = tick.tick() => {
                app.tick();
                dispatch_pending(&socket_path, &mut app, resp_tx.clone());
            }
        }
    }

    Ok(())
}

/// Drains pending commands and spawns each one as an independent background task.
/// The main loop is never blocked — responses arrive via `resp_tx`.
fn dispatch_pending(
    socket_path: &str,
    app: &mut App,
    resp_tx: mpsc::Sender<(Response, CmdContext)>,
) {
    let cmds: Vec<PendingCommand> = app.pending_commands.drain(..).collect();
    for pc in cmds {
        let sock = socket_path.to_string();
        let tx = resp_tx.clone();
        tokio::spawn(async move {
            let client = DaemonClient::new(&sock);
            let resp = match client.send(pc.command).await {
                Ok(r) => r,
                Err(e) => Response::err("RpcError", e.to_string()),
            };
            let _ = tx.send((resp, pc.context)).await;
        });
    }
}

fn fallback_socket() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .map(|home| format!("{home}/.local/share/rustploy/rustploy.sock"))
}

async fn resolve_socket(candidates: &[Option<String>]) -> anyhow::Result<String> {
    let paths: Vec<&str> = candidates.iter().flatten().map(String::as_str).collect();

    for path in &paths {
        let client = DaemonClient::new(path);
        if client.ping().await {
            return Ok(path.to_string());
        }
    }

    anyhow::bail!(
        "daemon não encontrado.\n\
         Inicie o daemon primeiro: rustployd\n\
         Caminhos tentados: {}",
        paths.join(", ")
    )
}
