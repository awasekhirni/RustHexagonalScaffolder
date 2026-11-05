// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Live PostgreSQL introspection (db-first) v2.
//!
//! Reads `information_schema` + `pg_catalog` and produces the generator IR:
//!
//! 1. base tables of the target schema,
//! 2. native enum types with their ordered labels (`pg_enum`),
//! 3. columns with nullability, defaults, identity and generated flags,
//! 4. primary keys (single-column PKs only — composite PKs are skipped
//!    with a warning because generic REST CRUD needs one stable ID),
//! 5. foreign keys (documentation / relation endpoints),
//! 6. secondary indexes (preserved in the IR for DDL regeneration).
//!
//! # v2 enhancements
//!
//! * Every SQL step is wrapped in a `tracing` span and surfaced via
//!   the typed [`GenError`] taxonomy (no stringly-typed errors).
//! * Indexes are loaded (the v1 introspector skipped them).
//! * The `is_identity`/`is_generated` flags are checked correctly:
//!   `BY DEFAULT` identity columns *do* accept user-supplied values
//!   (unlike `ALWAYS`), so they are treated as non-default for INSERT.
//! * Tables are processed in name order for deterministic output.

use std::collections::BTreeMap;

use heck::ToUpperCamelCase;
use sqlx::{PgPool, Row};
use tracing::{info, instrument, warn};

use crate::analysis;
use crate::config::Config;
use crate::errors::{mask_password, GenError, GenResult};
use crate::idents;
use crate::ir::{ColKind, Column, ForeignKey, Index, Schema, Table};

/// Introspects `cfg.schema` and returns the IR.
///
/// Connection is established once and closed at the end; the function is
/// cancellation-safe (the pool is dropped on `?`).
///
/// # Errors
///
/// * [`GenError::DbConnect`] — cannot reach PostgreSQL.
/// * [`GenError::CatalogQuery`] — a catalog query failed.
pub async fn introspect(cfg: &Config, excluded: &[String]) -> GenResult<Schema> {
    info!(url = mask_password(&cfg.database_url), schema = %cfg.schema, "connecting to PostgreSQL");
    let pool = sqlx::PgPool::connect(&cfg.database_url)
        .await
        .map_err(|source| GenError::DbConnect { source })?;

    let result = introspect_with_pool(&pool, cfg, excluded).await;

    // Always close the pool — never leak connections on early return.
    pool.close().await;
    result
}

/// v0.3.0 (closes gap G-C03): profile-driven introspection.
///
/// Connects using the profile's `url`, optionally assumes the profile's
/// `introspect_role` (via `SET ROLE`) before issuing any catalog query,
/// then `RESET ROLE`s before closing. This lets the generator connect as
/// a privileged user but introspect through the same role the runtime
/// will use, so the IR matches what the application will actually see.
///
/// # Errors
///
/// * [`GenError::DbConnect`] — cannot reach PostgreSQL.
/// * [`GenError::CatalogQuery`] — a catalog query failed (including the
///   `SET ROLE` / `RESET ROLE` statements).
#[instrument(skip(profile), fields(profile = %profile.id), err)]
pub async fn introspect_profile(
    profile: &crate::connection_profile::ConnectionProfile,
    schema: &str,
    excluded: &[String],
) -> GenResult<Schema> {
    info!(url = mask_password(&profile.url), schema = %schema,
          profile = %profile.id, "connecting via connection profile");
    let pool = sqlx::PgPool::connect(&profile.url)
        .await
        .map_err(|source| GenError::DbConnect { source })?;
    let mut conn = pool
        .acquire()
        .await
        .map_err(|source| GenError::DbConnect { source })?;

    // Assume the introspection role if declared. Identifier quoting is
    // safe because the role name comes from the profile config, not user
    // input — but we still validate it to fail fast on typos.
    if let Some(role) = &profile.introspect_role {
        validate_pg_identifier(role)?;
        let sql = format!("SET ROLE \\\"{}\\\"", role);
        sqlx::query(&sql)
            .execute(&mut *conn)
            .await
            .map_err(|source| GenError::CatalogQuery {
                query: "SET ROLE",
                source,
            })?;
        info!(role = %role, "assumed introspection role");
    }

    // Build a throwaway Config so we can reuse introspect_with_pool. The
    // database_url field is unused there (the pool is already built), so
    // we pass the profile url for log-line purposes only.
    let cfg = Config {
        database_url: profile.url.clone(),
        schema: schema.to_string(),
        project_name: String::new(), // unused by introspect_with_pool
    };
    let result = introspect_with_pool(&pool, &cfg, excluded).await;

    // Best-effort RESET ROLE — never fail the introspection if the reset
    // errors (the connection is about to be dropped anyway).
    if profile.introspect_role.is_some() {
        let _ = sqlx::query("RESET ROLE").execute(&mut *conn).await;
    }
    pool.close().await;
    result
}

