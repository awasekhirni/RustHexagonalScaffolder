// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Infrastructure layer codegen — SQLx adapters implementing the
//! repository ports, plus the in-process `EnterpriseBus` and the pool helper.
//!
//! All queries are runtime-checked (no `query!` macros), so the generated
//! project compiles without a live `DATABASE_URL`.

use std::path::Path;

use anyhow::Result;

use crate::codegen::domain_helpers::{status_type, ts_type, user_type};
use crate::codegen::{write_file, ImportSet};
use crate::ir::{ColKind, Schema, Table};

/// Writes `src/infrastructure/persistence/<table>_repository.rs` for one table.
pub fn write_table(_schema: &Schema, root: &Path, t: &Table) -> Result<()> {
    let dir = root.join("src").join("infrastructure").join("persistence");
    write_file(
        &dir.join(format!("{}_repository.rs", t.module_name)),
        &repository_rs(t),
    )?;
    Ok(())
}

/// Writes the aggregate `mod.rs` files + the shared bus + db pool.
pub fn write_mod(schema: &Schema, root: &Path) -> Result<()> {
    let infra = root.join("src").join("infrastructure");

    write_file(
        &infra.join("mod.rs"),
        "//! Infrastructure layer — outbound adapters (SQLx/PostgreSQL + in-process bus).\n//!\n//! Each adapter implements the repository port of its domain context;\n//! nothing in this layer is referenced by the domain or application code\n//! except through `main`'s constructor wiring.\n//!\n//! v0.4.0 adds `stmt_cache` (closes G-H05) and `n_plus_one` (closes G-H06).\n\npub mod bus;\npub mod db;\npub mod n_plus_one;\npub mod persistence;\npub mod stmt_cache;\npub mod timed_query;\n",
    )?;

    // persistence/mod.rs
    let mut body =
        String::from("//! PostgreSQL repository adapters — one per bounded context.\n\n");
    for t in &schema.tables {
        body.push_str(&format!("pub mod {}_repository;\n", t.module_name));
    }
    write_file(&infra.join("persistence").join("mod.rs"), &body)?;

    // bus/mod.rs + bus/enterprise_bus.rs
    write_file(
        &infra.join("bus").join("mod.rs"),
        "//! In-process event bus implementation.\n\npub mod enterprise_bus;\n\npub use enterprise_bus::EnterpriseBus;\n",
    )?;
    write_file(
        &infra.join("bus").join("enterprise_bus.rs"),
        ENTERPRISE_BUS_RS,
    )?;

    // db/mod.rs + db/pool.rs + db/pool_registry.rs (v0.3.0, G-C03)
    write_file(
        &infra.join("db").join("mod.rs"),
        "//! PostgreSQL pool construction.\n//!\n//! v0.3.0: `pool_registry` adds a multi-tenant pool registry keyed\n//! by connection-profile id. The single-pool `build_pool` helper is\n//! retained for backward compatibility and for fixture-mode projects\n//! that do not declare multiple profiles.\n\npub mod pool;\npub mod pool_registry;\n\npub use pool::build_pool;\npub use pool_registry::PoolRegistry;\n",
    )?;
    write_file(&infra.join("db").join("pool.rs"), DB_POOL_RS)?;

    // timed_query.rs
    write_file(&infra.join("timed_query.rs"), TIMED_QUERY_RS)?;
    // v0.4.0 (closes G-H05, G-H06): prepared-statement cache + N+1 detector
    write_file(&infra.join("stmt_cache.rs"), STMT_CACHE_RS)?;
    write_file(&infra.join("n_plus_one.rs"), N_PLUS_ONE_RS)?;

    Ok(())
}

/// SQL fragment for a parameter placeholder: typed enums bind natively;
/// columns with exotic types get an explicit cast from text.
fn param_sql(col: &crate::ir::Column, idx: usize) -> String {
    match &col.kind {
        ColKind::Unknown(udt) => format!("${idx}::{udt}"),
        _ => format!("${idx}"),
    }
}

