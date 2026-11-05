// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Code generation orchestrator.
//!
//! Emits a complete, compiling Rust project laid out as the v0.6.0
//! desired-state platform:
//!
//! ```text
//! <output>/Cargo.toml  /  .env  /  .env.example  /  .gitignore  /  README.md
//! <output>/migrations/20240101000001_init_schema.sql
//! <output>/config/default.toml
//! <output>/src/
//!   ├── lib.rs                        crate root re-exports
//!   ├── main.rs                       axum bootstrap + wiring
//!   ├── shared/{error,result,pagination,mod}.rs        cross-cutting types
//!   ├── config/{settings,mod}.rs                       typed settings
//!   ├── logging/{init,mod}.rs                          tracing init
//!   ├── telemetry.rs                                   OTLP + Prometheus (stub)
//!   ├── core/{error_codes,ids,clock,aggregate,value_objects/*,unit_of_work,
//!   │        shutdown,startup,bulkhead,saga,chaos,events,mod}.rs
//!   ├── patterns/{idempotency,outbox_worker,outbox_dlq,circuit_breaker,retry,
//!   │            pagination,filtering,webhooks,cache,feature_flags,load_shed,mod}.rs
//!   ├── security/{rls,gdpr,pii,secrets,headers,tenancy,auth/*,mod}.rs
//!   ├── domain/entities/<table>.rs                     per-table aggregate root
//!   ├── domain/value_objects/{email,id,mod}.rs
//!   ├── application/{dto/<table>_dto.rs, services/<table>_service.rs,
//!   │                ports/{bus,repository,mod}.rs, sagas/mod.rs, mod.rs}
//!   ├── infrastructure/{persistence/<table>_repository.rs, db/pool.rs,
//!   │                    bus/enterprise_bus.rs, timed_query.rs, mod.rs}
//!   ├── presentation/{state.rs, routes.rs, error.rs, middleware/*, handlers/<table>_handler.rs, mod.rs}
//!   └── api/{openapi.rs, mod.rs}
//! ```
//!
//! Per-table files depend on the IR; platform files (`core`, `patterns`,
//! `security`, `shared`, `logging`, `telemetry`, `config`) are emitted
//! verbatim from [`assets`] with light substitution.

pub mod api;
pub mod application;
pub mod assets;
pub mod domain;
pub mod domain_helpers;
pub mod graphql;
pub mod infrastructure;
pub mod migrations;
pub mod pool_registry;
pub mod presentation;
pub mod project_files;
pub mod test_harness;
pub mod wiring;

use std::path::Path;

use tracing::{info, instrument};

use crate::config::Config;
use crate::errors::{GenError, GenResult};
use crate::ir::{Column, Schema};

/// Statistics returned by [`generate`] for the CLI summary and the
/// on-disk `GENERATION_REPORT.md`.
#[derive(Debug, Clone, Default)]
pub struct GenerationStats {
    /// Number of files written (recursive).
    pub files_written: usize,
    /// Total bytes written across all files.
    pub bytes_written: u64,
}

/// Generates the whole project into `out` (created fresh).
///
/// # Errors
///
/// * [`GenError::OutputExists`] — `out` exists and `force` is `false`.
/// * [`GenError::Io`] — any filesystem failure.
/// * [`GenError::CodegenNoPk`] — a table reached codegen without a PK.
#[instrument(skip(schema, cfg), fields(tables = schema.tables.len()), err)]
pub fn generate(
    schema: &Schema,
    out: &Path,
    cfg: &Config,
    force: bool,
) -> GenResult<GenerationStats> {
    if out.exists() {
        if !force {
            return Err(GenError::OutputExists {
                path: out.to_path_buf(),
            });
        }
        info!(path = %out.display(), "removing existing output directory (--force)");
        std::fs::remove_dir_all(out).map_err(|source| GenError::Io {
            path: out.to_path_buf(),
            source,
        })?;
    }

    // Phase 1: project-level files (Cargo.toml, .env, README, etc.).
    project_files::write_all(schema, out, cfg)?;
    // Phase 2: platform/static assets (core, patterns, security, shared, …).
    assets::write_all(out, cfg)?;
    // Phase 3: DDL migration.
    migrations::write_all(schema, out)?;
    // Phase 3b (v0.3.0, G-C03): multi-tenant pool registry.
    pool_registry::write_all(schema, out)?;
    // Phase 3c (v0.3.0, G-C02): GraphQL surface (SDL + Query/Mutation
    // roots + per-table resolvers). DataLoader and Federation are
    // emitted as documented skeletons — full impls land in v0.4.0.
    graphql::write_all(schema, out)?;
    // Phase 3d (v0.4.0, G-H09): per-project integration smoke tests.
    test_harness::write_all(schema, out)?;

    // Phase 4: per-table layers.
    for table in &schema.tables {
        info!(table = %table.name, "emitting per-table artefacts");
        domain::write_table(schema, out, table)?;
        application::write_table(schema, out, table)?;
        infrastructure::write_table(schema, out, table)?;
        presentation::write_table(schema, out, table)?;
    }

    // Phase 5: aggregate `mod.rs` files and wiring.
    domain::write_mod(schema, out)?;
    application::write_mod(schema, out)?;
    infrastructure::write_mod(schema, out)?;
    presentation::write_shared(schema, out)?;
    api::write_shared(schema, out)?;
    wiring::write(schema, out, cfg)?;

    let stats = count_files(out);
    info!(
        files = stats.files_written,
        bytes = stats.bytes_written,
        "generation stats"
    );
    Ok(stats)
}

