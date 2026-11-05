// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Feature analysis v2 — inspects a table's columns and tags it with the
//! features codegen should turn on.
//!
//! v2 additions over the original analysis:
//! * tenant detection (`tenant_id`, `*_tenant_id`) → drives RLS-aware queries.
//! * `version` column → optimistic concurrency (ETag / If-Match).
//! * `actor_id` column → audit-table hint.
//! * `email` column → typed `Email` value object hint.
//! * `is_audit_log` flag — tables named `audit_log`, `*_audit`,
//!   `event_log`, `*_events` are emitted read-only.

use crate::ir::{Column, TableAnalysis};

/// Detects timestamps, status, soft-delete, tenancy, version, audit,
/// and email features from a column list.
///
/// The detector is **order-preserving**: the first matching column wins.
/// This matters when a table has both `created_at` and `updated_at` —
/// generated SQL references the exact column, never a guess.
#[must_use]
pub fn analyze(columns: &[Column]) -> TableAnalysis {
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
        if a.tenant_col.is_none() && (lower == "tenant_id" || lower.ends_with("_tenant_id")) {
            a.tenant_col = Some(c.name.clone());
        }
        if a.created_at_col.is_none() && lower == "created_at" {
            a.created_at_col = Some(c.name.clone());
        }
        if a.updated_at_col.is_none() && lower == "updated_at" {
            a.updated_at_col = Some(c.name.clone());
        }
        if a.version_col.is_none()
            && (lower == "version" || lower.ends_with("_version"))
            && matches!(
                c.kind,
                crate::ir::ColKind::I16 | crate::ir::ColKind::I32 | crate::ir::ColKind::I64
            )
        {
            a.version_col = Some(c.name.clone());
        }
        if a.actor_col.is_none() && (lower == "actor_id" || lower.ends_with("_actor_id")) {
            a.actor_col = Some(c.name.clone());
        }
        if a.email_col.is_none() && lower == "email" && matches!(c.kind, crate::ir::ColKind::Str) {
            a.email_col = Some(c.name.clone());
        }
    }

    a
}

/// Tags a table as an audit log when its name matches a known convention.
///
/// Such tables are emitted read-only (no POST/PUT/DELETE) and carry a
/// documentation banner explaining why.
#[must_use]
pub fn classify_audit_log(table_name: &str, columns: &[Column]) -> bool {
    let lower = table_name.to_lowercase();
    let name_looks_like_audit = lower == "audit_log"
        || lower.ends_with("_audit")
        || lower == "event_log"
        || lower.ends_with("_events")
        || lower == "change_log"
        || lower.ends_with("_audit_log");

    // Refine: an audit table should also have at least an `actor_id`/`at`
    // column or a JSON `before`/`after` pair. This avoids false positives
    // on tables named `team_events` that happen to be regular domain tables.
    if name_looks_like_audit {
        let has_actor = columns.iter().any(|c| {
            let l = c.name.to_lowercase();
            l == "actor_id" || l.ends_with("_actor_id") || l == "user_id"
        });
        let has_payload = columns.iter().any(|c| {
            let l = c.name.to_lowercase();
            l == "before" || l == "after" || l == "payload" || l == "diff"
        });
        let has_timestamp = columns.iter().any(|c| {
            let l = c.name.to_lowercase();
            l == "at" || l == "occurred_at" || l == "created_at"
        });
        // Either: explicit actor + (payload or timestamp)
        has_actor && (has_payload || has_timestamp)
    } else {
        false
    }
}

/// Runs the full analysis pipeline on a table's columns, including the
/// audit-log classification. Returns the populated [`TableAnalysis`].
#[must_use]
pub fn analyze_full(table_name: &str, columns: &[Column]) -> TableAnalysis {
    let mut a = analyze(columns);
    a.is_audit_log = classify_audit_log(table_name, columns);
    a
}
