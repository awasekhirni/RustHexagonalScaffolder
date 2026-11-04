//! Code generation orchestrator.
//!
//! Emits a complete, compiling Rust project laid out as:
//!
//! ```text
//! <output>/Cargo.toml
//! <output>/.env / .env.example / .gitignore / README.md
//! <output>/src/main.rs                     Axum bootstrap + bus wiring
//! <output>/src/config.rs                   env config
//! <output>/src/error.rs                    HTTP error taxonomy
//! <output>/src/enterprise/
//!     ├── bus.rs                           CommandBus / QueryBus (TypeId dispatch)
//!     └── repository.rs                    Repository<T, ID> + Page/Paginated
//! <output>/src/domain/<table>/
//!     ├── entity.rs                        FromRow entity
//!     ├── enums.rs                         native PG enums (when used)
//!     ├── input.rs                         Create/Update input models
//!     ├── repository.rs                    outbound port (trait)
//!     └── service.rs                       inbound port (trait)
//! <output>/src/application/
//!     ├── <table>_mapper.rs                input → entity translation
//!     ├── <table>_service.rs               service adapter (impl of port)
//!     └── <table>_messaging.rs             commands/queries + bus handlers
//! <output>/src/infrastructure/
//!     └── <table>_repository.rs            SQLx adapter (impl of port)
//! <output>/src/api/
//!     ├── <table>/{dto.rs, handler.rs}
//!     ├── state.rs / routes.rs / openapi.rs
//! ```

pub mod api;
pub mod application;
pub mod core_static;
pub mod domain;
pub mod infrastructure;
pub mod main_app;
pub mod project_files;

use std::path::Path;

use anyhow::{bail, Result};

use crate::config::Config;
use crate::ir::{Column, Schema};

/// Generates the whole project into `out` (created fresh).
pub fn generate(schema: &Schema, out: &Path, cfg: &Config, force: bool) -> Result<()> {
    if out.exists() {
        if !force {
            bail!(
                "output directory {} already exists — pass --force to overwrite",
                out.display()
            );
        }
        std::fs::remove_dir_all(out)?;
    }

    project_files::write_all(schema, out, cfg)?;
    core_static::write_all(out)?;

    for table in &schema.tables {
        domain::write_table(schema, out, table)?;
        application::write_table(schema, out, table)?;
        infrastructure::write_table(schema, out, table)?;
        api::write_table(out, table)?;
    }

    domain::write_mod(schema, out)?;
    application::write_mod(schema, out)?;
    infrastructure::write_mod(schema, out)?;
    api::write_shared(schema, out)?;
    main_app::write(schema, out, cfg)?;
    Ok(())
}

/// Creates parent directories and writes a file.
pub fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
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
            && l.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    })
}
