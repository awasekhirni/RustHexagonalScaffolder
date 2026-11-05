//! Structured logging initialisation — `tracing` subscriber setup.
//!
//! v0.3.0 (closes G-C04): re-exports both `init` and `flush` so the
//! generated `main.rs` can drain the subscriber before `process::exit`.

pub mod init;

pub use init::init as init_logging;
pub use init::flush as flush_logging;
