//! Runtime configuration resolved from the environment (12-factor style).
//!
//! `.env` is loaded by `dotenvy` at boot; every value can be overridden
//! by real environment variables (container-friendly).

use anyhow::Context;

/// Application settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// PostgreSQL connection string.
    pub database_url: String,
    /// Socket the HTTP server binds to, e.g. `0.0.0.0:8080`.
    pub bind_address: String,
    /// Maximum PostgreSQL pool connections.
    pub db_max_connections: u32,
}

impl Config {
    /// Loads configuration, failing fast when `DATABASE_URL` is missing.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set (see .env.example)")?,
            bind_address: std::env::var("BIND_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            db_max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        })
    }
}
