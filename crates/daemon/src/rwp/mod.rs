//! RWP — Rustploy Wire Protocol: remote administrative TCP channel.
//!
//! This exposes the exact same `Command`/`Response`/`Event` surface as the
//! local Unix-socket API, so any command the TUI can issue is reachable
//! remotely. It reuses the daemon's async `dispatch()` and `EventBus`.
//!
//! Note on the execution model: `docs/protocolo-remoto-binario.md` sketches a
//! synchronous thread-per-connection design for minimal RAM. Because the daemon
//! already runs a full Tokio runtime and every handler (DB via sqlx, the event
//! bus, Docker) is async, we implement the listener on Tokio instead. Bridging
//! async handlers from blocking threads would add a `block_on` per call for no
//! RAM benefit — the runtime already exists. Connection and frame limits keep
//! the footprint bounded.

pub mod server;

pub use server::run;
