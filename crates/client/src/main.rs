mod app;
mod events;
mod transport;
mod ui;

use app::App;
use crossterm::{
    event::{Event as TermEvent, EventStream, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::handle_key;
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use shared::Command;
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

    let result = run(&mut terminal, client, socket_path.clone()).await;

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

    load_initial_data(&client, &mut app).await;

    let (event_tx, mut event_rx) = mpsc::channel::<shared::Event>(256);

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
                    if key.kind == KeyEventKind::Press && app.can_quit()
                        && key.code == crossterm::event::KeyCode::Char('q')
                    {
                        break;
                    }
                    handle_key(&mut app, key);
                    process_pending(&client, &mut app).await;
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

async fn process_pending(client: &DaemonClient, app: &mut App) {
    let cmds: Vec<app::PendingCommand> = app.pending_commands.drain(..).collect();
    for pc in cmds {
        match client.send(pc.command).await {
            Ok(resp) => app.handle_response(resp, pc.context),
            Err(e) => app.set_notification(format!("Erro: {e}"), true),
        }
    }
}

async fn load_initial_data(client: &DaemonClient, app: &mut App) {
    if let Ok(shared::Response::Projects(projects)) = client.send(Command::ProjectList).await {
        app.projects = projects;
    }
}

fn fallback_socket() -> Option<String> {
    std::env::var("HOME").ok().map(|home| {
        format!("{home}/.local/share/rustploy/rustploy.sock")
    })
}

/// Tries each candidate socket path in order, returning the first one where
/// the daemon responds to a ping.
async fn resolve_socket(candidates: &[Option<String>]) -> anyhow::Result<String> {
    for candidate in candidates.iter().flatten() {
        let client = DaemonClient::new(candidate.as_str());
        if client.ping().await {
            return Ok(candidate.clone());
        }
    }
    anyhow::bail!(
        "daemon not reachable; tried: {}",
        candidates
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    )
}
