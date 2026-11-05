//! Live PostgreSQL introspection.
//!
//! Reads `information_schema` + `pg_catalog` (the SQLx analogue of the
//! Python script's SQLAlchemy `automap_base()` reflection) and produces
//! the generator IR:
//!
//! 1. base tables of the target schema,
//! 2. native enum types with their ordered labels (`pg_enum`),
//! 3. columns with nullability, defaults, identity and generated flags,
//! 4. primary keys (single-column PKs only — composite PKs are skipped
//!    with a warning because generic REST CRUD needs one stable ID),
//! 5. foreign keys (documentation / future relation endpoints).

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use heck::ToUpperCamelCase;
use sqlx::{PgPool, Row};

use crate::analysis;
use crate::config::Config;
use crate::idents;
use crate::ir::{ColKind, Column, ForeignKey, Schema, Table};

/// Introspects `cfg.schema` and returns the IR.
pub async fn introspect(cfg: &Config, excluded: &[String]) -> Result<Schema> {
    let pool = PgPool::connect(&cfg.database_url)
        .await
        .context("connecting to PostgreSQL (check DATABASE_URL)")?;

    let schema = Schema {
        schema: cfg.schema.clone(),
        tables: Vec::new(),
        enums: load_enums(&pool, &cfg.schema).await?,
    };

    let table_rows = sqlx::query(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = $1 AND table_type = 'BASE TABLE'
        ORDER BY table_name
        "#,
    )
    .bind(&cfg.schema)
    .fetch_all(&pool)
    .await
    .context("listing tables from information_schema")?;

    let mut tables = Vec::new();
    for row in table_rows {
        let name: String = row.get("table_name");

        if excluded.iter().any(|x| x == &name) {
            eprintln!("  ↷ skipping excluded table `{name}`");
            continue;
        }
        let Some(module_name) = idents::module_ident(&name) else {
            eprintln!("  ↷ skipping table `{name}` (name is not a valid Rust module)");
            continue;
        };

        match load_table(&pool, &cfg.schema, &name, module_name, &schema.enums).await {
            Ok(table) => tables.push(table),
            Err(reason) => eprintln!("  ↷ skipping table `{name}`: {reason}"),
        }
    }

    pool.close().await;
    Ok(Schema { tables, ..schema })
}

/// Loads ordered enum labels for every native enum in the schema.
async fn load_enums(pool: &PgPool, schema: &str) -> Result<BTreeMap<String, Vec<String>>> {
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
    .context("reading pg_enum")?;

    let mut enums: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        let type_name: String = row.get("type_name");
        let label: String = row.get("label");
        enums.entry(type_name).or_default().push(label);
    }
    Ok(enums)
}

/// Loads one table: columns, PK, FKs, then applies feature analysis.
async fn load_table(
    pool: &PgPool,
    schema: &str,
    name: &str,
    module_name: String,
    enums: &BTreeMap<String, Vec<String>>,
) -> Result<Table, String> {
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
    .map_err(|e| e.to_string())?;

    let pks: Vec<String> = pk_rows.iter().map(|r| r.get::<String, _>("column_name")).collect();
    if pks.len() != 1 {
        return Err(format!(
            "expected exactly one primary-key column, found {} (composite PKs are not supported)",
            pks.len()
        ));
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
    .map_err(|e| e.to_string())?;

    if col_rows.is_empty() {
        return Err("table has no columns".into());
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

        columns.push(Column {
            name: col_name.clone(),
            field_ident,
            kind,
            nullable: nullable == "YES",
            is_primary_key: pks[0] == col_name,
            has_default: default.is_some() || identity == "YES" || generated == "ALWAYS",
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
    .map_err(|e| e.to_string())?;

    let foreign_keys = fk_rows
        .iter()
        .map(|r| ForeignKey {
            column: r.get("column_name"),
            ref_table: r.get("ref_table"),
            ref_column: r.get("ref_column"),
        })
        .collect();

    let analysis = analysis::analyze(&columns);
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
            ColKind::PgEnum { .. } => ColKind::Unknown(format!("array-of-enum {udt}")),
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
        "text" | "varchar" | "bpchar" | "char" | "name" | "citext" | "inet" | "xml" => ColKind::Str,
        "bytea" => ColKind::Bytes,
        "date" => ColKind::NaiveDate,
        "time" | "timetz" => ColKind::NaiveTime,
        "timestamp" => ColKind::NaiveDateTime,
        "timestamptz" => ColKind::DateTimeUtc,
        "json" | "jsonb" => ColKind::Json,
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
            } else {
                // Exotic labels / domains / composites → String fallback.
                ColKind::Unknown(other.to_string())
            }
        }
    }
}
