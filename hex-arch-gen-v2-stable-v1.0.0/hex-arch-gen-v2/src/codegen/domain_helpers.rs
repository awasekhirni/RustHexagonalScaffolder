// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Shared helpers consumed by `application.rs`, `infrastructure.rs`, and
//! `presentation.rs`. Extracted from `domain.rs` so the per-table codegen
//! modules do not form a cyclic dependency.

use crate::ir::{ColKind, Column, Table};

/// Rust type used to *name* a status value in ports and handlers.
#[must_use]
pub fn status_type(t: &Table) -> Option<String> {
    let col = t
        .columns
        .iter()
        .find(|c| Some(&c.name) == t.analysis.status_col.as_ref())?;
    Some(match &col.kind {
        ColKind::PgEnum { rust_name, .. } => rust_name.clone(),
        _ => "String".to_string(),
    })
}

/// Rust type of the user-reference column, when present.
#[must_use]
pub fn user_type(t: &Table) -> Option<String> {
    let col = t
        .columns
        .iter()
        .find(|c| Some(&c.name) == t.analysis.user_col.as_ref())?;
    Some(col.kind.rust_type())
}

/// Rust type of a timestamp feature column (`created_at`/`updated_at`).
#[must_use]
pub fn ts_type(t: &Table, which: &Option<String>) -> Option<String> {
    let col = t.columns.iter().find(|c| Some(&c.name) == which.as_ref())?;
    Some(col.kind.rust_type())
}

/// Columns referenced by port signatures (for import calculation).
///
/// Returns references tied to the table's lifetime; the caller passes them
/// to [`crate::codegen::ImportSet::from_cols`] which only needs `&Column`.
#[must_use]
pub fn port_feature_cols(t: &Table) -> Vec<&Column> {
    let mut cols: Vec<&Column> = Vec::new();
    for name in [
        t.analysis.status_col.as_ref(),
        t.analysis.deleted_at_col.as_ref(),
        t.analysis.user_col.as_ref(),
        t.analysis.created_at_col.as_ref(),
        t.analysis.updated_at_col.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(c) = t.columns.iter().find(|c| &c.name == name) {
            cols.push(c);
        }
    }
    cols
}
