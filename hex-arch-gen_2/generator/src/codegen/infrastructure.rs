//! Infrastructure layer codegen — SQLx adapters implementing the
//! repository ports. All queries are runtime-checked (no `query!` macros),
//! so the generated project compiles without a live `DATABASE_URL`.

use std::path::Path;

use anyhow::Result;

use crate::codegen::domain::{status_type, ts_type, user_type};
use crate::codegen::{write_file, ImportSet};
use crate::ir::{ColKind, Schema, Table};

/// Writes `src/infrastructure/<table>_repository.rs` for one table.
pub fn write_table(_schema: &Schema, root: &Path, t: &Table) -> Result<()> {
    let dir = root.join("src").join("infrastructure");
    write_file(
        &dir.join(format!("{}_repository.rs", t.module_name)),
        &repository_rs(t),
    )?;
    Ok(())
}

/// Writes `src/infrastructure/mod.rs`.
pub fn write_mod(schema: &Schema, root: &Path) -> Result<()> {
    let mut body = String::from(
        "//! Infrastructure layer — outbound adapters (SQLx/PostgreSQL).\n//!\n//! Each adapter implements the repository port of its domain context;\n//! nothing in this layer is referenced by the domain or application code\n//! except through `main`'s constructor wiring.\n\n",
    );
    for t in &schema.tables {
        body.push_str(&format!("pub mod {}_repository;\n", t.module_name));
    }
    write_file(&root.join("src").join("infrastructure").join("mod.rs"), &body)
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
            .map(|(i, c)| format!("        .bind(entity.{}.clone()) // ${}", c.field_ident, i + 1))
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
        // Degenerate table: nothing client-writable — fall back to a read.
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
            .map(|(i, c)| format!("        .bind(entity.{}.clone()) // ${}", c.field_ident, i + 1))
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

    if t.analysis.soft_flag_col.is_some() || t.analysis.deleted_at_col.is_some() {
        // build the SET fragments for soft delete / restore
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

    // ── imports ─────────────────────────────────────────────────────────
    let mut out = String::new();
    out.push_str(&format!(
        "//! PostgreSQL adapter (outbound) for the `{}` repository port.\n//!\n//! Generated by `hex-gen`. Queries are runtime-checked — the project\n//! compiles without a database connection, and SQL problems surface as\n//! [`RepositoryError`]s instead of build failures.\n\n",
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
        "use crate::domain::{}::entity::{};\nuse crate::domain::{}::repository::{}Repository;\nuse crate::enterprise::repository::{{map_db_error, Page, Paginated, Repository, RepositoryError}};\n",
        t.module_name, struct_name, t.module_name, struct_name
    ));
    for (_, rust_name) in t.used_enums() {
        out.push_str(&format!(
            "use crate::domain::{}::enums::{};\n",
            t.module_name, rust_name
        ));
    }

    out.push_str(&format!(
        "\n/// SQLx-backed adapter for the `{table}` table.\npub struct Postgres{struct_name}Repository {{\n    pool: PgPool,\n}}\n\nimpl Postgres{struct_name}Repository {{\n    /// Creates the adapter over a shared connection pool.\n    pub fn new(pool: PgPool) -> Self {{\n        Self {{ pool }}\n    }}\n}}\n\n#[async_trait]\nimpl Repository<{struct_name}, {pk_ty}> for Postgres{struct_name}Repository {{\n    async fn find_by_id(&self, id: {pk_ty}) -> Result<Option<{struct_name}>, RepositoryError> {{\n        sqlx::query_as::<_, {struct_name}>(\n            \"SELECT * FROM {table} WHERE {pk_col} = $1\",\n        )\n        .bind(id.clone())\n        .fetch_optional(&self.pool)\n        .await\n        .map_err(map_db_error)\n    }}\n\n    async fn find_all(&self, page: Page) -> Result<Paginated<{struct_name}>, RepositoryError> {{\n        let items = sqlx::query_as::<_, {struct_name}>(\n            \"SELECT * FROM {table} LIMIT $1 OFFSET $2\",\n        )\n        .bind(page.limit())\n        .bind(page.offset())\n        .fetch_all(&self.pool)\n        .await\n        .map_err(map_db_error)?;\n        let total = self.count().await?;\n        Ok(Paginated {{ items, total, page: page.page, per_page: page.per_page }})\n    }}\n\n    async fn create(&self, entity: &{struct_name}) -> Result<{struct_name}, RepositoryError> {{\n        sqlx::query_as::<_, {struct_name}>(\n            \"{insert_sql}\",\n        )\n{insert_binds}\n        .fetch_one(&self.pool)\n        .await\n        .map_err(map_db_error)\n    }}\n\n    async fn update(&self, id: {pk_ty}, entity: &{struct_name}) -> Result<Option<{struct_name}>, RepositoryError> {{\n        sqlx::query_as::<_, {struct_name}>(\n            \"{update_sql}\",\n        )\n{update_binds}\n        .fetch_optional(&self.pool)\n        .await\n        .map_err(map_db_error)\n    }}\n\n    async fn delete(&self, id: {pk_ty}) -> Result<bool, RepositoryError> {{\n        let result = sqlx::query(\n            \"DELETE FROM {table} WHERE {pk_col} = $1\",\n        )\n        .bind(id.clone())\n        .execute(&self.pool)\n        .await\n        .map_err(map_db_error)?;\n        Ok(result.rows_affected() > 0)\n    }}\n\n    async fn count(&self) -> Result<i64, RepositoryError> {{\n        let (total,): (i64,) = sqlx::query_as(\n            \"SELECT COUNT(*) FROM {table}\",\n        )\n        .fetch_one(&self.pool)\n        .await\n        .map_err(map_db_error)?;\n        Ok(total)\n    }}\n}}\n\n#[async_trait]\nimpl {struct_name}Repository for Postgres{struct_name}Repository {{\n{extra_impl}}}\n",
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
