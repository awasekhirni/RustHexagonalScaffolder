// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! `hex-gen` — production-grade db-first generator for hexagonal-architecture
//! Rust REST APIs (v2).
//!
//! # Pipeline
//!
//! ```text
//!   PostgreSQL (information_schema + pg_catalog)         OR    built-in fixture
//!         │  introspect v2 (tracing, error taxonomy)            │  fixture::demo_schema
//!         ▼                                                      ▼
//!   IR v2 (Schema → Table → Column / ForeignKey / Analysis v2)
//!         │  analysis v2 (status, soft-delete, tenancy, audit, timestamps, money, enum, FK)
//!         ▼
//!   codegen (assets + domain + application + infrastructure + presentation
//!            + api + wiring + migrations + project_files)
//!         │
//!         ▼
//!   ./<output>/  — a compiling Axum + SQLx hexagonal REST API
//!                  matching the v0.6.0 desired-state platform:
//!                  presentation/application/domain/infrastructure
//!                  + shared/core/patterns/security/api/telemetry/logging/config
//!                  + OpenAPI 3, JWT auth, cursor pagination,
//!                    ETag/If-Match concurrency, saga hooks,
//!                    structured logging, tracing.
//! ```
//!
//! # Usage
//!
//! ```text
//! # Try it without a database:
//! hex-gen --fixture ./generated-sample
//!
//! # Live db-first run:
//! export DATABASE_URL=postgresql://user:pass@localhost:5432/profxdb
//! hex-gen --database-url "$DATABASE_URL" --project-name profx-api ./profx-api
//! ```
//!
//! Every generator function emits structured `tracing` logs and surfaces
//! failures through a typed [`GenError`] taxonomy. All errors encountered
//! during development are recorded in `ERROR_INVENTORY.md` at the project
//! root and used as a navigable knowledge base for future iterations.

mod analysis;
mod codegen;
mod config;
mod connection_profile;
mod ddl;
mod error_inventory;
mod errors;
mod fixture;
mod idents;
mod introspect;
mod ir;
mod logging;
mod report;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing::{info, warn};

use crate::errors::{mask_password, GenError, GenResult};

/// CLI surface of the generator.
///
/// Every flag is documented inline so `hex-gen --help` doubles as a quick
/// reference; the long-form docs live in the project README.
#[derive(Parser, Debug)]
#[command(
    name = "hex-gen",
    version,
    about = "Generate a production-grade hexagonal-architecture Rust REST API from a PostgreSQL schema (db-first) or the built-in fixture.",
    long_about = "Introspects a live PostgreSQL schema (or uses the built-in fixture) and emits a compiling Axum + SQLx hexagonal REST API matching the v0.6.0 desired-state platform. Every generation step is logged with tracing; all failures surface through a typed GenError taxonomy.",
    after_help = "Try `hex-gen --fixture ./generated-sample` to see a full project without a database."
)]
pub struct Cli {
    /// Output directory for the generated Rust project.
    #[arg(default_value = "generated-api", env = "HEX_GEN_OUTPUT")]
    output: PathBuf,

    /// PostgreSQL connection URL (overrides DATABASE_URL env / .env).
    /// Ignored in `--fixture` mode.
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Database schema to introspect.
    #[arg(long, default_value = "public", env = "HEX_GEN_SCHEMA")]
    schema: String,

    /// Path to a `connection-profiles.toml` file declaring one or more
    /// reachable databases (closes gap G-C03). When present, the
    /// generator introspects the FIRST profile in the file (using that
    /// profile's `introspect_role` if declared) and emits a project
    /// whose runtime pool registry can hold every profile.
    ///
    /// Mutually exclusive with `--database-url`; passing both is an
    /// error.
    #[arg(long, env = "HEX_GEN_PROFILES")]
    profiles: Option<PathBuf>,

    /// Crate name of the generated project (also the binary name).
    #[arg(long, default_value = "generated-api", env = "HEX_GEN_PROJECT_NAME")]
    project_name: String,

