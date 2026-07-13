//! Registry Docker OCI Distribution API v2 embutido — push/pull, GC e Basic
//! auth por token. Ver `docs/plano-registry-embutido.md`.

pub mod auth;
pub mod error;
pub mod gc;
pub mod http;
pub mod name;
pub mod storage;