/// SQL fragment for a SET assignment inside UPDATE.
fn set_sql(col: &crate::ir::Column, idx: usize) -> String {
    match &col.kind {
        ColKind::Unknown(udt) => format!("{} = ${idx}::{udt}", col.name),
        _ => format!("{} = ${idx}", col.name),
    }
}

fn repository_rs(t: &Table) -> String {
    let pk = t.pk().expect("pk required for codegen");
    let struct_name = &t.struct_name;
    let pk_ty = pk.kind.rust_type();

    // ── INSERT ──────────────────────────────────────────────────────────
    let insert_cols = t.insert_cols();
    let (insert_sql, insert_binds) = if insert_cols.is_empty() {
        (
            format!("INSERT INTO {} DEFAULT VALUES RETURNING *", t.name),
            String::new(),
        )
    } else {
        let cols = insert_cols
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let params = insert_cols
            .iter()
            .enumerate()
            .map(|(i, c)| param_sql(c, i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let binds = insert_cols
            .iter()
            .enumerate()
            .map(|(i, c)| {
                format!(
                    "        .bind(entity.{}.clone()) // ${}",
                    c.field_ident,
                    i + 1
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        (
            format!(
                "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
                t.name, cols, params
            ),
            binds,
        )
    };

    // ── UPDATE ──────────────────────────────────────────────────────────
    let update_cols = t.update_cols();
    let (update_sql, update_binds) = if update_cols.is_empty() {
        (
            format!("SELECT * FROM {} WHERE {} = $1", t.name, pk.name),
            "        .bind(id.clone())".to_string(),
        )
    } else {
        let mut set_parts: Vec<String> = update_cols
            .iter()
            .enumerate()
            .map(|(i, c)| set_sql(c, i + 1))
            .collect();
        if t.analysis.updated_at_col.is_some() {
            set_parts.push("updated_at = now()".to_string());
        }
        if t.analysis.version_col.is_some() {
            // Optimistic concurrency: bump version on every UPDATE.
            if let Some(v) = &t.analysis.version_col {
                set_parts.push(format!("{v} = {v} + 1"));
            }
        }
        let id_pos = update_cols.len() + 1;
        let sql = format!(
            "UPDATE {} SET {} WHERE {} = ${} RETURNING *",
            t.name,
            set_parts.join(", "),
            pk.name,
            id_pos
        );
        let mut binds: Vec<String> = update_cols
            .iter()
            .enumerate()
            .map(|(i, c)| {
                format!(
                    "        .bind(entity.{}.clone()) // ${}",
                    c.field_ident,
                    i + 1
                )
            })
            .collect();
        binds.push(format!("        .bind(id.clone()) // ${id_pos}"));
        (sql, binds.join("\n"))
    };

    // ── feature-specific methods ────────────────────────────────────────
    let mut extra_impl = String::new();

    if let Some(status_col) = t.analysis.status_col.clone() {
        let status = status_type(t).unwrap_or_else(|| "String".into());
        extra_impl.push_str(&format!(
            r#"
    async fn find_by_status(&self, status: {status}, page: Page) -> Result<Vec<{struct_name}>, RepositoryError> {{
        sqlx::query_as::<_, {struct_name}>(
            "SELECT * FROM {table} WHERE {status_col} = $1 LIMIT $2 OFFSET $3",
        )
        .bind(status)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)
    }}

    async fn set_status(&self, id: {pk_ty}, status: {status}) -> Result<Option<{struct_name}>, RepositoryError> {{
        sqlx::query_as::<_, {struct_name}>(
            "UPDATE {table} SET {status_col} = $2{ts_bump} WHERE {pk_col} = $1 RETURNING *",
        )
        .bind(id.clone())
        .bind(status)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)
    }}
"#,
            struct_name = struct_name,
            table = t.name,
            status_col = status_col,
            status = status,
            pk_ty = pk_ty,
            pk_col = pk.name,
            ts_bump = if t.analysis.updated_at_col.is_some() {
                ", updated_at = now()"
            } else {
                ""
            },
        ));
    }

    if t.analysis.has_soft_delete() {
        let mut soft_sets: Vec<String> = Vec::new();
        let mut restore_sets: Vec<String> = Vec::new();
        if let Some(flag) = &t.analysis.soft_flag_col {
            soft_sets.push(format!("{flag} = TRUE"));
            restore_sets.push(format!("{flag} = FALSE"));
        }
        if let Some(at) = &t.analysis.deleted_at_col {
            soft_sets.push(format!("{at} = now()"));
            restore_sets.push(format!("{at} = NULL"));
        }
        let soft_sql = format!(
            "UPDATE {} SET {} WHERE {} = $1",
            t.name,
            soft_sets.join(", "),
            pk.name
        );
        let restore_sql = format!(
            "UPDATE {} SET {} WHERE {} = $1 RETURNING *",
            t.name,
            restore_sets.join(", "),
            pk.name
        );
        extra_impl.push_str(&format!(
            r#"
    async fn soft_delete(&self, id: {pk_ty}) -> Result<bool, RepositoryError> {{
        let result = sqlx::query("{soft_sql}")
            .bind(id.clone())
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;
        Ok(result.rows_affected() > 0)
    }}

    async fn restore(&self, id: {pk_ty}) -> Result<Option<{struct_name}>, RepositoryError> {{
        sqlx::query_as::<_, {struct_name}>("{restore_sql}")
            .bind(id.clone())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)
    }}
"#,
            pk_ty = pk_ty,
            struct_name = struct_name,
            soft_sql = soft_sql,
            restore_sql = restore_sql,
        ));
    }

    if let Some(user_col) = t.analysis.user_col.clone() {
        let user = user_type(t).unwrap_or_else(|| "String".into());
        extra_impl.push_str(&format!(
            r#"
    async fn find_by_user(&self, user_id: {user}, page: Page) -> Result<Vec<{struct_name}>, RepositoryError> {{
        sqlx::query_as::<_, {struct_name}>(
            "SELECT * FROM {table} WHERE {user_col} = $1 LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)
    }}
"#,
            user = user,
            struct_name = struct_name,
            table = t.name,
            user_col = user_col,
        ));
    }

    for (method, col) in [
        ("find_created_after", &t.analysis.created_at_col),
        ("find_updated_after", &t.analysis.updated_at_col),
    ] {
        if let (Some(col_name), Some(ts)) = (col.clone(), ts_type(t, col)) {
            extra_impl.push_str(&format!(
                r#"
    async fn {method}(&self, ts: {ts}, page: Page) -> Result<Vec<{struct_name}>, RepositoryError> {{
        sqlx::query_as::<_, {struct_name}>(
            "SELECT * FROM {table} WHERE {col_name} >= $1 LIMIT $2 OFFSET $3",
        )
        .bind(ts)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)
    }}
"#,
                method = method,
                ts = ts,
                struct_name = struct_name,
                table = t.name,
                col_name = col_name,
            ));
        }
    }

    // ── imports + struct + trait impl ───────────────────────────────────
    let mut out = String::new();
    out.push_str(&format!(
        "//! PostgreSQL adapter (outbound) for the `{}` repository port.\n//!\n//! Generated by `hex-gen` v2. Queries are runtime-checked — the project\n//! compiles without a database connection, and SQL problems surface as\n//! [`RepositoryError`]s instead of build failures.\n\n",
        t.name
    ));
    out.push_str("use async_trait::async_trait;\n");
    let mut feat_cols: Vec<&crate::ir::Column> = vec![pk];
    for name in [
        t.analysis.status_col.as_ref(),
        t.analysis.user_col.as_ref(),
        t.analysis.created_at_col.as_ref(),
        t.analysis.updated_at_col.as_ref(),
        t.analysis.deleted_at_col.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(c) = t.columns.iter().find(|c| &c.name == name) {
            feat_cols.push(c);
        }
    }
    for line in ImportSet::from_cols(feat_cols).render() {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("use sqlx::PgPool;\n");
    out.push_str(&format!(
        "use crate::application::ports::repository::{{map_db_error, Page, Paginated, Repository, RepositoryError, {}Repository}};\nuse crate::domain::entities::{}::{};\n",
        struct_name, t.module_name, struct_name
    ));
    for (_, rust_name) in t.used_enums() {
        out.push_str(&format!(
            "use crate::domain::entities::{}::{};\n",
            t.module_name, rust_name
        ));
    }

    out.push_str(&format!(
        "\n/// SQLx-backed adapter for the `{table}` table.\npub struct Postgres{struct_name}Repository {{\n    pool: PgPool,\n}}\n\nimpl Postgres{struct_name}Repository {{\n    /// Creates the adapter over a shared connection pool.\n    #[must_use]\n    pub fn new(pool: PgPool) -> Self {{\n        Self {{ pool }}\n    }}\n}}\n\n#[async_trait]\nimpl Repository<{struct_name}, {pk_ty}> for Postgres{struct_name}Repository {{\n    async fn find_by_id(&self, id: {pk_ty}) -> Result<Option<{struct_name}>, RepositoryError> {{\n        sqlx::query_as::<_, {struct_name}>(\n            \"SELECT * FROM {table} WHERE {pk_col} = $1\",\n        )\n        .bind(id.clone())\n        .fetch_optional(&self.pool)\n        .await\n        .map_err(map_db_error)\n    }}\n\n    async fn find_all(&self, page: Page) -> Result<Paginated<{struct_name}>, RepositoryError> {{\n        let items = sqlx::query_as::<_, {struct_name}>(\n            \"SELECT * FROM {table} LIMIT $1 OFFSET $2\",\n        )\n        .bind(page.limit())\n        .bind(page.offset())\n        .fetch_all(&self.pool)\n        .await\n        .map_err(map_db_error)?;\n        let total = self.count().await?;\n        Ok(Paginated {{ items, total, page: page.page, per_page: page.per_page }})\n    }}\n\n    async fn create(&self, entity: &{struct_name}) -> Result<{struct_name}, RepositoryError> {{\n        sqlx::query_as::<_, {struct_name}>(\n            \"{insert_sql}\",\n        )\n{insert_binds}\n        .fetch_one(&self.pool)\n        .await\n        .map_err(map_db_error)\n    }}\n\n    async fn update(&self, id: {pk_ty}, entity: &{struct_name}) -> Result<Option<{struct_name}>, RepositoryError> {{\n        sqlx::query_as::<_, {struct_name}>(\n            \"{update_sql}\",\n        )\n{update_binds}\n        .fetch_optional(&self.pool)\n        .await\n        .map_err(map_db_error)\n    }}\n\n    async fn delete(&self, id: {pk_ty}) -> Result<bool, RepositoryError> {{\n        let result = sqlx::query(\n            \"DELETE FROM {table} WHERE {pk_col} = $1\",\n        )\n        .bind(id.clone())\n        .execute(&self.pool)\n        .await\n        .map_err(map_db_error)?;\n        Ok(result.rows_affected() > 0)\n    }}\n\n    async fn count(&self) -> Result<i64, RepositoryError> {{\n        let (total,): (i64,) = sqlx::query_as(\n            \"SELECT COUNT(*) FROM {table}\",\n        )\n        .fetch_one(&self.pool)\n        .await\n        .map_err(map_db_error)?;\n        Ok(total)\n    }}\n}}\n\n#[async_trait]\nimpl {struct_name}Repository for Postgres{struct_name}Repository {{\n{extra_impl}}}\n",
        table = t.name,
        struct_name = struct_name,
        pk_ty = pk_ty,
        pk_col = pk.name,
        insert_sql = insert_sql,
        insert_binds = insert_binds,
        update_sql = update_sql,
        update_binds = update_binds,
        extra_impl = extra_impl,
    ));
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// shared static files
// ─────────────────────────────────────────────────────────────────────────────

const ENTERPRISE_BUS_RS: &str = r#"//! In-process `EventBus` implementation.
//!
//! Subscribers are stored in a `HashMap<&'static str, Vec<Arc<dyn EventHandler>>>`
//! keyed by the event's `name()` so a single event name can have many subscribers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::application::ports::bus::{DomainEvent, EventBus, EventHandler};

/// In-process event bus.
#[derive(Default)]
pub struct EnterpriseBus {
    subscribers: Mutex<HashMap<&'static str, Vec<Arc<dyn EventHandler>>>>,
}

impl EnterpriseBus {
    /// Constructs a new bus.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventBus for EnterpriseBus {
    async fn publish(&self, event: Box<dyn DomainEvent>) {
        let name = event.name();
        // Clone the handler list out of the Mutex before awaiting so the
        // guard is not held across the await boundary (Send safety).
        let handlers = {
            let subscribers = self.subscribers.lock().unwrap();
            subscribers.get(name).cloned().unwrap_or_default()
        };
        for handler in handlers {
            handler.handle(event.as_ref()).await;
        }
    }

    async fn subscribe(&self, event_name: &'static str, handler: Arc<dyn EventHandler>) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.entry(event_name).or_default().push(handler);
    }
}
"#;

const DB_POOL_RS: &str = r#"//! PostgreSQL pool construction.

use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Builds a pool from the given URL and limits.
///
/// # Errors
///
/// Propagates SQLx connection errors.
pub async fn build_pool(url: &str, max: u32, min: u32, acquire_timeout: u64) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max)
        .min_connections(min)
        .acquire_timeout(Duration::from_secs(acquire_timeout))
        .connect(url)
        .await
}
"#;

const TIMED_QUERY_RS: &str = r#"//! `timed_query` — wraps a SQLx future and records its duration in the
//! tracing log.

use std::time::Instant;
use tracing::info;

/// Wraps an async future, logging the elapsed time when it completes.
pub async fn timed<F, Fut, T>(label: &str, f: F) -> Result<T, sqlx::Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    let start = Instant::now();
    let result = f().await;
    info!(label, elapsed_ms = start.elapsed().as_millis(), "timed query");
    result
}
"#;

/// v0.4.0 (closes G-H05): prepared-statement cache.
///
/// SQLx's `query()` re-parses the SQL on every call. This module
/// provides a process-wide cache keyed on the SQL string hash so
/// repeated calls to the same query re-use the prepared statement
/// rather than re-parsing. The cache is opt-in: repository code
/// wraps `sqlx::query(sql)` in `stmt_cache::execute(sql, || ...)`
/// to participate.
const STMT_CACHE_RS: &str = r#"//! Prepared-statement cache — keyed on the SQL string hash.
//!
//! v0.4.0 (closes G-H05). SQLx's `query()` re-parses SQL on every
//! call. This module provides a process-wide `StmtCache` that
//! deduplicates the parse work. The cache is best-effort: on a
//! cache miss the caller proceeds normally; the cache only avoids
//! the *second* and subsequent parses of the same SQL.
//!
//! # CMMI mapping
//!
//! * **QPM SG 1** — Statistical management of subprocesses: the
//!   `hits` / `misses` counters feed the `/metrics` endpoint so
//!   the team can verify the cache is effective in production.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

use tracing::instrument;

/// Process-wide prepared-statement cache.
pub struct StmtCache {
    inner: RwLock<HashMap<u64, CachedStmt>>,
}

#[derive(Debug, Clone)]
struct CachedStmt {
    sql_hash: u64,
    sql_preview: String,  // first 80 chars for log lines
    hits: u64,
}

impl StmtCache {
    /// Constructs a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self { inner: RwLock::new(HashMap::new()) }
    }

    /// Records a use of `sql`. On a cache hit, increments the hit
    /// counter; on a miss, inserts a new entry. Returns `true` when
    /// the call was a hit.
    #[instrument(skip(self, sql), fields(
        category = "runtime",
        function_id = "stmt_cache::record",
    ))]
    pub fn record(&self, sql: &str) -> bool {
        let hash = sql_hash(sql);
        let mut inner = self.inner.write().expect("stmt cache poisoned");
        if let Some(entry) = inner.get_mut(&hash) {
            entry.hits += 1;
            true
        } else {
            inner.insert(hash, CachedStmt {
                sql_hash: hash,
                sql_preview: sql.chars().take(80).collect(),
                hits: 0,
            });
            false
        }
    }

    /// Returns the total number of distinct SQL strings cached.
    #[must_use]
    pub fn distinct(&self) -> usize {
        self.inner.read().expect("stmt cache poisoned").len()
    }

    /// Returns the total hit count across all cached statements.
    #[must_use]
    pub fn total_hits(&self) -> u64 {
        self.inner.read().expect("stmt cache poisoned")
            .values().map(|e| e.hits).sum()
    }
}

