// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Built-in demo schema — lets you exercise the full generator without a
//! live database (`hex-gen --fixture`).
//!
//! The three tables deliberately exercise every feature detector:
//!
//! | table               | features demonstrated                                                  |
//! |---------------------|------------------------------------------------------------------------|
//! | `users`             | native PG enum, jsonb, arrays, timestamps, status, email, version     |
//! | `orders`            | FK to users, numeric money, soft delete, user-ref, timestamps, enum, tenant |
//! | `product_categories`| integer serial PK, varchar status (String, not enum), `type` keyword   |

use std::collections::BTreeMap;

use crate::analysis;
use crate::ir::{ColKind, Column, ForeignKey, Index, Schema, Table};

/// Builds the demo IR (identical to loading `fixture.sql` into PostgreSQL
/// and running the live introspector).
#[must_use]
pub fn demo_schema() -> Schema {
    let mut enums = BTreeMap::new();
    enums.insert(
        "user_status".to_string(),
        vec!["active".into(), "inactive".into(), "suspended".into()],
    );
    enums.insert(
        "order_status".to_string(),
        vec![
            "pending".into(),
            "paid".into(),
            "shipped".into(),
            "delivered".into(),
            "cancelled".into(),
        ],
    );

    let users_cols = vec![
        col("id", ColKind::Uuid, false, true, true),
        col("email", ColKind::Str, false, false, false),
        col("full_name", ColKind::Str, true, false, false),
        col(
            "user_status",
            ColKind::PgEnum {
                pg_name: "user_status".into(),
                rust_name: "UserStatusEnum".into(),
            },
            false,
            false,
            true,
        ),
        col("is_active", ColKind::Bool, false, false, true),
        col("metadata", ColKind::Json, true, false, false),
        col(
            "tags",
            ColKind::VecOf(Box::new(ColKind::Str)),
            true,
            false,
            false,
        ),
        col("version", ColKind::I32, false, false, true),
        col("created_at", ColKind::DateTimeUtc, false, false, true),
        col("updated_at", ColKind::DateTimeUtc, false, false, true),
        col("deleted_at", ColKind::DateTimeUtc, true, false, false),
    ];

    let users = Table {
        name: "users".into(),
        struct_name: "Users".into(),
        module_name: "users".into(),
        columns: users_cols,
        foreign_keys: vec![],
        analysis: Default::default(),
    };

    let orders_cols = vec![
        col("id", ColKind::Uuid, false, true, true),
        col("user_id", ColKind::Uuid, false, false, false),
        col("tenant_id", ColKind::Uuid, false, false, false),
        col("amount", ColKind::Decimal, false, false, false),
        col(
            "order_status",
            ColKind::PgEnum {
                pg_name: "order_status".into(),
                rust_name: "OrderStatusEnum".into(),
            },
            false,
            false,
            true,
        ),
        col("notes", ColKind::Str, true, false, false),
        col("is_deleted", ColKind::Bool, false, false, true),
        col("deleted_at", ColKind::DateTimeUtc, true, false, false),
        col("version", ColKind::I32, false, false, true),
        col("created_at", ColKind::DateTimeUtc, false, false, true),
        col("updated_at", ColKind::DateTimeUtc, false, false, true),
    ];

    let orders = Table {
        name: "orders".into(),
        struct_name: "Orders".into(),
        module_name: "orders".into(),
        columns: orders_cols,
        foreign_keys: vec![ForeignKey {
            column: "user_id".into(),
            ref_table: "users".into(),
            ref_column: "id".into(),
        }],
        analysis: Default::default(),
    };

    // Third shape: integer serial PK, varchar status (String, not enum),
    // and a column named `type` — exercises keyword handling (`r#type`).
    let categories_cols = vec![
        col("id", ColKind::I32, false, true, true),
        col("name", ColKind::Str, false, false, false),
        col("type", ColKind::Str, false, false, false),
        col("status", ColKind::Str, false, false, false),
        col("created_at", ColKind::DateTimeUtc, false, false, true),
    ];

    let categories = Table {
        name: "product_categories".into(),
        struct_name: "ProductCategories".into(),
        module_name: "product_categories".into(),
        columns: categories_cols,
        foreign_keys: vec![],
        analysis: Default::default(),
    };

    let mut tables = vec![users, orders, categories];
    tables.sort_by(|a, b| a.name.cmp(&b.name));
    for table in &mut tables {
        table.analysis = analysis::analyze_full(&table.name, &table.columns);
    }

    // A couple of secondary indexes so the DDL generator has something to emit.
    let indexes = vec![
        Index {
            name: "users_email_key".into(),
            table: "users".into(),
            columns: vec!["email".into()],
            unique: true,
        },
        Index {
            name: "orders_user_id_idx".into(),
            table: "orders".into(),
            columns: vec!["user_id".into()],
            unique: false,
        },
        Index {
            name: "orders_tenant_status_idx".into(),
            table: "orders".into(),
            columns: vec!["tenant_id".into(), "order_status".into()],
            unique: false,
        },
    ];

    Schema {
        schema: "public".into(),
        tables,
        enums,
        indexes,
    }
}

