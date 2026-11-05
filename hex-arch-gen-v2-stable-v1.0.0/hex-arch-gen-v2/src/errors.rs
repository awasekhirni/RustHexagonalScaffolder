// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Typed error taxonomy for the generator.
//!
//! Every failure inside the generator surfaces as a [`GenError`] variant,
//! never a panic and never a bare `String`. The taxonomy is consumed by:
//!
//! * `main.rs` — converts the error to a process exit code and a log line.
//! * `ERROR_INVENTORY.md` — the persistent record of every error we have
//!   ever hit during development, kept up to date per the master prompt.
//! * `error_inventory.json` — the machine-readable sibling inventory
//!   introduced by `error_inventory.rs` (CMMI MA / CAR traceability).
//!
//! Design notes:
//! * [`thiserror`] is used so each variant gets a `#[from]` impl for
//!   ergonomic `?` propagation without losing the original cause.
//! * Every variant carries enough structured context (file path, table
//!   name, SQLSTATE, …) to be actionable without grepping logs.
//! * Every variant carries a stable [`GenError::error_code`] so the
//!   generated runtime, the generator log lines, and the error inventory
//!   can all reference the same defect class (CMMI MA SG 1 "specify
//!   measurement objectives"; CAR SG 1 "determine root causes").

use std::path::PathBuf;

use thiserror::Error;

/// Convenience alias used throughout the generator.
pub type GenResult<T> = std::result::Result<T, GenError>;

/// The generator error taxonomy.
///
/// Variants are grouped by lifecycle stage so the top of the match in
/// `main.rs` reads as a state machine: config → introspect → analysis →
/// codegen → io.
#[derive(Debug, Error)]
pub enum GenError {
    // ── configuration stage ────────────────────────────────────────────────
    /// `DATABASE_URL` is required for live mode but missing.
    #[error("DATABASE_URL is required when --fixture is not set (set it via env, .env, or --database-url)")]
    MissingDatabaseUrl,

    /// The project name contains characters that are not valid in a crate
    /// name (`[a-zA-Z][a-zA-Z0-9_-]*`).
    #[error("invalid project name `{name}`: must match [a-zA-Z][a-zA-Z0-9_-]*")]
    InvalidProjectName { name: String },

    // ── introspection stage ────────────────────────────────────────────────
    /// The live introspector could not connect to PostgreSQL.
    #[error("could not connect to PostgreSQL (check DATABASE_URL and network): {source}")]
    DbConnect {
        #[source]
        source: sqlx::Error,
    },

    /// A catalog query failed unexpectedly.
    #[error("catalog query failed ({query}): {source}")]
    CatalogQuery {
        query: &'static str,
        #[source]
        source: sqlx::Error,
    },

    /// A table was skipped because it has no usable primary key
    /// (zero or composite PKs are not supported by generic REST CRUD).
    #[error("table `{table}` skipped: expected exactly one primary-key column, found {found}")]
    NoPrimaryKey { table: String, found: usize },

    /// The schema introspected cleanly but contains no usable tables.
    #[error("schema `{schema}` contains no usable tables (every table needs exactly one primary-key column)")]
    NoUsableTables { schema: String },

    // ── analysis stage ─────────────────────────────────────────────────────
    /// A column type was encountered that cannot be safely mapped.
    ///
    /// Codegen falls back to `String` and emits a warning, so this variant
    /// is reserved for *structural* problems (e.g. arrays of composites).
    #[error("unsupported column type `{udt}` on `{table}.{column}` — mapped to String")]
    UnsupportedType {
        table: String,
        column: String,
        udt: String,
    },

    // ── codegen stage ──────────────────────────────────────────────────────
    /// The output directory already exists and `--force` was not supplied.
    #[error("output directory `{path}` already exists — pass --force to overwrite")]
    OutputExists { path: PathBuf },

    /// A table is missing its primary key at codegen time (defensive —
    /// the introspector should already have skipped such tables).
    #[error("internal: table `{table}` reached codegen without a primary key")]
    CodegenNoPk { table: String },

    /// Template substitution failed (a placeholder was left unreplaced).
    ///
    /// Should never happen in practice; surfaced as a hard error so it
    /// can never silently produce broken Rust.
    #[error("template error in `{file}`: unreplaced placeholder `{placeholder}`")]
    TemplatePlaceholder { file: String, placeholder: String },

