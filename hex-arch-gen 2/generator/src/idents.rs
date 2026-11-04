//! Rust identifier hygiene for generated code.
//!
//! PostgreSQL is permissive with identifiers; Rust is not. This module
//! guarantees that every generated field/module name is a valid identifier
//! and keeps the original SQL name for `#[serde(rename)]`/`#[sqlx(rename)]`.

/// Rust strict/reserved keywords (2021 edition).
const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "Self", "static",
    "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do",
    "final", "macro", "override", "priv", "typeof", "unsized", "virtual",
    "yield", "try", "gen",
];

/// Returns `true` when `name` is usable as a Rust identifier verbatim.
pub fn is_plain_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let first_ok = chars.next().map_or(false, |c| c.is_ascii_alphabetic() || c == '_');
    first_ok
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !KEYWORDS.contains(&name)
}

/// Sanitizes a column name into a valid field identifier, returning
/// `(ident, changed)`. Keywords become raw identifiers (`r#type`);
/// exotic characters collapse to `_`.
pub fn field_ident(name: &str) -> (String, bool) {
    if is_plain_ident(name) {
        return (name.to_string(), false);
    }

    let valid_shape = {
        let mut chars = name.chars();
        matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };

    // Valid identifier shape but reserved — escape as a raw identifier.
    // (`self`/`Self`/`crate`/`super` cannot be raw; rename them instead.)
    if valid_shape && !matches!(name, "self" | "Self" | "crate" | "super") {
        return (format!("r#{name}"), true);
    }

    // Fallback: sanitize exotic names character by character.
    let mut ident: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if ident.starts_with(|c: char| c.is_ascii_digit()) {
        ident.insert(0, '_');
    }
    (ident, true)
}

/// Sanitizes a table name into a valid module identifier; `None` when the
/// table cannot be safely generated (caller skips it with a warning).
pub fn module_ident(name: &str) -> Option<String> {
    let (ident, changed) = field_ident(name);
    if ident.starts_with("r#") {
        // A table literally called `type` etc. — rename to `<name>_table`.
        Some(format!("{}_table", &ident[2..]))
    } else if changed {
        Some(ident)
    } else {
        Some(ident)
    }
}