/// Column constructor shorthand.
fn col(name: &str, kind: ColKind, nullable: bool, pk: bool, has_default: bool) -> Column {
    let is_pg_enum = matches!(&kind, ColKind::PgEnum { .. });
    let (field_ident, _changed) = crate::idents::field_ident(name);
    Column {
        name: name.to_string(),
        field_ident,
        kind,
        nullable,
        is_primary_key: pk,
        has_default,
        is_pg_enum,
    }
}

// ════════════════════════════════════════════════════════════════════════════
// v0.5.0 (closes G-L08): FixtureBuilder DSL
// ════════════════════════════════════════════════════════════════════════════

/// Builder DSL for constructing demo schemas without writing a full
/// `demo_schema()` function.
///
/// v0.5.0 (closes G-L08). The previous fixture module was a single
/// hard-coded `demo_schema()` function — adding a new test table
/// required editing the function. This builder lets test code
/// construct ad-hoc schemas:
///
/// ```
/// use hex_arch_gen::fixture::FixtureBuilder;
/// use hex_arch_gen::ir::ColKind;
///
/// let schema = FixtureBuilder::new("public")
///     .table("users", |t| {
///         t.col("id", ColKind::Uuid, false).primary_key().has_default();
///         t.col("email", ColKind::Str, false);
///     })
///     .table("orders", |t| {
///         t.col("id", ColKind::Uuid, false).primary_key().has_default();
///         t.col("user_id", ColKind::Uuid, false).fk("users", "id");
///     })
///     .build();
/// assert_eq!(schema.tables.len(), 2);
/// ```
///
/// The builder is intentionally minimal — it covers the common case
/// (a few tables with simple columns and FKs) and falls back to the
/// full `demo_schema()` for anything more elaborate.
#[derive(Debug, Clone)]
pub struct FixtureBuilder {
    schema: String,
    tables: Vec<TableBuilder>,
    enums: BTreeMap<String, Vec<String>>,
    indexes: Vec<Index>,
}

impl FixtureBuilder {
    /// Constructs a new builder targeting the named Postgres schema.
    #[must_use]
    pub fn new(schema: &str) -> Self {
        Self {
            schema: schema.to_string(),
            tables: Vec::new(),
            enums: BTreeMap::new(),
            indexes: Vec::new(),
        }
    }

    /// Adds a table to the fixture. The closure receives a
    /// [`TableBuilder`] it can call `.col(...)` on to add columns.
    #[must_use]
    pub fn table<F>(mut self, name: &str, build: F) -> Self
    where
        F: FnOnce(&mut TableBuilder),
    {
        let mut tb = TableBuilder::new(name);
        build(&mut tb);
        self.tables.push(tb);
        self
    }