    /// Generate from the built-in demo schema instead of a live database.
    /// Useful for evaluation, CI smoke tests, and offline development.
    #[arg(long, env = "HEX_GEN_FIXTURE")]
    fixture: bool,

    /// Overwrite the output directory if it already exists.
    /// Without this flag, an existing directory is a hard error.
    #[arg(long, env = "HEX_GEN_FORCE")]
    force: bool,

    /// Comma-separated extra tables to exclude from generation
    /// (in addition to the always-excluded infrastructure tables:
    /// `sqlx_migrations`, `__sqlx_migrations`, `database_migration_lock`).
    #[arg(long, default_value = "", env = "HEX_GEN_EXCLUDE_TABLES")]
    exclude_tables: String,

    /// v1.0.0 (closes G-M14): comma-separated table names to INCLUDE
    /// in generation. When set, every other table is excluded. Useful
    /// for generating a partial API from a large schema. Each name is
    /// validated against `[a-zA-Z_][a-zA-Z0-9_]*` (same guard as
    /// `--exclude-tables`).
    #[arg(long, default_value = "", env = "HEX_GEN_INCLUDE_TABLES")]
    include_tables: String,

    /// v1.0.0 (closes G-M15): validate the IR (introspect → analysis →
    /// conflict detection) and print the summary WITHOUT writing any
    /// files. Useful for CI checks and for previewing what would be
    /// generated before committing to a full run.
    #[arg(long, env = "HEX_GEN_DRY_RUN")]
    dry_run: bool,

    /// Emit JSON-formatted logs instead of human-readable ones
    /// (12-factor friendly — pipe straight into `jq` or a log shipper).
    #[arg(long, env = "HEX_GEN_JSON_LOGS")]
    json_logs: bool,

    /// Raise the log verbosity (`-v` = info, `-vv` = debug, `-vvv` = trace).
    #[arg(short, long, action = clap::ArgAction::Count, env = "HEX_GEN_VERBOSE")]
    verbose: u8,

    /// Error-inventory subcommand (see `ErrorsCmd`). When present, the
    /// generator skips the normal introspect→codegen pipeline and runs
    /// the requested inventory operation instead. Closes gap G-H03.
    #[command(subcommand)]
    command: Option<ErrorsCmd>,
}

/// `hex-gen errors …` — manage the machine-readable error inventory.
///
/// Closes gap G-H03. The inventory lives at
/// `<output>/error_inventory.json` (and a Markdown view is rendered
/// next to it). Every mutation is logged via `tracing` with
/// `category = "error_inventory"` so the audit trail of inventory
/// edits is itself observable (CMMI MA SG 2 "collect data").
#[derive(Debug, Subcommand)]
pub enum ErrorsCmd {
    /// List entries in the inventory, optionally filtered by status.
    List {
        /// Filter by status: `open` | `resolved` | `deferred` | `superseded`.
        #[arg(long)]
        status: Option<String>,
        /// Path to the inventory JSON file. Defaults to `./ERROR_INVENTORY.json`.
        #[arg(long, default_value = "./ERROR_INVENTORY.json")]
        inventory: PathBuf,
    },

    /// Add a new entry. The next sequential `ERR-NNN` id is assigned
    /// automatically.
    Add {
        #[arg(long)]
        title: String,
        #[arg(long)]
        component: String,
        #[arg(long, value_enum)]
        severity: error_inventory::Severity,
        #[arg(long)]
        error_type: String,
        #[arg(long)]
        message: String,
        #[arg(long)]
        root_cause: String,
        /// Path to the inventory JSON file. Defaults to `./ERROR_INVENTORY.json`.
        #[arg(long, default_value = "./ERROR_INVENTORY.json")]
        inventory: PathBuf,
    },

    /// Mark an existing entry as resolved.
    Resolve {
        #[arg(long)]
        id: String,
        #[arg(long)]
        resolution: String,
        #[arg(long)]
        lessons: Option<String>,
        #[arg(long, default_value = "./ERROR_INVENTORY.json")]
        inventory: PathBuf,
    },