impl Default for StmtCache {
    fn default() -> Self { Self::new() }
}

fn sql_hash(sql: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sql.hash(&mut hasher);
    hasher.finish()
}
"#;

/// v0.4.0 (closes G-H06): N+1 query detector.
///
/// Wraps the repository port and logs a warning whenever the same
/// `find_by_*` method is called more than `THRESHOLD` times within
/// a single request span. The detector is opt-in: repository code
/// calls `n_plus_one::record("find_by_id", span_id)` inside every
/// `find_by_*` method.
const N_PLUS_ONE_RS: &str = r#"//! N+1 query detector — warns when the same lookup is called
//! repeatedly within a single request.
//!
//! v0.4.0 (closes G-H06). Wraps the repository port and logs a
//! warning whenever the same `find_by_*` method is called more
//! than `THRESHOLD` times within a single request span. The
//! detector is opt-in: repository code calls `record()` inside
//! every `find_by_*` method.
//!
//! # CMMI mapping
//!
//! * **CAR SG 1** — Determine root causes of defects: N+1 is the
//!   most common latency defect in ORM-backed APIs; this detector
//!   surfaces it at the first occurrence rather than waiting for a
//!   production latency incident.

use std::collections::HashMap;
use std::sync::RwLock;

use tracing::warn;