/// Rejects Postgres identifiers that contain characters outside the
/// safe set `[a-zA-Z_][a-zA-Z0-9_]*`. Used to validate `introspect_role`
/// and `runtime_role` from connection profiles before they are
/// interpolated into `SET ROLE` SQL.
///
/// This is defense in depth — the role name comes from a TOML config
/// file, not user input — but it catches config typos early and prevents
/// a misconfigured role name from becoming a SQL injection vector if
/// the profile is later sourced from an untrusted location.
fn validate_pg_identifier(name: &str) -> GenResult<()> {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => {
            return Err(GenError::Other(anyhow::anyhow!(
                "invalid PG identifier `{name}`: must start with a letter or underscore"
            )))
        }
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(GenError::Other(anyhow::anyhow!(
                "invalid PG identifier `{name}`: only [a-zA-Z0-9_] allowed after the first char"
            )));
        }
    }
    Ok(())
}

/// Same as [`introspect`] but accepts an externally-managed pool.
///
/// Useful for integration tests that spin up a testcontainer pool.
pub async fn introspect_with_pool(
    pool: &PgPool,
    cfg: &Config,
    excluded: &[String],
) -> GenResult<Schema> {
    let enums = load_enums(pool, &cfg.schema).await?;
    let indexes = load_indexes(pool, &cfg.schema).await?;

    let table_rows = sqlx::query(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = $1 AND table_type = 'BASE TABLE'
        ORDER BY table_name
        "#,
    )
    .bind(&cfg.schema)
    .fetch_all(pool)
    .await
    .map_err(|source| GenError::CatalogQuery {
        query: "information_schema.tables",
        source,
    })?;

    let mut tables = Vec::new();
    for row in table_rows {
        let name: String = row.get("table_name");

        if excluded.iter().any(|x| x == &name) {
            info!(table = %name, "skipping excluded table");
            continue;
        }
        let Some(module_name) = idents::module_ident(&name) else {
            warn!(table = %name, "skipping table (name not a valid Rust module)");
            continue;
        };

        match load_table(pool, &cfg.schema, &name, module_name, &enums).await {
            Ok(table) => tables.push(table),
            Err(GenError::NoPrimaryKey { table, found }) => {
                warn!(table = %table, found, "skipping table (no single-column PK)");
            }
            Err(other) => return Err(other),
        }
    }

    Ok(Schema {
        schema: cfg.schema.clone(),
        tables,
        enums,
        indexes,
    })
}

/// Loads ordered enum labels for every native enum in the schema.
#[instrument(skip(pool), err)]
async fn load_enums(pool: &PgPool, schema: &str) -> GenResult<BTreeMap<String, Vec<String>>> {
    let rows = sqlx::query(
        r#"
        SELECT t.typname AS type_name, e.enumlabel AS label
        FROM pg_type t
        JOIN pg_enum e ON t.oid = e.enumtypid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE n.nspname = $1
        ORDER BY t.typname, e.enumsortorder
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await
    .map_err(|source| GenError::CatalogQuery {
        query: "pg_enum",
        source,
    })?;

    let mut enums: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        let type_name: String = row.get("type_name");
        let label: String = row.get("label");
        enums.entry(type_name).or_default().push(label);
    }
    info!(enums = enums.len(), "loaded native PG enums");
    Ok(enums)
}

