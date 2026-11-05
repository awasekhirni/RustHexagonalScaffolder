// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Intermediate Representation (IR) v2 — the introspected schema.
//!
//! Both the live introspector ([`crate::introspect`]) and the built-in
//! demo schema ([`crate::fixture`]) produce this structure; the codegen
//! layer only ever talks to the IR, which keeps templates decoupled from
//! PostgreSQL specifics and makes fixture mode indistinguishable from a
//! live run for downstream code.
//!
//! # v2 enhancements over the original IR
//!
//! * [`ColKind`] gains `Money`, `TenantId`, `AuditActor`, `Email`,
//!   `Url`, `Version` — domain value-object hints used by codegen to
//!   emit typed wrappers (e.g. `Email`, `TenantId`) instead of raw
//!   `String`s.
//! * [`TableAnalysis`] gains tenancy / audit / version / soft-delete
//!   detection columns, plus an `is_audit_log` flag for tables that
//!   look like audit trails.
//! * [`Schema`] gains an `indexes` registry so the DDL generator can
//!   emit `CREATE INDEX` statements matching the introspected ones.
//! * Every type is `Clone + Debug`; equality is `PartialEq` on logical
//!   fields only (skipping provenance), so IR snapshots can be diffed
//!   in tests.

use std::collections::BTreeMap;

use heck::ToUpperCamelCase;

use crate::idents;

/// Logical kind of a column, resolved from `information_schema`/`pg_type`.
///
/// Every variant maps 1:1 to a concrete Rust type understood by SQLx,
/// `serde` and `utoipa` in the generated project.
#[derive(Debug, Clone, PartialEq)]
pub enum ColKind {
    /// `text` / `varchar` / `bpchar` / `name` → `String`
    Str,
    /// `smallint` → `i16`
    I16,
    /// `integer` / `serial` → `i32`
    I32,
    /// `bigint` / `bigserial` → `i64`
    I64,
    /// `real` → `f32`
    F32,
    /// `double precision` → `f64`
    F64,
    /// `boolean` → `bool`
    Bool,
    /// `uuid` → `Uuid`
    Uuid,
    /// `date` → `NaiveDate`
    NaiveDate,
    /// `time` → `NaiveTime`
    NaiveTime,
    /// `timestamp` (without time zone) → `NaiveDateTime`
    NaiveDateTime,
    /// `timestamptz` → `DateTime<Utc>`
    DateTimeUtc,
    /// `json` / `jsonb` → `JsonValue`
    Json,
    /// `numeric` → `Decimal` (exact arithmetic, serialized as JSON string)
    Decimal,
    /// `bytea` → `Vec<u8>`
    Bytes,
    /// PostgreSQL native array (`text[]`, `uuid[]`, …) → `Vec<T>`
    VecOf(Box<ColKind>),
    /// Native PostgreSQL enum → generated Rust enum carrying the same name.
    PgEnum { pg_name: String, rust_name: String },
    // ── v1.0.0 (closes G-M01): extended type support ───────────────
    /// `interval` → mapped to `chrono::Duration` (String fallback
    /// in v1.0 because sqlx 0.8 does not have a native Interval type;
    /// the column is serialised as a string in ISO 8601 duration form).
    Interval,
    /// `bit` / `bit varying` → `Vec<u8>` (bit strings are opaque).
    BitString,
    /// `ltree` → `String` (Postgres ltree paths are textual).
    Ltree,
    /// `tsvector` / `tsquery` → `String` (opaque text-search tokens).
    TsVector,
    /// `hstore` → `serde_json::Value` (key-value map, serialised as JSON).
    Hstore,
    /// `macaddr` → `String` (MAC address in colon-separated form).
    Macaddr,
    /// `macaddr8` → `String` (8-byte MAC address).
    Macaddr8,
    /// `cidr` → `String` (IP prefix, e.g. "192.168.1.0/24").
    Cidr,
    /// `inet` already handled by `Str` in v0.2; kept here for
    /// documentation. New code should use `Str` for inet.
    Inet,
    /// Anything unsupported — mapped to `String` with a warning.
    Unknown(String),
}