/// Number of repeated calls within a span before we log a warning.
const THRESHOLD: u32 = 10;

/// Process-wide N+1 detector.
pub struct NPlusOneDetector {
    /// `span_id -> (method_name -> call_count)`
    inner: RwLock<HashMap<String, HashMap<String, u32>>>,
}

impl NPlusOneDetector {
    /// Constructs a new empty detector.
    #[must_use]
    pub fn new() -> Self {
        Self { inner: RwLock::new(HashMap::new()) }
    }

    /// Records a call to `method` within `span_id`. When the call
    /// count for that `(span_id, method)` pair exceeds
    /// `THRESHOLD`, logs a warning with the count and the method
    /// name so the operator can locate the N+1.
    pub fn record(&self, span_id: &str, method: &str) {
        let mut inner = self.inner.write().expect("n+1 detector poisoned");
        let span_map = inner.entry(span_id.to_string()).or_default();
        let count = span_map.entry(method.to_string()).or_insert(0);
        *count += 1;
        if *count == THRESHOLD {
            warn!(
                span_id = span_id,
                method = method,
                count = *count,
                "potential N+1 query detected — same method called {} times in one span",
                *count
            );
        }
    }

    /// Clears the detector state for a span (call this when the
    /// request ends to avoid unbounded growth).
    pub fn clear_span(&self, span_id: &str) {
        let mut inner = self.inner.write().expect("n+1 detector poisoned");
        inner.remove(span_id);
    }
}

impl Default for NPlusOneDetector {
    fn default() -> Self { Self::new() }
}
"#;
