//! `hex-gen` — Hexagonal Architecture REST API generator for PostgreSQL.
//!
//! This binary is the Rust rewrite of the Python "Hexagonal Flask
//! Architecture — Artefacts generation from database" script.
//!
//! # Pipeline
//!
//! ```text
//!  PostgreSQL (information_schema + pg_catalog)
//!        │  introspect
//!        ▼
//!  IR (Schema → Table → Column/ForeignKey/Analysis)
//!        │  codegen
//!        ▼
//!  ./<output>/  — a compiling Axum + SQLx hexagonal REST API
//!                 (enterprise bus + enterprise repository + OpenAPI 3)
//! ```
//!
//! # Usage
//!
//! ```text
//! export DATABASE_URL=postgresql://user:pass@localhost:5432/profxdb
//! hex-gen --database-url "$DATABASE_URL" --project-name profx-api ./profx-api
//!
//! # or try it without a database:
//! hex-gen --fixture ./generated-sample
//! ```

mod analysis;
mod codegen;
mod config;
mod fixture;
mod idents;
mod introspect;
mod ir;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

/// CLI surface of the generator.
#[derive(Parser, Debug)]
#[command(
    name = "hex-gen",
    version,
    about = "Generate a hexagonal-architecture Rust REST API from a PostgreSQL schema",
    after_help = "Try `hex-gen --fixture ./generated-sample` to see a full project without a database."
)]
struct Cli {
    /// Output directory for the generated Rust project.
    #[arg(default_value = "generated-api")]
    output: PathBuf,

    /// PostgreSQL connection URL (overrides DATABASE_URL env / .env).
    #[arg(long)]
    database_url: Option<String>,

    /// Database schema to introspect.
    #[arg(long, default_value = "public")]
    schema: String,

    /// Crate name of the generated project.
    #[arg(long, default_value = "generated-api")]
    project_name: String,

    /// Generate from the built-in demo schema instead of a live database.
    #[arg(long)]
    fixture: bool,

    /// Overwrite the output directory if it already exists.
    #[arg(long)]
    force: bool,

    /// Comma-separated extra tables to exclude from generation.
    #[arg(long, default_value = "")]
    exclude_tables: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // Fixture mode never touches PostgreSQL — inject a placeholder so the
    // generated `.env`/`Config` stay valid without a live connection string.
    let database_url = if cli.fixture {
        Some("postgresql://user:password@localhost:5432/dbname".to_string())
    } else {
        cli.database_url.clone()
    };

    let cfg = config::Config::from_parts(database_url, cli.schema, cli.project_name)
        .context("resolving configuration")?;

    let mut excluded = config::default_excluded_tables();
    for extra in cli.exclude_tables.split(',') {
        let t = extra.trim();
        if !t.is_empty() {
            excluded.push(t.to_string());
        }
    }

    // ── 1. obtain the IR ────────────────────────────────────────────────────
    let schema = if cli.fixture {
        println!("▶ using built-in fixture schema (no database required)");
        fixture::demo_schema()
    } else {
        let masked = mask_password(&cfg.database_url);
        println!("▶ introspecting {masked} (schema: {})", cfg.schema);
        introspect::introspect(&cfg, &excluded)
            .await
            .context("introspecting the database")?
    };

    if schema.tables.is_empty() {
        anyhow::bail!(
            "no usable tables found — every table needs exactly one primary-key column"
        );
    }

    // ── 2. human summary of what was detected ───────────────────────────────
    println!("✓ introspected {} table(s):", schema.tables.len());
    for t in &schema.tables {
        let pk = t
            .pk()
            .map(|c| format!("{}:{}", c.name, c.kind.rust_type()))
            .unwrap_or_else(|| "<none>".into());
        println!(
            "  • {:<24} pk={:<22} features={}",
            t.name,
            pk,
            if t.analysis.features().is_empty() {
                "-".to_string()
            } else {
                t.analysis.features().join(",")
            }
        );
    }

    // ── 3. emit the project ─────────────────────────────────────────────────
    println!(
        "▶ generating hexagonal project into {} (crate: {})",
        cli.output.display(),
        cfg.project_name
    );
    codegen::generate(&schema, &cli.output, &cfg, cli.force)?;

    let files = count_files(&cli.output);
    println!("✓ wrote {files} files");

    println!();
    println!("next steps:");
    println!("  cd {}", cli.output.display());
    println!("  cargo run            # Swagger UI at http://localhost:8080/swagger-ui");
    Ok(())
}

/// Hides the password when echoing the connection string.
fn mask_password(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let rest = &url[scheme_end + 3..];
        if let Some(at) = rest.rfind('@') {
            let (creds, host) = rest.split_at(at);
            if let Some(colon) = creds.find(':') {
                return format!(
                    "{}://{}:***{}",
                    &url[..scheme_end],
                    &creds[..colon],
                    host
                );
            }
        }
    }
    url.to_string()
}

/// Counts emitted files (README/summary only — cheap enough for CLI output).
fn count_files(root: &std::path::Path) -> usize {
    fn walk(dir: &std::path::Path, count: &mut usize) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, count);
            } else {
                *count += 1;
            }
        }
    }
    let mut n = 0;
    walk(root, &mut n);
    n
}