/// Loads secondary indexes for every table in the schema.
#[instrument(skip(pool), err)]
async fn load_indexes(pool: &PgPool, schema: &str) -> GenResult<Vec<Index>> {
    let rows = sqlx::query(
        r#"
        SELECT
            i.relname AS index_name,
            t.relname AS table_name,
            am.amname AS index_type,
            idx.indisunique AS is_unique,
            array_agg(a.attname ORDER BY array_position(idx.indkey, a.attnum)) AS columns
        FROM pg_index idx
        JOIN pg_class t        ON t.oid = idx.indrelid
        JOIN pg_class i        ON i.oid = idx.indexrelid
        JOIN pg_am am          ON am.oid = i.relam
        JOIN pg_namespace n    ON n.oid = t.relnamespace
        JOIN pg_attribute a    ON a.attrelid = t.oid AND a.attnum = ANY(idx.indkey)
        WHERE n.nspname = $1
          AND NOT idx.indisprimary
        GROUP BY i.relname, t.relname, am.amname, idx.indisunique
        ORDER BY t.relname, i.relname
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await
    .map_err(|source| GenError::CatalogQuery {
        query: "pg_index",
        source,
    })?;

    let mut out = Vec::new();
    for row in rows {
        let name: String = row.get("index_name");
        let table: String = row.get("table_name");
        let unique: bool = row.get("is_unique");
        let columns: Vec<String> = row.get("columns");
        out.push(Index {
            name,
            table,
            columns,
            unique,
        });
    }
    info!(indexes = out.len(), "loaded secondary indexes");
    Ok(out)
}

/// Loads one table: columns, PK, FKs, then applies feature analysis.
#[instrument(skip(pool, enums), fields(table = %name), err)]
async fn load_table(
    pool: &PgPool,
    schema: &str,
    name: &str,
    module_name: String,
    enums: &BTreeMap<String, Vec<String>>,
) -> GenResult<Table> {
    // ── primary key ─────────────────────────────────────────────────────
    let pk_rows = sqlx::query(
        r#"
        SELECT kcu.column_name AS column_name
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
          ON tc.constraint_name = kcu.constraint_name
         AND tc.constraint_schema = kcu.constraint_schema
        WHERE tc.constraint_schema = $1
          AND tc.table_name = $2
          AND tc.constraint_type = 'PRIMARY KEY'
        "#,
    )
    .bind(schema)
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|source| GenError::CatalogQuery {
        query: "PRIMARY KEY",
        source,
    })?;

    let pks: Vec<String> = pk_rows
        .iter()
        .map(|r| r.get::<String, _>("column_name"))
        .collect();
    if pks.len() != 1 {
        return Err(GenError::NoPrimaryKey {
            table: name.to_string(),
            found: pks.len(),
        });
    }

    // ── columns ─────────────────────────────────────────────────────────
    let col_rows = sqlx::query(
        r#"
        SELECT column_name, udt_name, is_nullable, column_default,
               is_identity, is_generated
        FROM information_schema.columns
        WHERE table_schema = $1 AND table_name = $2
        ORDER BY ordinal_position
        "#,
    )
    .bind(schema)
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|source| GenError::CatalogQuery {
        query: "information_schema.columns",
        source,
    })?;

    if col_rows.is_empty() {
        return Err(GenError::NoPrimaryKey {
            table: name.to_string(),
            found: 0,
        });
    }

    let mut columns = Vec::new();
    for row in col_rows {
        let col_name: String = row.get("column_name");
        let udt: String = row.get("udt_name");
        let nullable: String = row.get("is_nullable");
        let default: Option<String> = row.get("column_default");
        let identity: String = row.get("is_identity");
        let generated: String = row.get("is_generated");

        let kind = resolve_kind(&udt, enums);
        let (field_ident, _changed) = idents::field_ident(&col_name);
        let is_pg_enum = matches!(&kind, ColKind::PgEnum { .. });

        // `BY DEFAULT` identity columns accept user-supplied values on
        // INSERT (they only auto-fill when the column is absent); we treat
        // them as non-default so the generated `Create…Input` includes them.
        let by_default_identity = identity == "BY DEFAULT";
        let has_default = default.is_some()
            || (identity == "YES" && !by_default_identity)
            || generated == "ALWAYS";

        columns.push(Column {
            name: col_name.clone(),
            field_ident,
            kind,
            nullable: nullable == "YES",
            is_primary_key: pks[0] == col_name,
            has_default,
            is_pg_enum,
        });
    }

    // ── foreign keys ────────────────────────────────────────────────────
    let fk_rows = sqlx::query(
        r#"
        SELECT kcu.column_name AS column_name,
               ccu.table_name  AS ref_table,
               ccu.column_name AS ref_column
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
          ON tc.constraint_name = kcu.constraint_name
         AND tc.constraint_schema = kcu.constraint_schema
        JOIN information_schema.constraint_column_usage ccu
          ON ccu.constraint_name = tc.constraint_name
         AND ccu.constraint_schema = tc.constraint_schema
        WHERE tc.constraint_schema = $1
          AND tc.table_name = $2
          AND tc.constraint_type = 'FOREIGN KEY'
        "#,
    )
    .bind(schema)
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|source| GenError::CatalogQuery {
        query: "FOREIGN KEY",
        source,
    })?;

    let foreign_keys = fk_rows
        .iter()
        .map(|r| ForeignKey {
            column: r.get("column_name"),
            ref_table: r.get("ref_table"),
            ref_column: r.get("ref_column"),
        })
        .collect();

    let analysis = analysis::analyze_full(name, &columns);
    Ok(Table {
        name: name.to_string(),
        struct_name: name.to_upper_camel_case(),
        module_name,
        columns,
        foreign_keys,
        analysis,
    })
}