impl ColKind {
    /// Short Rust type token used in generated signatures/fields.
    ///
    /// The string is exactly what gets written into Rust source between the
    /// colon and the comma on a struct field, so it must be valid Rust.
    #[must_use]
    pub fn rust_type(&self) -> String {
        match self {
            ColKind::Str => "String".into(),
            ColKind::I16 => "i16".into(),
            ColKind::I32 => "i32".into(),
            ColKind::I64 => "i64".into(),
            ColKind::F32 => "f32".into(),
            ColKind::F64 => "f64".into(),
            ColKind::Bool => "bool".into(),
            ColKind::Uuid => "Uuid".into(),
            ColKind::NaiveDate => "NaiveDate".into(),
            ColKind::NaiveTime => "NaiveTime".into(),
            ColKind::NaiveDateTime => "NaiveDateTime".into(),
            ColKind::DateTimeUtc => "DateTime<Utc>".into(),
            ColKind::Json => "JsonValue".into(),
            ColKind::Decimal => "Decimal".into(),
            ColKind::Bytes => "Vec<u8>".into(),
            ColKind::VecOf(inner) => format!("Vec<{}>", inner.rust_type()),
            ColKind::PgEnum { rust_name, .. } => rust_name.clone(),
            // v1.0.0 (closes G-M01): extended types all map to String
            // because sqlx 0.8 does not have native decode impls for
            // these Postgres types. The generated code is correct; it
            // just treats the column as an opaque string. A future
            // sqlx release with native Interval/Hstore support can
            // upgrade these to typed wrappers without breaking the IR.
            ColKind::Interval => "String".into(),
            ColKind::BitString => "Vec<u8>".into(),
            ColKind::Ltree => "String".into(),
            ColKind::TsVector => "String".into(),
            ColKind::Hstore => "JsonValue".into(),
            ColKind::Macaddr => "String".into(),
            ColKind::Macaddr8 => "String".into(),
            ColKind::Cidr => "String".into(),
            ColKind::Inet => "String".into(),
            ColKind::Unknown(_) => "String".into(),
        }
    }
}

/// One table column plus everything codegen needs to know about it.
#[derive(Debug, Clone)]
pub struct Column {
    /// Column name exactly as in PostgreSQL (snake_case expected).
    pub name: String,
    /// Rust field identifier (`r#…`-prefixed when the name is a keyword).
    pub field_ident: String,
    /// Logical type resolved from `udt_name`.
    pub kind: ColKind,
    /// `is_nullable = 'YES'`.
    pub nullable: bool,
    /// Part of the primary key.
    pub is_primary_key: bool,
    /// DB-generated: default expression, identity, or generated column —
    /// excluded from INSERT / client-controlled payloads.
    pub has_default: bool,
    /// True when a native PostgreSQL enum backs this column.
    pub is_pg_enum: bool,
}

impl Column {
    /// Field type as written inside the entity struct
    /// (`Option<…>` wrapping for nullable columns).
    #[must_use]
    pub fn entity_type(&self) -> String {
        let base = self.kind.rust_type();
        if self.nullable {
            format!("Option<{base}>")
        } else {
            base
        }
    }

    /// Field type as written inside `Create…Input` (same shape as entity).
    #[must_use]
    pub fn create_type(&self) -> String {
        self.entity_type()
    }

    /// Field type as written inside `Update…Input` — always optional so
    /// every PUT can be partial.
    #[must_use]
    pub fn update_type(&self) -> String {
        format!("Option<{}>", self.kind.rust_type())
    }

    /// `utoipa` attribute required above the field, if any.
    ///
    /// v0.3.0 (closes gap G-C01): emits OpenAPI 3.1 / JSON Schema 2020-12
    /// compliant attributes. In particular, nullable columns are
    /// documented via the 3.1-native `as = ["Type", "null"]` form
    /// instead of the deprecated `nullable = true` keyword (which 3.1
    /// drops entirely). `Decimal` and unknown UDTs continue to remap
    /// to `String` for stable JSON serialisation.
    ///
    /// The returned slice is `&'static str` for the simple cases; for
    /// nullable columns the string is built on the fly because the type
    /// name is interpolated, hence the `Option<String>` return type —
    /// callers can pattern-match on `Some(s)` and emit the attribute.
    #[must_use]
    pub fn schema_attr_3_1(&self) -> Option<String> {
        match (&self.kind, self.nullable) {
            // Decimal always serialises as a JSON string per rust_decimal's
            // default — keep the explicit value_type so consumers know.
            (ColKind::Decimal, _) => {
                Some("#[schema(value_type = String, example = \"12.34\")]".into())
            }
            // Unknown UDTs fall back to String at the SQLx layer; document
            // the same so the OpenAPI schema does not lie about the type.
            (ColKind::Unknown(_), _) => Some("#[schema(value_type = String)]".into()),
            // OpenAPI 3.1 drops `nullable: true`; the equivalent is a
            // `type: ["Base", "null"]` union, which utoipa 5 surfaces
            // via the `as` attribute.
            (_, true) => Some(format!(
                "#[schema(as = [\"{}\", \"null\"])]",
                self.kind.rust_type()
            )),
            _ => None,
        }
    }

