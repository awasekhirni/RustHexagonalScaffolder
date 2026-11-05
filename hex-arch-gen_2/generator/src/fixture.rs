//! Built-in demo schema — lets you exercise the full generator without a
//! live database (`hex-gen --fixture`).
//!
//! The two tables deliberately exercise every feature detector:
//!
//! | table    | features demonstrated                                            |
//! |----------|------------------------------------------------------------------|
//! | `users`  | native PG enum, jsonb, arrays, timestamps, status                |
//! | `orders` | FK to users, numeric money, soft delete, user-ref, timestamps, enum |

use crate::ir::{ColKind, Column, ForeignKey, Schema, Table};

/// Builds the demo IR (identical to loading `fixture.sql` into PostgreSQL
/// and running the live introspector).
pub fn demo_schema() -> Schema {
    let mut enums = std::collections::BTreeMap::new();
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
                ColKind::PgEnum { pg_name: "user_status".into(), rust_name: "UserStatusEnum".into() },
                false, false, true,
            ),
            col("is_active", ColKind::Bool, false, false, true),
            col("metadata", ColKind::Json, true, false, false),
            col("tags", ColKind::VecOf(Box::new(ColKind::Str)), true, false, false),
            col("created_at", ColKind::DateTimeUtc, false, false, true),
            col("updated_at", ColKind::DateTimeUtc, false, false, true),
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
            col("amount", ColKind::Decimal, false, false, false),
            col(
                "order_status",
                ColKind::PgEnum { pg_name: "order_status".into(), rust_name: "OrderStatusEnum".into() },
                false, false, true,
            ),
            col("notes", ColKind::Str, true, false, false),
            col("is_deleted", ColKind::Bool, false, false, true),
            col("deleted_at", ColKind::DateTimeUtc, true, false, false),
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
        table.analysis = crate::analysis::analyze(&table.columns);
    }

    Schema {
        schema: "public".into(),
        tables,
        enums,
    }
}

/// Column constructor shorthand.
#[allow(clippy::too_many_arguments)]
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