    // ── I/O stage ──────────────────────────────────────────────────────────
    /// A filesystem operation failed during project emission.
    #[error("filesystem error at `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    // ── generic escape hatch ───────────────────────────────────────────────
    /// Catch-all for unexpected conditions; carries a context string.
    ///
    /// Use sparingly — prefer adding a specific variant when the same
    /// generic error surfaces more than once.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl GenError {
    /// Returns `true` when this error originated from the database driver.
    ///
    /// Useful for the CLI to decide whether to print a hint about
    /// `DATABASE_URL` / network reachability.
    #[must_use]
    pub fn is_db_error(&self) -> bool {
        matches!(
            self,
            GenError::DbConnect { .. } | GenError::CatalogQuery { .. }
        )
    }

    /// Stable, machine-readable code identifying the defect class.
    ///
    /// The string is short (`GEN-001` … `GEN-011`), stable across releases,
    /// and identical for every occurrence of the same variant. The
    /// `error_inventory.json` writer (see `error_inventory.rs`) records
    /// this code on every entry so CI tooling can aggregate defect rates
    /// by code without parsing free-form messages.
    ///
    /// # CMMI mapping
    ///
    /// * **MA SG 1** — Specify measurement objectives: every defect
    ///   class is a measurable unit.
    /// * **CAR SG 1** — Determine root causes of defects: a stable code
    ///   lets the same root cause be tracked across incidents.
    #[must_use]
    pub fn error_code(&self) -> &'static str {
        match self {
            GenError::MissingDatabaseUrl => "GEN-001",
            GenError::InvalidProjectName { .. } => "GEN-002",
            GenError::DbConnect { .. } => "GEN-003",
            GenError::CatalogQuery { .. } => "GEN-004",
            GenError::NoPrimaryKey { .. } => "GEN-005",
            GenError::NoUsableTables { .. } => "GEN-006",
            GenError::UnsupportedType { .. } => "GEN-007",
            GenError::OutputExists { .. } => "GEN-008",
            GenError::CodegenNoPk { .. } => "GEN-009",
            GenError::TemplatePlaceholder { .. } => "GEN-010",
            GenError::Io { .. } => "GEN-011",
            GenError::Other { .. } => "GEN-099",
        }
    }

    /// CMMI Process Area that owns the defect class, used by
    /// `hex-gen errors stats` to group the inventory for appraisers.
    #[must_use]
    pub fn cmmi_pa(&self) -> &'static str {
        match self {
            GenError::MissingDatabaseUrl => "OPF",
            GenError::InvalidProjectName { .. } => "OPD",
            GenError::DbConnect { .. } => "OPF",
            GenError::CatalogQuery { .. } => "OPF",
            GenError::NoPrimaryKey { .. } => "MA",
            GenError::NoUsableTables { .. } => "MA",
            GenError::UnsupportedType { .. } => "CAR",
            GenError::OutputExists { .. } => "OPD",
            GenError::CodegenNoPk { .. } => "CAR",
            GenError::TemplatePlaceholder { .. } => "CAR",
            GenError::Io { .. } => "MA",
            GenError::Other { .. } => "CAR",
        }
    }
}

/// Hides the password when echoing the connection string.
///
/// `postgresql://user:hunter2@host:5432/db` → `postgresql://user:***@host:5432/db`
///
/// v0.5.0 (closes G-L06): the password is percent-decoded before
/// being searched for the `:` separator, so URL-encoded passwords
/// like `p%40ssw0rd` (which decodes to `p@ssw0rd`) are correctly
/// masked. The previous implementation matched on the literal `%40`
/// sequence and missed the `:` separator entirely, leaving the
/// encoded password visible in log lines.
///
/// Used only for log lines; the real value is forwarded untouched.
///
/// # Errors
///
/// This function never returns an error — it is best-effort. If the
/// URL is malformed (no `://`, no `@`, no `:`), the input is
/// returned unchanged so the log line still ships.
#[must_use]
pub fn mask_password(url: &str) -> String {
    // v0.5.0: percent-decode the whole URL first so encoded
    // characters like `%40` (→ `@`), `%3A` (→ `:`), `%2F` (→ `/`)
    // are normalised before we search for the credential separator.
    // We only decode for the *purpose of finding the password* —
    // the returned string uses the original (encoded) userinfo so
    // the masked form is safe to log even if it still contains
    // encoded characters.
    let decoded = percent_decode(url);
    if let Some(scheme_end) = decoded.find("://") {
        let rest = &decoded[scheme_end + 3..];
        if let Some(at) = rest.rfind('@') {
            let (creds, _host_decoded) = rest.split_at(at);
            if let Some(colon) = creds.find(':') {
                // Map the decoded offset back to the original (encoded)
                // string. We do this by re-walking both strings in
                // parallel — decoded and original have the same number
                // of userinfo chars, just different byte lengths for
                // percent-encoded sequences.
                let original_userinfo = original_userinfo_slice(url, scheme_end, creds);
                if let Some(orig_colon) = find_nth_percent_aware(original_userinfo, colon) {
                    let (_user, _pw) = original_userinfo.split_at(orig_colon);
                    // Reconstruct using the ORIGINAL url slices so
                    // the masked form is safe to log even if it
                    // contains encoded characters.
                    let host_slice = host_slice_original(url, scheme_end);
                    return format!(
                        "{}://{}:***{}",
                        &url[..scheme_end],
                        &original_userinfo[..orig_colon],
                        host_slice
                    );
                }
            }
        }
    }
    // Fallback: mask nothing. Better to ship the URL unmasked than
    // to crash the logger.
    url.to_string()
}