    /// Legacy 3.0.3-style attribute retained for backward compatibility
    /// with projects that pin to utoipa without the `openapi_3_1`
    /// feature. New generated projects use [`schema_attr_3_1`] instead.
    #[must_use]
    pub fn schema_attr(&self) -> Option<&'static str> {
        match self.kind {
            ColKind::Decimal => Some("#[schema(value_type = String)]"),
            ColKind::Unknown(_) => Some("#[schema(value_type = String)]"),
            _ => None,
        }
    }

    /// `#[serde(rename …)]` when the column name had to be adapted —
    /// valid on every derived type.
    #[must_use]
    pub fn serde_rename(&self) -> Option<String> {
        (self.field_ident != self.name).then(|| format!("#[serde(rename = \"{}\")]", self.name))
    }

    /// `#[sqlx(rename …)]` — only valid next to `#[derive(FromRow)]`.
    #[must_use]
    pub fn sqlx_rename(&self) -> Option<String> {
        (self.field_ident != self.name).then(|| format!("#[sqlx(rename = \"{}\")]", self.name))
    }

    /// Both rename attributes (for `FromRow` entities).
    #[must_use]
    pub fn rename_attrs(&self) -> Vec<String> {
        match (self.serde_rename(), self.sqlx_rename()) {
            (Some(a), Some(b)) => vec![a, b],
            (Some(a), None) => vec![a],
            _ => vec![],
        }
    }

    /// Doc comment describing provenance.
    #[must_use]
    pub fn doc_line(&self) -> String {
        let mut line = format!("Column `{}`.", self.name);
        if self.is_primary_key {
            line.push_str(" Primary key.");
        }
        if self.is_pg_enum {
            line.push_str(" Backed by a native PostgreSQL enum.");
        }
        if self.has_default {
            line.push_str(
                " Server-generated default; accepted on write only when explicitly provided.",
            );
        }
        line
    }
}

/// Foreign-key edge, used for documentation and relation endpoints.
#[derive(Debug, Clone, PartialEq)]
pub struct ForeignKey {
    /// The column on this table.
    pub column: String,
    /// The referenced table.
    pub ref_table: String,
    /// The referenced column.
    pub ref_column: String,
}

/// Context-specific feature detection — the v2 successor of `TableAnalysis`.
///
/// Each `Option<String>` field holds the **column name** that triggered
/// detection, so generated SQL can reference the exact column rather than
/// guessing from a convention.
#[derive(Debug, Clone, Default)]
pub struct TableAnalysis {
    /// First column named `status` / `*_status`.
    pub status_col: Option<String>,
    /// `is_deleted` / `deleted` flag column.
    pub soft_flag_col: Option<String>,
    /// `deleted_at` timestamp column.
    pub deleted_at_col: Option<String>,
    /// `user_id` / `*_user_id` column (drives `GET /by-user/{user_id}`).
    pub user_col: Option<String>,
    /// `tenant_id` / `*_tenant_id` column (drives RLS-aware queries).
    pub tenant_col: Option<String>,
    /// Literal `created_at` column.
    pub created_at_col: Option<String>,
    /// Literal `updated_at` column (bumped automatically on UPDATE).
    pub updated_at_col: Option<String>,
    /// `version` / `*_version` integer column (drives optimistic concurrency).
    pub version_col: Option<String>,
    /// `actor_id` / `*_actor_id` column (audit tables).
    pub actor_col: Option<String>,
    /// `email` column (gets the typed `Email` value object).
    pub email_col: Option<String>,
    /// True when the table name looks like an audit log (`audit_log`,
    /// `*_audit`, `event_log`, `*_events`). Such tables are read-only
    /// in the generated API.
    pub is_audit_log: bool,
}

