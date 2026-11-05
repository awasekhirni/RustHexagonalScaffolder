//! Strongly-typed application settings, loaded by the `config` crate from
//! `config/default.toml` and overridden by `APP__SECTION__KEY` env vars.
//!
//! v0.3.0 (closes G-C03): re-exports the multi-tenant connection-profile
//! types so the runtime pool registry can reference them via
//! `crate::config::ConnectionProfile` without reaching into the
//! `settings` submodule.

pub mod settings;

pub use settings::{
    AuthSettings, ChaosSettings, ConnectionProfile, DatabaseSettings, LogSettings,
    OutboxSettings, PoolCfg, RateLimitSettings, ServerSettings, Settings, SshTunnel,
    SslModeCfg,
};
