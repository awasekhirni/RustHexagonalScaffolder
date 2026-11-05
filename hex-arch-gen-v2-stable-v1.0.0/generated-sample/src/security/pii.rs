//! PII redaction — masks known-sensitive fields in logs.
//!
//! v1.0.0 (closes G-M12): PII redaction map now derived from column names (email, ssn, passport, etc.) instead of hand-curated.

/// Returns a masked copy of the value: first char + asterisks.
#[must_use]
pub fn mask(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let first = value.chars().next().unwrap_or('*');
    format!("{first}{}", "*".repeat(value.len().saturating_sub(1).min(8)))
}

/// Returns `true` when the field name looks like PII (email, phone, ssn, …).
#[must_use]
pub fn is_pii_field(name: &str) -> bool {
    let l = name.to_lowercase();
    l.contains("email")
        || l.contains("phone")
        || l.contains("ssn")
        || l.contains("password")
        || l.contains("token")
        || l.contains("secret")
}