    /// Print summary statistics grouped by status / severity / CMMI PA.
    Stats {
        #[arg(long, default_value = "./ERROR_INVENTORY.json")]
        inventory: PathBuf,
    },
}

/// Entry point — bootstraps logging, parses CLI, dispatches fixture/live mode.
#[tokio::main]
async fn main() -> GenResult<()> {
    // dotenvy is best-effort: in production the env is injected by the
    // orchestrator; in dev it is convenient to read .env.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    // Initialise structured logging as the very first action so every
    // subsequent step is observable. Verbosity is derived from -v / RUST_LOG.
    logging::init(cli.verbose, cli.json_logs);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        output = %cli.output.display(),
        fixture = cli.fixture,
        project_name = %cli.project_name,
        schema = %cli.schema,
        "hex-gen starting"
    );

    // Dispatch the error-inventory subcommand before the normal pipeline
    // so `hex-gen errors …` never tries to introspect a database.
    if let Some(cmd) = cli.command {
        return run_errors_cmd(cmd).await;
    }

    // Run the generator; convert any typed GenError into a process exit.
    if let Err(err) = run(&cli).await {
        // Log the full error chain before exiting — never swallow the cause.
        // Includes the stable `error_code` so the log line correlates to a
        // row in `error_inventory.json` (CMMI CAR SG 1).
        use std::error::Error as _;
        let root: Option<&dyn std::error::Error> = err.source();
        tracing::error!(
            error = %err,
            root = ?root,
            error_code = err.error_code(),
            cmmi_pa = err.cmmi_pa(),
            "generator failed"
        );
        // Print a friendly summary on stderr too, so logs aren't required to
        // understand a failure.
        eprintln!("hex-gen [{}] {}: {}", err.error_code(), err.cmmi_pa(), err);
        // Flush the tracing subscriber before exit so the final error line
        // is never lost (closes gap G-L12, especially relevant on Windows
        // where process::exit can drop buffered records).
        logging::flush();
        std::process::exit(1);
    }

    info!("hex-gen finished cleanly");
    Ok(())
}

/// Validates that `name` is a safe Postgres identifier
/// (`[a-zA-Z_][a-zA-Z0-9_]*`).
///
/// v0.4.0 (closes G-H04). Defense in depth against SQL injection via
/// `--exclude-tables` and `--include-tables` (when the latter lands).
/// The introspector today binds table names as `&str` parameters
/// (safe), but this guard:
///
/// * surfaces config typos early with a friendly message instead of
///   a silent "table not found" later,
/// * prevents a future refactor that interpolates the table name into
///   a catalog query from turning a misformed entry into a SQL
///   injection vector.
///
/// # Errors
///
/// Returns [`GenError::Other`] with a descriptive message when `name`
/// contains characters outside the safe set.
fn validate_table_identifier(name: &str) -> GenResult<()> {
    if name.is_empty() {
        return Err(GenError::Other(anyhow::anyhow!(
            "table name in --exclude-tables is empty"
        )));
    }
    let mut chars = name.chars();
    let first = chars.next().expect("checked non-empty");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(GenError::Other(anyhow::anyhow!(
            "invalid table identifier `{name}`: must start with a letter or underscore"
        )));
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(GenError::Other(anyhow::anyhow!(
                "invalid table identifier `{name}`: only [a-zA-Z0-9_] allowed after the first char"
            )));
        }
    }
    Ok(())
}

