// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Identifier helpers — convert PostgreSQL identifiers (snake_case) into
//! safe Rust identifiers (module, field, struct, variant names).
//!
//! Centralising the rules here means every codegen module produces the
//! *same* identifier for the same input, so cross-module `use` paths
//! always line up.
//!
//! v0.5.0 (closes G-L07): adds [`detect_struct_name_conflicts`], which
//! scans a list of table names and returns a list of conflicts where
//! two distinct tables would map to the same `UpperCamel` struct name.
//! The generator calls this in `main::run` after introspection and
//! logs a warning per conflict so the operator can rename one of the
//! tables before generation produces ambiguous Rust.

use heck::ToUpperCamelCase;

/// Returns a valid Rust module identifier for a table name, or `None` when
/// the name cannot be salvaged (empty, starts with a digit, contains
/// characters outside `[a-zA-Z0-9_]` after sanitisation).
///
/// Postgres allows almost anything in a quoted identifier; we intentionally
/// reject exotic names rather than guess a sanitisation strategy.
///
/// # Errors
///
/// This function never returns an error — it returns `None` for inputs
/// that cannot be salvaged, and the caller is expected to skip the
/// table with a warning.
#[must_use]
pub fn module_ident(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let first = trimmed.chars().next()?;
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    Some(trimmed.to_lowercase())
}

/// Returns the field identifier for a column name, applying the `r#` prefix
/// when the name is a Rust keyword (`type`, `match`, `ref`, …).
///
/// The second return value is `true` when the prefix was applied — used by
/// templates to decide whether to emit `#[serde(rename = "…")]`.
#[must_use]
pub fn field_ident(name: &str) -> (String, bool) {
    if is_rust_keyword(name) {
        (format!("r#{name}"), true)
    } else {
        (name.to_string(), false)
    }
}

/// Returns the UpperCamelCase struct name for a table or column.
///
/// `user_orders` → `UserOrders`, `product_categories` → `ProductCategories`.
#[must_use]
pub fn upper_camel(name: &str) -> String {
    name.to_upper_camel_case()
}

/// Returns `true` when the identifier is a reserved Rust keyword.
///
/// The list is the union of `strict` and `reserved` keywords from the
/// Rust Reference (edition 2021).
#[must_use]
pub fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
            | "union"
    )
}

/// v0.5.0 (closes G-L07): detects when two distinct PostgreSQL table
/// names map to the same `UpperCamel` Rust struct name.
///
/// This happens when two tables differ only in snake_case spelling —
/// for example `user_orders` and `userOrders` both map to
/// `UserOrders`. Postgres treats these as distinct tables, but
/// `heck::ToUpperCamelCase` produces the same Rust identifier for
/// both, which would cause a compile error in the generated project
/// (two structs with the same name in the same module).
///
/// The function returns a list of `(struct_name, [table_a, table_b])`
/// pairs for every conflict found. The caller is expected to log a
/// warning per conflict and either rename one of the tables or pass
/// `--exclude-tables` to skip one of them.
///
/// # CMMI mapping
///
/// * **CAR SG 1** — Determine root causes of defects: the conflict
///   is a root cause for "two structs with the same name" compile
///   errors; surfacing it at codegen time lets the operator fix the
///   schema before the error reaches `cargo check`.
///
/// # Examples
///
/// ```
/// use hex_arch_gen::idents::detect_struct_name_conflicts;
/// let conflicts = detect_struct_name_conflicts(&["user_orders", "userOrders", "orders"]);
/// assert_eq!(conflicts.len(), 1);
/// assert_eq!(conflicts[0].0, "UserOrders");
/// ```
#[must_use]
pub fn detect_struct_name_conflicts(table_names: &[&str]) -> Vec<(String, Vec<String>)> {
    use std::collections::HashMap;
    let mut by_struct: HashMap<String, Vec<String>> = HashMap::new();
    for name in table_names {
        let struct_name = upper_camel(name);
        by_struct
            .entry(struct_name)
            .or_default()
            .push((*name).to_string());
    }
    by_struct
        .into_iter()
        .filter(|(_, tables)| tables.len() > 1)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_ident_plain() {
        assert_eq!(module_ident("users"), Some("users".into()));
        assert_eq!(module_ident("UserOrders"), Some("userorders".into()));
    }

    #[test]
    fn module_ident_rejects_exotic() {
        assert_eq!(module_ident(""), None);
        assert_eq!(module_ident("1table"), None);
        assert_eq!(module_ident("table-name"), None);
        assert_eq!(module_ident("table name"), None);
    }

    #[test]
    fn field_ident_keyword() {
        let (s, prefixed) = field_ident("type");
        assert_eq!(s, "r#type");
        assert!(prefixed);
    }

    #[test]
    fn field_ident_non_keyword() {
        let (s, prefixed) = field_ident("email");
        assert_eq!(s, "email");
        assert!(!prefixed);
    }

    #[test]
    fn upper_camel_basic() {
        assert_eq!(upper_camel("user_orders"), "UserOrders");
        assert_eq!(upper_camel("product_categories"), "ProductCategories");
    }

    // ─── v0.5.0 (closes G-L07): conflict detection tests ───────────────

    #[test]
    fn detect_struct_name_conflicts_no_conflicts() {
        let conflicts = detect_struct_name_conflicts(&["users", "orders", "products"]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn detect_struct_name_conflicts_snake_vs_camel() {
        // `user_orders` and `userOrders` both produce `UserOrders`.
        let conflicts = detect_struct_name_conflicts(&["user_orders", "userOrders"]);
        assert_eq!(conflicts.len(), 1);
        let (struct_name, tables) = &conflicts[0];
        assert_eq!(struct_name, "UserOrders");
        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"user_orders".to_string()));
        assert!(tables.contains(&"userOrders".to_string()));
    }

    #[test]
    fn detect_struct_name_conflicts_underscore_collisions() {
        // `user_orders` and `user__orders` both produce `UserOrders`
        // because heck collapses repeated underscores.
        let conflicts = detect_struct_name_conflicts(&["user_orders", "user__orders"]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].0, "UserOrders");
    }

    #[test]
    fn detect_struct_name_conflicts_empty_input() {
        assert!(detect_struct_name_conflicts(&[]).is_empty());
    }
}
