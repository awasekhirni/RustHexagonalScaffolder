//! Intermediate Representation (IR) of the introspected database schema.
//!
//! Both the live introspector ([`crate::introspect`]) and the built-in
//! demo schema ([`crate::fixture`]) produce this structure; the code
//! generator ([`crate::codegen`]) only ever talks to the IR, which keeps
//! templates decoupled from PostgreSQL specifics.

use std::collections::BTreeMap;

use heck::ToUpperCamelCase;

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
    /// `numeric` → `Decimal` (exact arithmetic, serialized as JSON number)
    Decimal,
    /// `bytea` → `Vec<u8>`
    Bytes,
    /// PostgreSQL native array (`text[]`, `uuid[]`, …) → `Vec<T>`
    VecOf(Box<ColKind>),
    /// Native PostgreSQL enum → generated Rust enum carrying the same name.
    PgEnum { pg_name: String, rust_name: String },
    /// Anything unsupported — mapped to `String` with a warning.
    Unknown(String),
}

impl ColKind {
    /// Short Rust type token used in generated signatures/fields.
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
    pub fn entity_type(&self) -> String {
        let base = self.kind.rust_type();
        if self.nullable {
            format!("Option<{base}>")
        } else {
            base
        }
    }

    /// Field type as written inside `Create…Input` (same shape as entity).
    pub fn create_type(&self) -> String {
        self.entity_type()
    }

    /// Field type as written inside `Update…Input` — always optional so
    /// every PUT can be partial.
    pub fn update_type(&self) -> String {
        format!("Option<{}>", self.kind.rust_type())
    }

    /// `utoipa` attribute required above the field, if any.
    ///
    /// `Decimal` serializes to a JSON *string* (rust_decimal's default) —
    /// the precise, float-safe representation money APIs should document.
    pub fn schema_attr(&self) -> Option<&'static str> {
        match self.kind {
            ColKind::Decimal => Some("#[schema(value_type = String)]"),
            ColKind::Unknown(_) => Some("#[schema(value_type = String)]"),
            _ => None,
        }
    }

    /// `#[serde(rename …)]` when the column name had to be adapted —
    /// valid on every derived type.
    pub fn serde_rename(&self) -> Option<String> {
        (self.field_ident != self.name)
            .then(|| format!("#[serde(rename = \"{}\")]", self.name))
    }

    /// `#[sqlx(rename …)]` — only valid next to `#[derive(FromRow)]`.
    pub fn sqlx_rename(&self) -> Option<String> {
        (self.field_ident != self.name)
            .then(|| format!("#[sqlx(rename = \"{}\")]", self.name))
    }

    /// Both rename attributes (for `FromRow` entities).
    pub fn rename_attrs(&self) -> Vec<String> {
        match (self.serde_rename(), self.sqlx_rename()) {
            (Some(a), Some(b)) => vec![a, b],
            (Some(a), None) => vec![a],
            _ => vec![],
        }
    }

    /// Doc comment describing provenance.
    pub fn doc_line(&self) -> String {
        let mut line = format!("Column `{}`.", self.name);
        if self.is_primary_key {
            line.push_str(" Primary key.");
        }
        if self.is_pg_enum {
            line.push_str(" Backed by a native PostgreSQL enum.");
        }
        if self.has_default {
            line.push_str(" Server-generated default; accepted on write only when explicitly provided.");
        }
        line
    }
}

/// Foreign-key edge, used for documentation and future relation endpoints.
#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub column: String,
    pub ref_table: String,
    pub ref_column: String,
}

/// Context-specific feature detection — the Rust counterpart of the
/// Python `analyze_table()` function.
#[derive(Debug, Clone, Default)]
pub struct TableAnalysis {
    /// First column named `status` / `*_status`.
    pub status_col: Option<String>,
    /// `is_deleted` / `deleted` flag column.
    pub soft_flag_col: Option<String>,
    /// `deleted_at` timestamp column.
    pub deleted_at_col: Option<String>,
    /// `user_id` / `*_user_id` column.
    pub user_col: Option<String>,
    /// Literal `created_at` column (drives `created-after` endpoint).
    pub created_at_col: Option<String>,
    /// Literal `updated_at` column (bumped automatically on UPDATE).
    pub updated_at_col: Option<String>,
}

impl TableAnalysis {
    /// Human-readable feature list used in the CLI summary and READMEs.
    pub fn features(&self) -> Vec<&'static str> {
        let mut f = Vec::new();
        if self.status_col.is_some() { f.push("status"); }
        if self.soft_flag_col.is_some() || self.deleted_at_col.is_some() { f.push("soft-delete"); }
        if self.user_col.is_some() { f.push("user-ref"); }
        if self.created_at_col.is_some() || self.updated_at_col.is_some() { f.push("timestamps"); }
        f
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
    pub columns: Vec<Column>,
    pub foreign_keys: Vec<ForeignKey>,
    pub analysis: TableAnalysis,
}

impl Table {
    /// The primary-key column; `None` means the table was skipped for CRUD.
    pub fn pk(&self) -> Option<&Column> {
        self.columns.iter().find(|c| c.is_primary_key)
    }

    /// Columns included in INSERT statements (everything without a
    /// server-side default). A primary key without default is included.
    pub fn insert_cols(&self) -> Vec<&Column> {
        self.columns.iter().filter(|c| !c.has_default).collect()
    }

    /// Columns driven by `Update…Input` / the UPDATE statement:
    /// mutable business columns (no PK, no timestamps, no soft-delete cols).
    pub fn update_cols(&self) -> Vec<&Column> {
        self.columns
            .iter()
            .filter(|c| {
                !c.is_primary_key
                    && Some(&c.name) != self.analysis.created_at_col.as_ref()
                    && Some(&c.name) != self.analysis.updated_at_col.as_ref()
                    && Some(&c.name) != self.analysis.soft_flag_col.as_ref()
                    && Some(&c.name) != self.analysis.deleted_at_col.as_ref()
            })
            .collect()
    }

    /// Native PostgreSQL enum types referenced by this table's columns.
    pub fn used_enums(&self) -> Vec<(String, String)> {
        // (pg_name, rust_name) preserving first-use order.
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
}

impl Schema {
    /// Rust enum type name for a native PG enum.
    pub fn enum_rust_name(pg_name: &str) -> String {
        format!("{}Enum", pg_name.to_upper_camel_case())
    }
}