/// Creates parent directories and writes a file, returning the byte count
/// for statistics. The caller aggregates counts via [`count_files`].
pub fn write_file(path: &Path, contents: &str) -> GenResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|source| GenError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }
    std::fs::write(path, contents).map_err(|source| GenError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

/// Recursively counts files and bytes under `root`.
fn count_files(root: &Path) -> GenerationStats {
    let mut stats = GenerationStats::default();
    fn walk(dir: &Path, stats: &mut GenerationStats) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, stats);
            } else if let Ok(meta) = path.metadata() {
                stats.files_written += 1;
                stats.bytes_written += meta.len();
            }
        }
    }
    walk(root, &mut stats);
    stats
}

/// Import-set accumulator shared by all templates.
#[derive(Default, Clone, Debug)]
pub struct ImportSet {
    pub uuid: bool,
    pub json: bool,
    pub decimal: bool,
    pub chrono_utc: bool,
    pub chrono_date: bool,
    pub chrono_datetime: bool,
    pub chrono_time: bool,
}

impl ImportSet {
    /// Scans a set of columns for the Rust types they need.
    pub fn from_cols<'a>(cols: impl IntoIterator<Item = &'a Column>) -> Self {
        let mut set = ImportSet::default();
        for c in cols {
            set.absorb_kind(&c.kind);
        }
        set
    }

    /// Absorbs one logical type.
    pub fn absorb_kind(&mut self, kind: &crate::ir::ColKind) {
        use crate::ir::ColKind;
        match kind {
            ColKind::Uuid => self.uuid = true,
            ColKind::Json => self.json = true,
            ColKind::Decimal => self.decimal = true,
            ColKind::DateTimeUtc => self.chrono_utc = true,
            ColKind::NaiveDate => self.chrono_date = true,
            ColKind::NaiveDateTime => self.chrono_datetime = true,
            ColKind::NaiveTime => self.chrono_time = true,
            ColKind::VecOf(inner) => self.absorb_kind(inner),
            _ => {}
        }
    }

    /// Renders chrono import (single merged `use`) when needed.
    fn chrono_line(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if self.chrono_utc {
            parts.push("DateTime".into());
            parts.push("Utc".into());
        }
        if self.chrono_date {
            parts.push("NaiveDate".into());
        }
        if self.chrono_datetime {
            parts.push("NaiveDateTime".into());
        }
        if self.chrono_time {
            parts.push("NaiveTime".into());
        }
        if parts.is_empty() {
            None
        } else {
            parts.sort();
            Some(format!("use chrono::{{{}}};", parts.join(", ")))
        }
    }

    /// Renders all type imports (sorted, without enum imports which are
    /// module-specific).
    pub fn render(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(line) = self.chrono_line() {
            lines.push(line);
        }
        if self.decimal {
            lines.push("use rust_decimal::Decimal;".into());
        }
        if self.json {
            lines.push("use serde_json::Value as JsonValue;".into());
        }
        if self.uuid {
            lines.push("use uuid::Uuid;".into());
        }
        lines
    }
}

/// `true` when every label is lowercase snake_case (safe for
/// `rename_all = "snake_case"` on both serde and sqlx).
pub fn labels_snake_case(labels: &[String]) -> bool {
    labels.iter().all(|l| {
        let mut chars = l.chars();
        matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
            && l.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    })
}