    /// Adds a native PG enum to the fixture.
    #[must_use]
    pub fn pg_enum(mut self, name: &str, labels: Vec<&str>) -> Self {
        self.enums.insert(
            name.to_string(),
            labels.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    /// Builds the final [`Schema`] IR.
    ///
    /// Runs the v2 analysis pipeline on each table so the produced
    /// IR is indistinguishable from a live-introspected schema.
    #[must_use]
    pub fn build(self) -> Schema {
        let mut tables: Vec<Table> = self.tables.into_iter().map(|tb| tb.finalize()).collect();
        // Tables are processed in name order for deterministic output
        // (mirrors the live introspector's behaviour).
        tables.sort_by(|a, b| a.name.cmp(&b.name));
        Schema {
            schema: self.schema,
            tables,
            enums: self.enums,
            indexes: self.indexes,
        }
    }
}

/// Builder for one table inside a [`FixtureBuilder`].
#[derive(Debug, Clone)]
pub struct TableBuilder {
    name: String,
    struct_name: String,
    module_name: String,
    columns: Vec<Column>,
    foreign_keys: Vec<ForeignKey>,
}

impl TableBuilder {
    /// Constructs a new table builder. The struct and module names
    /// are derived from the table name via [`idents::upper_camel`]
    /// and [`idents::module_ident`].
    #[must_use]
    pub fn new(name: &str) -> Self {
        let struct_name = crate::idents::upper_camel(name);
        let module_name = crate::idents::module_ident(name).unwrap_or_else(|| name.to_lowercase());
        Self {
            name: name.to_string(),
            struct_name,
            module_name,
            columns: Vec::new(),
            foreign_keys: Vec::new(),
        }
    }

    /// Adds a column to the table. Returns `&mut self` so the caller
    /// can chain `.primary_key()` / `.has_default()` / `.fk(...)` on
    /// the same line.
    pub fn col(&mut self, name: &str, kind: ColKind, nullable: bool) -> &mut Self {
        let (field_ident, _) = crate::idents::field_ident(name);
        let is_pg_enum = matches!(&kind, ColKind::PgEnum { .. });
        self.columns.push(Column {
            name: name.to_string(),
            field_ident,
            kind,
            nullable,
            is_primary_key: false,
            has_default: false,
            is_pg_enum,
        });
        // The last pushed column is the one we are configuring.
        // The chained methods below operate on it.
        // (This is safe because `col` always pushes exactly one.)
        self
    }

    /// Marks the most recently added column as the primary key.
    pub fn primary_key(&mut self) -> &mut Self {
        if let Some(c) = self.columns.last_mut() {
            c.is_primary_key = true;
        }
        self
    }

    /// Marks the most recently added column as having a server-side
    /// default (excluded from INSERT payloads).
    pub fn has_default(&mut self) -> &mut Self {
        if let Some(c) = self.columns.last_mut() {
            c.has_default = true;
        }
        self
    }

    /// Marks the most recently added column as a foreign key to
    /// `(ref_table, ref_column)`.
    pub fn fk(&mut self, ref_table: &str, ref_column: &str) -> &mut Self {
        if let Some(c) = self.columns.last() {
            self.foreign_keys.push(ForeignKey {
                column: c.name.clone(),
                ref_table: ref_table.to_string(),
                ref_column: ref_column.to_string(),
            });
        }
        self
    }

    /// Finalises the table — runs the v2 analysis pipeline and
    /// constructs the [`Table`] IR.
    fn finalize(self) -> Table {
        let analysis = analysis::analyze_full(&self.name, &self.columns);
        Table {
            name: self.name,
            struct_name: self.struct_name,
            module_name: self.module_name,
            columns: self.columns,
            foreign_keys: self.foreign_keys,
            analysis,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_builder_basic() {
        let schema = FixtureBuilder::new("public")
            .table("users", |t| {
                t.col("id", ColKind::Uuid, false)
                    .primary_key()
                    .has_default();
                t.col("email", ColKind::Str, false);
            })
            .table("orders", |t| {
                t.col("id", ColKind::Uuid, false)
                    .primary_key()
                    .has_default();
                t.col("user_id", ColKind::Uuid, false).fk("users", "id");
            })
            .build();
        assert_eq!(schema.tables.len(), 2);
        // Sorted by name: orders, users.
        assert_eq!(schema.tables[0].name, "orders");
        assert_eq!(schema.tables[1].name, "users");
        assert!(schema.tables[0]
            .foreign_keys
            .iter()
            .any(|f| f.ref_table == "users"));
    }

    #[test]
    fn fixture_builder_with_enum() {
        let schema = FixtureBuilder::new("public")
            .pg_enum("status", vec!["active", "inactive"])
            .table("users", |t| {
                t.col("id", ColKind::Uuid, false)
                    .primary_key()
                    .has_default();
            })
            .build();
        assert_eq!(schema.enums.len(), 1);
        assert_eq!(schema.enums["status"], vec!["active", "inactive"]);
    }

    #[test]
    fn fixture_builder_empty() {
        let schema = FixtureBuilder::new("public").build();
        assert_eq!(schema.tables.len(), 0);
        assert_eq!(schema.schema, "public");
    }
}