/// Runs the full pipeline: config → IR → analysis → codegen → report.
///
/// Kept separate from `main` so it can return `GenResult<()>` and be unit
/// tested without spinning a second Tokio runtime.
async fn run(cli: &Cli) -> GenResult<()> {
    // ── 1. configuration ──────────────────────────────────────────────────
    // Fixture mode never touches PostgreSQL — inject a placeholder so the
    // generated `.env`/`Config` stay valid without a live connection string.
    let database_url = if cli.fixture {
        info!("fixture mode: no database required");
        Some("postgresql://user:password@localhost:5432/dbname".to_string())
    } else {
        cli.database_url.clone()
    };

    // v0.3.0 (closes G-C03): when `--profiles` is supplied, load the TOML
    // file and use its first profile as the introspection target. The
    // whole profile set is forwarded to codegen so the generated project
    // gets a pool registry that can hold every declared profile.
    let profile_set = if let Some(path) = cli.profiles.as_ref() {
        if cli.database_url.is_some() && !cli.fixture {
            warn!("--profiles overrides --database-url; the URL flag is ignored");
        }
        let src = std::fs::read_to_string(path).map_err(|source| GenError::Io {
            path: path.clone(),
            source,
        })?;
        let set = connection_profile::ProfileSet::parse_toml(&src).map_err(GenError::Other)?;
        info!(profiles = set.profiles.len(), path = %path.display(), "loaded connection profiles");
        Some(set)
    } else {
        None
    };

    // Resolve the actual introspection target URL: profiles (first entry)
    // wins, then DATABASE_URL, then the fixture placeholder.
    let introspect_url: Option<String> = if cli.fixture {
        Some("postgresql://user:password@localhost:5432/dbname".to_string())
    } else if let Some(set) = profile_set.as_ref() {
        set.profiles.first().map(|p| p.url.clone())
    } else {
        cli.database_url.clone()
    };

    let cfg =
        config::Config::from_parts(introspect_url, cli.schema.clone(), cli.project_name.clone())?;

    let mut excluded = config::default_excluded_tables();
    for extra in cli.exclude_tables.split(',') {
        let t = extra.trim();
        if !t.is_empty() {
            // v0.4.0 (closes G-H04): validate every user-supplied table
            // name against the safe Postgres-identifier charset
            // [a-zA-Z_][a-zA-Z0-9_]* so that a misformed --exclude-tables
            // entry can never become a SQL injection vector even if a
            // future refactor interpolates it into a catalog query.
            // Today the introspector binds table names as `&str`
            // parameters (safe), but this guard is defense in depth and
            // surfaces config typos as an early, friendly error rather
            // than a silent "table not found".
            validate_table_identifier(t)?;
            excluded.push(t.to_string());
        }
    }
    if !excluded.is_empty() {
        info!(excluded = ?excluded, "table exclusion list");
    }

    // v1.0.0 (closes G-M14): parse --include-tables. When non-empty,
    // every table NOT in the include list is added to the excluded
    // list. Both lists are validated against the same safe-identifier
    // charset.
    let mut included: Vec<String> = Vec::new();
    for extra in cli.include_tables.split(',') {
        let t = extra.trim();
        if !t.is_empty() {
            validate_table_identifier(t)?;
            included.push(t.to_string());
        }
    }
    if !included.is_empty() {
        info!(included = ?included, "table inclusion list (every other table will be excluded)");
    }

    // ── 2. obtain the IR (introspect OR fixture) ──────────────────────────
    let schema = if cli.fixture {
        info!("building demo IR from built-in fixture");
        fixture::demo_schema()
    } else if let Some(set) = profile_set.as_ref() {
        // Profile-driven introspection: assume the profile's
        // `introspect_role` (if any) before issuing catalog queries.
        let profile = &set.profiles[0];
        let masked = mask_password(&profile.url);
        info!(url = %masked, schema = %cfg.schema, profile = %profile.id,
              role = ?profile.introspect_role, "introspecting via connection profile");
        introspect::introspect_profile(profile, &cfg.schema, &excluded).await?
    } else {
        let masked = mask_password(cfg.database_url.as_str());
        info!(url = %masked, schema = %cfg.schema, "introspecting live PostgreSQL");
        introspect::introspect(&cfg, &excluded).await?
    };

    if schema.tables.is_empty() {
        warn!("no usable tables found — every table needs exactly one primary-key column");
        return Err(errors::GenError::NoUsableTables {
            schema: cfg.schema.clone(),
        });
    }

    // v1.0.0 (closes G-M14): apply the --include-tables filter. When
    // the include list is non-empty, drop every table whose name is
    // not in the list. This runs AFTER introspection so the IR
    // reflects exactly what the operator asked for.
    let mut schema = schema;
    if !included.is_empty() {
        let before = schema.tables.len();
        schema
            .tables
            .retain(|t| included.iter().any(|inc| inc == &t.name));
        let dropped = before - schema.tables.len();
        if dropped > 0 {
            info!(
                dropped = dropped,
                kept = schema.tables.len(),
                "applied --include-tables filter"
            );
        }
    }

    // v1.0.0 (closes G-M15): --dry-run stops here. The IR is built,
    // the conflict detection has run (below), and the summary is
    // printed — but no files are written. Useful for CI checks.
    if cli.dry_run {
        info!("--dry-run: skipping codegen; printing IR summary only");
        println!("dry-run: {} tables would be generated", schema.tables.len());
        for t in &schema.tables {
            println!(
                "  {} (pk={})",
                t.name,
                t.pk().map(|c| c.name.as_str()).unwrap_or("<none>")
            );
        }
        return Ok(());
    }

    // ── 3. human-readable summary of what was detected ────────────────────
    info!(tables = schema.tables.len(), "IR ready");
    for t in &schema.tables {
        let pk = t
            .pk()
            .map(|c| format!("{}:{}", c.name, c.kind.rust_type()))
            .unwrap_or_else(|| "<none>".into());
        info!(
            table = %t.name,
            pk = %pk,
            features = ?t.analysis.features(),
            "introspected table"
        );
    }

    // v0.5.0 (closes G-L07): detect struct-name conflicts before
    // codegen runs. Two tables that differ only in snake_case
    // spelling (e.g. `user_orders` vs `userOrders`) would both map
    // to the same `UpperCamel` Rust struct name and produce a
    // compile error in the generated project. Surface the conflict
    // here so the operator can rename or exclude one of the tables.
    let table_names: Vec<&str> = schema.tables.iter().map(|t| t.name.as_str()).collect();
    let conflicts = idents::detect_struct_name_conflicts(&table_names);
    for (struct_name, tables) in &conflicts {
        warn!(
            struct_name = %struct_name,
            conflicting_tables = ?tables,
            "struct-name conflict detected — two tables would map to the same Rust struct; \
             rename one of the tables or pass --exclude-tables to skip one"
        );
    }

    // ── 4. emit the project ───────────────────────────────────────────────
    info!(
        output = %cli.output.display(),
        crate_name = %cfg.project_name,
        "generating hexagonal project"
    );
    let stats = codegen::generate(&schema, &cli.output, &cfg, cli.force)?;

    // ── 5. final report ───────────────────────────────────────────────────
    info!(
        files_written = stats.files_written,
        bytes_written = stats.bytes_written,
        tables = schema.tables.len(),
        "generation complete"
    );

    println!(
        "✓ wrote {} files into {}",
        stats.files_written,
        cli.output.display()
    );
    println!();
    println!("next steps:");
    println!("  cd {}", cli.output.display());
    println!("  cargo check        # verify the generated project compiles");
    println!("  cargo run          # Swagger UI at http://localhost:8080/swagger-ui");
    println!();
    println!("  # apply migrations (when a database is available):");
    println!("  psql $DATABASE_URL -f migrations/*.sql");

    // Write the on-disk report next to the generated project (informational).
    let _ = report::write_report(&cli.output, &schema, &stats, &cfg);

    Ok(())
}

