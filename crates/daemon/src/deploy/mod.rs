//! Motor de deploy: fila global, executor, recuperação no boot, clone git e
//! resolução de env vars.

pub mod env_resolve;
pub mod executor;
pub mod git;
pub mod queue;
pub mod recovery;
pub mod shared_net;
