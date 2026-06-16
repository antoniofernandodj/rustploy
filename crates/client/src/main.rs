mod app;
mod cli;
mod events;
mod models;
mod transport;
mod ui;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use app::{App, CmdContext, PendingCommand};
use crossterm::{
    event::{self, Event as TermEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use events::handle_key;
use ratatui::{Terminal, backend::CrosstermBackend};
use shared::{Command, Response};
use std::{
    io,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use transport::DaemonClient;

const TICK_MS: u64 = 100;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "import" => return handle_import(&args[2..]),
            "apply" => return cli::run_apply(&args[2..]),
            "export" => return cli::run_export(&args[2..]),
            _ => {}
        }
    }

    let socket_path = resolve_socket(&shared::RustployConfig::global().client_socket_candidates())?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, socket_path);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn handle_import(args: &[String]) -> anyhow::Result<()> {
    let mut cmd_name = "rustploy-import".to_string();

    // In dev mode, the binary might be in the same dir
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let local_bin = parent.join("rustploy-import");
            if local_bin.exists() {
                cmd_name = local_bin.to_string_lossy().to_string();
            }
        }
    }

    let status = std::process::Command::new(cmd_name).args(args).status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    socket_path: String,
) -> anyhow::Result<()> {
    let mut app = App::new();

    // Load initial data synchronously before entering the loop
    let client = DaemonClient::new(&socket_path);
    if let Ok(Response::Projects(projects)) = client.send(Command::ProjectList) {
        app.projects = projects;
    }

    // Channel for daemon stream events
    let (event_tx, event_rx) = mpsc::sync_channel::<shared::Event>(256);
    // Channel for RPC responses — commands are dispatched as background threads
    let (resp_tx, resp_rx) = mpsc::sync_channel::<(Response, CmdContext)>(64);

    // Daemon event stream thread
    {
        let sock = socket_path.clone();
        let tx = event_tx.clone();
        thread::spawn(move || {
            let client = DaemonClient::new(&sock);
            let _ = client.stream(None, move |ev| {
                let _ = tx.try_send(ev);
            });
        });
    }

    let tick = Duration::from_millis(TICK_MS);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        let timeout = tick.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let TermEvent::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && app.can_quit()
                    && key.code == crossterm::event::KeyCode::Char('q')
                {
                    break;
                }
                handle_key(&mut app, key);
                dispatch_pending(&socket_path, &mut app, &resp_tx);
            }
        }

        while let Ok(daemon_ev) = event_rx.try_recv() {
            app.apply_event(daemon_ev);
        }
        while let Ok((resp, ctx)) = resp_rx.try_recv() {
            app.handle_response(resp, ctx);
        }
        dispatch_pending(&socket_path, &mut app, &resp_tx);

        if last_tick.elapsed() >= tick {
            app.tick();
            dispatch_pending(&socket_path, &mut app, &resp_tx);
            last_tick = Instant::now();
        }
    }

    Ok(())
}

/// Drains pending commands and spawns each one on an independent background thread.
/// The main loop is never blocked — responses arrive via `resp_tx`.
fn dispatch_pending(
    socket_path: &str,
    app: &mut App,
    resp_tx: &mpsc::SyncSender<(Response, CmdContext)>,
) {
    let cmds: Vec<PendingCommand> = app.pending_commands.drain(..).collect();
    for pc in cmds {
        let sock = socket_path.to_string();
        let tx = resp_tx.clone();
        thread::spawn(move || {
            let client = DaemonClient::new(&sock);
            let resp = match client.send(pc.command) {
                Ok(r) => r,
                Err(e) => Response::err("RpcError", e.to_string()),
            };
            let _ = tx.send((resp, pc.context));
        });
    }
}

fn resolve_socket(candidates: &[String]) -> anyhow::Result<String> {
    for path in candidates {
        let client = DaemonClient::new(path);
        if client.ping() {
            return Ok(path.clone());
        }
    }

    anyhow::bail!(
        "daemon não encontrado.\n\
         Inicie o daemon primeiro: rustployd\n\
         Caminhos tentados: {}",
        candidates.join(", ")
    )
}