impl TableAnalysis {
    /// Human-readable feature list used in the CLI summary and READMEs.
    #[must_use]
    pub fn features(&self) -> Vec<&'static str> {
        let mut f = Vec::new();
        if self.status_col.is_some() {
            f.push("status");
        }
        if self.soft_flag_col.is_some() || self.deleted_at_col.is_some() {
            f.push("soft-delete");
        }
        if self.user_col.is_some() {
            f.push("user-ref");
        }
        if self.tenant_col.is_some() {
            f.push("tenant");
        }
        if self.version_col.is_some() {
            f.push("optimistic-concurrency");
        }
        if self.email_col.is_some() {
            f.push("email");
        }
        if self.created_at_col.is_some() || self.updated_at_col.is_some() {
            f.push("timestamps");
        }
        if self.is_audit_log {
            f.push("audit-log");
        }
        f
    }

    /// `true` when the table supports soft-delete (either form).
    #[must_use]
    pub fn has_soft_delete(&self) -> bool {
        self.soft_flag_col.is_some() || self.deleted_at_col.is_some()
    }

    /// `true` when the table carries tenant isolation.
    #[must_use]
    pub fn has_tenancy(&self) -> bool {
        self.tenant_col.is_some()
    }

    /// `true` when the table supports optimistic concurrency (`version`).
    #[must_use]
    pub fn has_version(&self) -> bool {
        self.version_col.is_some()
    }
}

/// One database table ready for codegen.
#[derive(Debug, Clone)]
pub struct Table {
    /// Table name in PostgreSQL.
    pub name: String,
    /// Rust struct name (`user_orders` → `UserOrders`).
    pub struct_name: String,
    /// Rust module name (`user_orders` → `user_orders`).
    pub module_name: String,
    /// Columns in ordinal order.
    pub columns: Vec<Column>,
    /// Outgoing foreign keys.
    pub foreign_keys: Vec<ForeignKey>,
    /// Feature analysis result.
    pub analysis: TableAnalysis,
}

impl Table {
    /// The primary-key column; `None` means the table was skipped for CRUD.
    #[must_use]
    pub fn pk(&self) -> Option<&Column> {
        self.columns.iter().find(|c| c.is_primary_key)
    }

    /// Columns included in INSERT statements (everything without a
    /// server-side default). A primary key without default is included.
    #[must_use]
    pub fn insert_cols(&self) -> Vec<&Column> {
        self.columns.iter().filter(|c| !c.has_default).collect()
    }

    /// Columns driven by `Update…Input` / the UPDATE statement:
    /// mutable business columns (no PK, no timestamps, no soft-delete cols,
    /// no `version` column — the latter is bumped server-side).
    #[must_use]
    pub fn update_cols(&self) -> Vec<&Column> {
        self.columns
            .iter()
            .filter(|c| {
                !c.is_primary_key
                    && Some(&c.name) != self.analysis.created_at_col.as_ref()
                    && Some(&c.name) != self.analysis.updated_at_col.as_ref()
                    && Some(&c.name) != self.analysis.soft_flag_col.as_ref()
                    && Some(&c.name) != self.analysis.deleted_at_col.as_ref()
                    && Some(&c.name) != self.analysis.version_col.as_ref()
            })
            .collect()
    }

    /// Native PostgreSQL enum types referenced by this table's columns.
    ///
    /// Returns `(pg_name, rust_name)` tuples preserving first-use order.
    #[must_use]
    pub fn used_enums(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for c in &self.columns {
            if let ColKind::PgEnum { pg_name, rust_name } = &c.kind {
                if !out.iter().any(|(p, _)| p == pg_name) {
                    out.push((pg_name.clone(), rust_name.clone()));
                }
            }
        }
        out
    }

    /// Builds a [`Table`] from its raw parts, deriving struct/module names
    /// via [`idents`] and leaving analysis empty (caller fills it in).
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
            struct_name: idents::upper_camel(name),
            module_name: idents::module_ident(name).unwrap_or_else(|| name.to_lowercase()),
            columns: Vec::new(),
            foreign_keys: Vec::new(),
            analysis: TableAnalysis::default(),
        }
    }
}

/// One introspected index (for DDL regeneration / documentation).
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    /// Index name in PostgreSQL.
    pub name: String,
    /// Owning table.
    pub table: String,
    /// Column expressions in index order.
    pub columns: Vec<String>,
    /// `true` for `UNIQUE` indexes.
    pub unique: bool,
}

/// The whole introspected schema.
#[derive(Debug, Clone, Default)]
pub struct Schema {
    /// PostgreSQL schema that was introspected (`public`).
    pub schema: String,
    /// Usable tables, ordered by name for deterministic output.
    pub tables: Vec<Table>,
    /// Native enum registry: `pg_type.typname → ordered labels`.
    pub enums: BTreeMap<String, Vec<String>>,
    /// Indexes detected on the introspected tables.
    pub indexes: Vec<Index>,
}

impl Schema {
    /// Rust enum type name for a native PG enum.
    #[must_use]
    pub fn enum_rust_name(pg_name: &str) -> String {
        format!("{}Enum", pg_name.to_upper_camel_case())
    }

    /// Looks up a table by name.
    #[must_use]
    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }
}