/// Percent-decodes a string. Replaces `%XX` sequences with the
/// corresponding byte. Used internally by [`mask_password`].
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_digit(bytes[i + 1]);
            let lo = hex_digit(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Parses a single hex digit. Returns `None` for non-hex characters.
fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Returns the slice of the original (encoded) URL corresponding to
/// the userinfo segment found in the decoded version.
fn original_userinfo_slice<'a>(url: &'a str, scheme_end: usize, _creds_decoded: &str) -> &'a str {
    // Walk from `scheme_end + 3` to the first `@` in the ORIGINAL
    // url. The decoded creds may have a different length, but the
    // userinfo segment in the original url is always bounded by
    // `://` on the left and `@` on the right.
    let rest = &url[scheme_end + 3..];
    if let Some(at) = rest.find('@') {
        &rest[..at]
    } else {
        ""
    }
}

/// Returns the slice of the original (encoded) URL corresponding to
/// the host+path part (everything after the `@`).
fn host_slice_original<'a>(url: &'a str, scheme_end: usize) -> &'a str {
    let rest = &url[scheme_end + 3..];
    if let Some(at) = rest.find('@') {
        &rest[at..]
    } else {
        ""
    }
}

/// Finds the byte offset in `original` that corresponds to the
/// `decoded_offset`-th byte in the percent-decoded version of
/// `original`. Used to map decoded offsets back to encoded offsets
/// so we can slice the original URL safely.
fn find_nth_percent_aware(original: &str, decoded_offset: usize) -> Option<usize> {
    let bytes = original.as_bytes();
    let mut decoded_pos = 0;
    let mut i = 0;
    while i < bytes.len() {
        if decoded_pos == decoded_offset {
            return Some(i);
        }
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            // A percent-encoded sequence contributes 1 decoded byte.
            decoded_pos += 1;
            i += 3;
        } else {
            decoded_pos += 1;
            i += 1;
        }
    }
    if decoded_pos == decoded_offset {
        Some(i)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_password_plain() {
        assert_eq!(
            mask_password("postgresql://user:hunter2@host:5432/db"),
            "postgresql://user:***@host:5432/db"
        );
    }

    #[test]
    fn mask_password_url_encoded_at_sign() {
        // `p%40ssw0rd` decodes to `p@ssw0rd`. Without the v0.5.0 fix
        // the function would match the encoded `@` inside the password
        // and miss the `:` separator, leaving the password visible.
        let masked = mask_password("postgresql://user:p%40ssw0rd@host:5432/db");
        assert!(
            masked.contains("***"),
            "masked form must contain ***: got {masked}"
        );
        assert!(
            !masked.contains("p%40ssw0rd"),
            "masked form must not contain the encoded password: got {masked}"
        );
        assert!(
            !masked.contains("p@ssw0rd"),
            "masked form must not contain the decoded password: got {masked}"
        );
    }

    #[test]
    fn mask_password_no_password() {
        assert_eq!(
            mask_password("postgresql://user@host:5432/db"),
            "postgresql://user@host:5432/db"
        );
    }

    #[test]
    fn mask_password_no_credentials() {
        assert_eq!(
            mask_password("postgresql://host:5432/db"),
            "postgresql://host:5432/db"
        );
    }

    #[test]
    fn mask_password_not_a_url() {
        assert_eq!(mask_password("not a url at all"), "not a url at all");
    }

    #[test]
    fn percent_decode_basic() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("p%40ssw0rd"), "p@ssw0rd");
        assert_eq!(percent_decode("no-encoding-here"), "no-encoding-here");
    }
}
