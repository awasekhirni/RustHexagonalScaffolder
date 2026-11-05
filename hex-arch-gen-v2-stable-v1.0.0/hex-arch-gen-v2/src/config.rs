// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Generator configuration.
//!
//! Resolved from CLI flags + environment + `.env`. Kept deliberately small:
//! every value the codegen templates need is on the [`Config`] struct, so
//! templates never read env vars directly (deterministic generation).

use crate::errors::{GenError, GenResult};

/// The resolved generator configuration.
///
/// Passed by reference to every codegen function; templates read fields,
/// never env vars, so the same `Config` always yields byte-identical output.
#[derive(Debug, Clone)]
pub struct Config {
    /// PostgreSQL connection string used by the live introspector and
    /// emitted into the generated `.env`/`Settings`.
    pub database_url: String,
    /// Schema introspected (`public` by default).
    pub schema: String,
    /// Crate name of the generated project (`generated-api` by default).
    pub project_name: String,
}

impl Config {
    /// Builds a [`Config`] from CLI parts.
    ///
    /// In fixture mode the caller passes `Some(placeholder)` for the URL;
    /// in live mode the caller passes the real URL or relies on
    /// `DATABASE_URL` env.
    ///
    /// # Errors
    ///
    /// Returns [`GenError::MissingDatabaseUrl`] when no URL is available,
    /// or [`GenError::InvalidProjectName`] when the project name contains
    /// characters that would produce an invalid crate name.
    pub fn from_parts(
        database_url: Option<String>,
        schema: String,
        project_name: String,
    ) -> GenResult<Self> {
        // Validate the project name: must be a valid Cargo crate name.
        // We allow `[a-zA-Z][a-zA-Z0-9_-]*` — slightly stricter than Cargo
        // but matches every generated identifier.
        let valid = project_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
            && project_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !valid {
            return Err(GenError::InvalidProjectName { name: project_name });
        }

        let database_url = match database_url {
            Some(url) => url,
            None => std::env::var("DATABASE_URL").map_err(|_| GenError::MissingDatabaseUrl)?,
        };

        Ok(Self {
            database_url,
            schema,
            project_name,
        })
    }
}

/// Tables always excluded from generation: SQLx migration bookkeeping
/// and any table whose name begins with `sqlx_migrations` (versioned).
///
/// User-supplied exclusions are merged on top of this list in `main`.
pub fn default_excluded_tables() -> Vec<String> {
    vec![
        "sqlx_migrations".into(),
        "__sqlx_migrations".into(),
        "database_migration_lock".into(),
        "_sqlx_migrations".into(),
    ]
}
