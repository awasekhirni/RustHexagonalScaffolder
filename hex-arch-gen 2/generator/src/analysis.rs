//! Feature analysis of a table — the direct port of the Python
//! `analyze_table()` function, extended to remember *which* columns
//! matched so generated SQL can reference them precisely.

use crate::ir::TableAnalysis;

/// Detects timestamps, status, soft-delete and user-relation features.
pub fn analyze(columns: &[crate::ir::Column]) -> TableAnalysis {
    let mut a = TableAnalysis::default();

    for c in columns {
        let lower = c.name.to_lowercase();

        if a.status_col.is_none() && (lower == "status" || lower.ends_with("_status")) {
            a.status_col = Some(c.name.clone());
        }
        if a.soft_flag_col.is_none() && (lower == "is_deleted" || lower == "deleted") {
            a.soft_flag_col = Some(c.name.clone());
        }
        if a.deleted_at_col.is_none() && lower == "deleted_at" {
            a.deleted_at_col = Some(c.name.clone());
        }
        if a.user_col.is_none() && (lower == "user_id" || lower.ends_with("_user_id")) {
            a.user_col = Some(c.name.clone());
        }
        if a.created_at_col.is_none() && lower == "created_at" {
            a.created_at_col = Some(c.name.clone());
        }
        if a.updated_at_col.is_none() && lower == "updated_at" {
            a.updated_at_col = Some(c.name.clone());
        }
    }

    a
}