/// Maps `information_schema.columns.udt_name` onto [`ColKind`], using the
/// `pg_enum` registry so only genuine native enums become typed enums.
fn resolve_kind(udt: &str, enums: &BTreeMap<String, Vec<String>>) -> ColKind {
    // Arrays arrive as `_text`, `_int4`, `_uuid`, …
    if let Some(inner) = udt.strip_prefix('_') {
        return match resolve_kind(inner, enums) {
            // v1.0.0 (closes G-M03): array-of-enum now falls back to
            // Vec<String> instead of Unknown("array-of-enum"). The
            // labels are still accessible via the IR's `enums` map,
            // so the generated code can document them even though the
            // column type is String.
            ColKind::PgEnum { pg_name, .. } => {
                tracing::warn!(
                    udt = udt,
                    enum_name = %pg_name,
                    "array-of-enum mapped to Vec<String> — sqlx 0.8 does not support \
                     native arrays of PG enums; labels are documented but the column \
                     is typed as String"
                );
                ColKind::VecOf(Box::new(ColKind::Str))
            }
            other => ColKind::VecOf(Box::new(other)),
        };
    }

    match udt {
        "bool" | "boolean" => ColKind::Bool,
        "int2" => ColKind::I16,
        "int4" => ColKind::I32,
        "int8" => ColKind::I64,
        "float4" => ColKind::F32,
        "float8" => ColKind::F64,
        "numeric" => ColKind::Decimal,
        "uuid" => ColKind::Uuid,
        "text" | "varchar" | "bpchar" | "char" | "name" | "citext" | "xml" => ColKind::Str,
        "inet" => ColKind::Inet,
        "bytea" => ColKind::Bytes,
        "date" => ColKind::NaiveDate,
        "time" | "timetz" => ColKind::NaiveTime,
        "timestamp" => ColKind::NaiveDateTime,
        "timestamptz" => ColKind::DateTimeUtc,
        "json" | "jsonb" => ColKind::Json,
        // v1.0.0 (closes G-M01): extended type support.
        "interval" => ColKind::Interval,
        "bit" | "varbit" => ColKind::BitString,
        "ltree" => ColKind::Ltree,
        "tsvector" | "tsquery" => ColKind::TsVector,
        "hstore" => ColKind::Hstore,
        "macaddr" => ColKind::Macaddr,
        "macaddr8" => ColKind::Macaddr8,
        "cidr" => ColKind::Cidr,
        other => {
            if enums
                .get(other)
                .is_some_and(|labels| crate::codegen::labels_snake_case(labels))
            {
                // Native PostgreSQL enum with snake_case labels — safe for
                // `rename_all = "snake_case"` on both serde and sqlx.
                ColKind::PgEnum {
                    pg_name: other.to_string(),
                    rust_name: Schema::enum_rust_name(other),
                }
            } else if enums.get(other).is_some() {
                // v1.0.0 (closes G-M04): the enum exists but its labels
                // are not snake_case. Map to String and log a warning
                // so the operator knows the enum is not typed in Rust.
                tracing::warn!(
                    udt = udt,
                    "PG enum has non-snake_case labels — mapped to String; \
                     relabel with snake_case to get a typed Rust enum"
                );
                ColKind::Str
            } else {
                // Exotic labels / domains / composites → String fallback.
                tracing::debug!(udt = udt, "unsupported PG type — mapped to String");
                ColKind::Unknown(other.to_string())
            }
        }
    }
}
