// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//
//! Connection-profile schema for multi-tenant Postgres connectivity.
//!
//! Closes gap **G-C03**. The generator previously accepted a single
//! `DATABASE_URL` and a single `schema`; this module adds a serialisable
//! profile type that lets a single generator invocation target N databases,
//! each with its own TLS mode, SSH tunnel, introspection role, and pool
//! sizing. The generated runtime picks up the same type via the
//! `pool_registry.rs` asset (see `codegen::pool_registry`).
//!
//! # CMMI mapping
//!
//! * **OPF SG 2** — Deploy organisational process assets: the profile
//!   file is the process asset that captures the connection topology.
//! * **OPP SG 1** — Establish performance baselines: per-tenant
//!   baselines require per-tenant pools, which require profiles.

use serde::{Deserialize, Serialize};

/// One reachable PostgreSQL database.
///
/// The `id` field is the stable key the runtime uses to pick a pool at
/// request time (typically driven by `X-Tenant-Id` or a JWT claim).
/// `url` may reference environment variables via the `${VAR}` syntax;
/// expansion happens at runtime so secrets never land in version control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    /// Stable key used by handlers to look up a pool.
    pub id: String,
    /// Full DSN, e.g. `postgresql://user:${PGPASS}@host:5432/db`.
    pub url: String,
    /// TLS mode for the connection.
    pub ssl_mode: SslModeCfg,
    /// Optional SSH bastion tunnel for databases behind a jump host.
    pub ssh_tunnel: Option<SshTunnel>,
    /// Postgres role assumed before any `pg_catalog` introspection
    /// query. Lets the generator connect as a privileged user but
    /// introspect through the same role the runtime will use.
    pub introspect_role: Option<String>,
    /// Postgres role assumed per request by the runtime. The runtime
    /// emits `SET ROLE "<role>"` on connection acquire and
    /// `RESET ROLE` on Drop. See `pool_registry::ScopedConn`.
    pub runtime_role: Option<String>,
    /// Pool sizing parameters.
    pub pool: PoolCfg,
}

/// TLS mode mirroring `sqlx::postgres::PgSslMode`.
///
/// The string form matches Postgres' `sslmode` query parameter so the
/// same enum round-trips through both the generator and the runtime.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SslModeCfg {
    /// No TLS. Default for local dev only.
    #[default]
    Disable,
    /// Try TLS, fall back to plaintext if the server does not support it.
    Prefer,
    /// Require TLS; fail if the server does not support it.
    Require,
    /// Require TLS and verify the CA against the system trust store.
    VerifyCa,
    /// Require TLS, verify the CA, and verify the server hostname.
    VerifyFull,
}

impl std::fmt::Display for SslModeCfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SslModeCfg::Disable => "disable",
            SslModeCfg::Prefer => "prefer",
            SslModeCfg::Require => "require",
            SslModeCfg::VerifyCa => "verify-ca",
            SslModeCfg::VerifyFull => "verify-full",
        };
        f.write_str(s)
    }
}

/// SSH bastion tunnel configuration for databases behind a jump host.
///
/// The runtime opens a local TCP listener and forwards every byte to
/// `remote_target` via the bastion. The `private_key_path` is loaded
/// once at pool-build time and never logged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshTunnel {
    /// Bastion hostname.
    pub host: String,
    /// Bastion SSH port (typically 22).
    pub port: u16,
    /// SSH user on the bastion.
    pub user: String,
    /// Path to the private key. `${ENV_VAR}` expansion at runtime.
    pub private_key_path: String,
    /// `host:port` of the database as visible from the bastion.
    pub remote_target: String,
}

/// Pool sizing parameters — forwarded to `sqlx::PoolOptions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolCfg {
    /// Maximum number of connections the pool will open.
    pub max_connections: u32,
    /// Minimum number of idle connections the pool maintains.
    pub min_connections: u32,
    /// Timeout for `pool.acquire()` in seconds.
    pub acquire_timeout_secs: u64,
    /// Idle connection timeout (None = no timeout).
    pub idle_timeout_secs: Option<u64>,
    /// Maximum connection lifetime (None = no max lifetime).
    pub max_lifetime_secs: Option<u64>,
}

impl Default for PoolCfg {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_secs: 5,
            idle_timeout_secs: Some(600),
            max_lifetime_secs: Some(3600),
        }
    }
}

/// A bundle of profiles, typically loaded from `connection-profiles.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileSet {
    pub profiles: Vec<ConnectionProfile>,
}

impl ProfileSet {
    /// Parse a TOML string into a [`ProfileSet`].
    ///
    /// Expected shape:
    /// ```toml
    /// [[profiles]]
    /// id = "tenant-a"
    /// url = "postgresql://user:${PGPASS}@host-a:5432/db"
    /// ssl_mode = "require"
    /// introspect_role = "introspect_ro"
    /// runtime_role = "tenant_a_app"
    ///
    /// [profiles.pool]
    /// max_connections = 20
    /// min_connections = 2
    /// acquire_timeout_secs = 5
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is malformed or fails the serde
    /// schema.
    pub fn parse_toml(src: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(src)?)
    }

    /// Look up a profile by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ConnectionProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }
}
