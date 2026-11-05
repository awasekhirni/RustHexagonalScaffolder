//! `Settings` — typed configuration loaded at boot.
//!
//! Loaded from `config/default.toml` and the environment (12-factor).
//! Unknown `APP__*` env vars are silently ignored (config crate default);
//! required keys that are missing surface as a typed error.

use serde::Deserialize;

/// Root settings object.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// HTTP server bind address.
    pub server: ServerSettings,
    /// PostgreSQL pool configuration.
    pub database: DatabaseSettings,
    /// Logging verbosity.
    pub log: LogSettings,
    /// Rate limiter (token bucket).
    pub rate_limit: RateLimitSettings,
    /// Outbox worker.
    pub outbox: OutboxSettings,
    /// JWT auth.
    pub auth: AuthSettings,
    /// Chaos engineering (dev/test only).
    pub chaos: ChaosSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub bind_address: String,
}

impl ServerSettings {
    /// Returns the configured bind address.
    #[must_use]
    pub fn bind_addr(&self) -> String {
        self.bind_address.clone()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub migrate_on_start: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogSettings {
    pub level: String,
    pub json: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitSettings {
    pub per_second: u32,
    pub burst: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutboxSettings {
    pub enabled: bool,
    pub batch_size: i64,
    pub poll_interval_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthSettings {
    pub jwt_secret: String,
    pub issuer: String,
    pub audience: String,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChaosSettings {
    pub enabled: bool,
}

// ── v0.3.0 (closes G-C03): multi-tenant connection profiles ──────────────
//
// The pool_registry module (`src/infrastructure/db/pool_registry.rs`)
// holds one PgPool per profile. Profiles are declared in a sibling
// `connection-profiles.toml` file (parsed by `ProfileSet::parse_toml`
// at boot) so secrets can be sourced from env vars via the `${VAR}`
// syntax. The types below mirror the generator's
// `connection_profile` module 1:1 so the runtime and the generator
// share the same schema.

/// TLS mode mirroring `sqlx::postgres::PgSslMode`.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SslModeCfg {
    #[default]
    Disable,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl std::fmt::Display for SslModeCfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SslModeCfg::Disable    => "disable",
            SslModeCfg::Prefer     => "prefer",
            SslModeCfg::Require    => "require",
            SslModeCfg::VerifyCa   => "verify-ca",
            SslModeCfg::VerifyFull => "verify-full",
        };
        f.write_str(s)
    }
}

/// SSH bastion tunnel for databases behind a jump host.
#[derive(Debug, Clone, Deserialize)]
pub struct SshTunnel {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub private_key_path: String,
    pub remote_target: String,
}

/// Pool sizing parameters — forwarded to `sqlx::PoolOptions`.
#[derive(Debug, Clone, Deserialize)]
pub struct PoolCfg {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: Option<u64>,
    pub max_lifetime_secs: Option<u64>,
}

/// One reachable PostgreSQL database. The `id` is the stable key the
/// runtime uses to pick a pool at request time (typically driven by
/// `X-Tenant-Id` or a JWT claim).
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionProfile {
    pub id: String,
    pub url: String,
    pub ssl_mode: SslModeCfg,
    pub ssh_tunnel: Option<SshTunnel>,
    pub introspect_role: Option<String>,
    pub runtime_role: Option<String>,
    pub pool: PoolCfg,
}

impl Settings {
    /// Loads settings from `config/default.toml` + env.
    ///
    /// # Errors
    ///
    /// Returns `anyhow::Error` when the config file is missing or a
    /// required field cannot be deserialised.
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(
                config::Environment::with_prefix("APP")
                    .prefix_separator("__")
                    .separator("__"),
            )
            .build()?;
        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
    }
}
