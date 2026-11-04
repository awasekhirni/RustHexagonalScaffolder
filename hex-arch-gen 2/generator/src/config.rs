//! Generator runtime configuration.
//!
//! Values may come from the CLI (higher priority) or the environment
//! (`.env` is honoured via `dotenvy`), mirroring the `DATABASE_URL`
//! constant of the original Python script.

use anyhow::{Context, Result};

/// Everything the generator needs to introspect and to stamp the project.
#[derive(Debug, Clone)]
pub struct Config {
    /// PostgreSQL connection string, e.g.
    /// `postgresql://awasekhirnisyed:secret@localhost:5432/profxdb`.
    pub database_url: String,
    /// Database schema to introspect (`public` by default).
    pub schema: String,
    /// Crate name of the generated project.
    pub project_name: String,
}

impl Config {
    /// Builds the configuration from CLI values + environment fallbacks.
    pub fn from_parts(database_url: Option<String>, schema: String, project_name: String) -> Result<Self> {
        let database_url = match database_url {
            Some(url) => url,
            None => std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set (env or .env) or passed via --database-url")?,
        };
        Ok(Self { database_url, schema, project_name })
    }
}

/// Tables that are generator/ORM book-keeping and must never become API
/// resources. Extend through the `HEXGEN_EXCLUDE_TABLES` env var
/// (comma-separated).
pub fn default_excluded_tables() -> Vec<String> {
    vec![
        "schema_migrations".into(),
        "_sqlx_migrations".into(),
        "__diesel_schema_migrations".into(),
        "alembic_version".into(),
        "migrations".into(),
        "pgmigrations".into(),
    ]
}