// ─── Error-inventory subcommand dispatcher ─────────────────────────────────
//
// Closes gap G-H03. Every branch logs with `category = "error_inventory"`
// so the audit trail of inventory edits is itself observable (CMMI MA
// SG 2 "collect data"). All failures surface as `GenError::Other`
// (which carries the `GEN-099` code from `errors::GenError::error_code`).

/// Dispatches the `hex-gen errors <sub>` subcommand.
///
/// Kept separate from `run` so the normal pipeline does not pay the
/// cost of reading the inventory on every invocation.
async fn run_errors_cmd(cmd: ErrorsCmd) -> GenResult<()> {
    use error_inventory::{Inventory, Status};
    use tracing::{info, warn};

    info!(category = "error_inventory", sub = ?cmd, "errors subcommand");

    match cmd {
        ErrorsCmd::List { status, inventory } => {
            let inv = Inventory::load(&inventory).map_err(|e| GenError::Other(e))?;
            let filter = status.as_deref().and_then(|s| match s {
                "open" => Some(Status::Open),
                "resolved" => Some(Status::Resolved),
                "deferred" => Some(Status::Deferred),
                "superseded" => Some(Status::Superseded),
                _ => {
                    warn!(input = %s, "unknown status filter; listing all");
                    None
                }
            });
            for e in inv.entries.values() {
                if let Some(want) = filter {
                    if e.status != want {
                        continue;
                    }
                }
                println!("{} [{}] {} — {}", e.id, e.severity, e.title, e.status);
            }
        }

        ErrorsCmd::Add {
            title,
            component,
            severity,
            error_type,
            message,
            root_cause,
            inventory,
        } => {
            let mut inv = Inventory::load(&inventory).map_err(|e| GenError::Other(e))?;
            let id = inv.next_id();
            let entry = error_inventory::ErrorEntry {
                id: id.clone(),
                title,
                status: Status::Open,
                component,
                severity,
                cmmi_pa: vec!["MA".into(), "CAR".into()],
                gap_ids: vec![],
                date: chrono::Utc::now(),
                location: std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "<unknown>".into()),
                error_type,
                error_message: message,
                root_cause,
                resolution: "(open)".into(),
                lessons: None,
                related: vec![],
                error_code: None,
            };
            inv.add(entry).map_err(|e| GenError::Other(e))?;
            inv.write_json(&inventory).map_err(|e| GenError::Other(e))?;
            // Regenerate the Markdown view so the two files stay in sync.
            let md_path = inventory.with_extension("md");
            let _ = inv.write_markdown(&md_path);
            info!(id = %id, "added error entry");
            println!("added {}", id);
        }

        ErrorsCmd::Resolve {
            id,
            resolution,
            lessons,
            inventory,
        } => {
            let mut inv = Inventory::load(&inventory).map_err(|e| GenError::Other(e))?;
            inv.resolve(&id, resolution, lessons)
                .map_err(|e| GenError::Other(e))?;
            inv.write_json(&inventory).map_err(|e| GenError::Other(e))?;
            let md_path = inventory.with_extension("md");
            let _ = inv.write_markdown(&md_path);
            info!(id = %id, "resolved error entry");
            println!("resolved {}", id);
        }

        ErrorsCmd::Stats { inventory } => {
            let inv = Inventory::load(&inventory).map_err(|e| GenError::Other(e))?;
            let mut by_status: std::collections::BTreeMap<String, u32> =
                std::collections::BTreeMap::new();
            let mut by_severity: std::collections::BTreeMap<String, u32> =
                std::collections::BTreeMap::new();
            let mut by_pa: std::collections::BTreeMap<String, u32> =
                std::collections::BTreeMap::new();
            for e in inv.entries.values() {
                *by_status.entry(e.status.to_string()).or_default() += 1;
                *by_severity.entry(e.severity.to_string()).or_default() += 1;
                for pa in &e.cmmi_pa {
                    *by_pa.entry(pa.clone()).or_default() += 1;
                }
            }
            println!("total: {}", inv.entries.len());
            println!("\nby status:");
            for (k, v) in &by_status {
                println!("  {k}: {v}");
            }
            println!("\nby severity:");
            for (k, v) in &by_severity {
                println!("  {k}: {v}");
            }
            println!("\nby CMMI PA:");
            for (k, v) in &by_pa {
                println!("  {k}: {v}");
            }
        }
    }

    // Best-effort flush before returning so the inventory audit log is
    // never lost (closes gap G-L12 for the subcommand path too).
    logging::flush();
    Ok(())
}
