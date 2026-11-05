//! Idempotency — checksums a request payload so duplicate POSTs return the
//! original response instead of creating duplicates.
//!
//! v1.0.0 (closes G-M09): InMemoryIdempotencyStore adapter now bound; Redis adapter lands in v1.1.

use sha2::{Digest, Sha256};

/// Computes a deterministic SHA-256 checksum of the serialised payload.
#[must_use]
pub fn checksum(payload: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    // serde_json::to_string is canonical only for ordered keys; we sort
    // by serialising with sorted keys via the `preserve_order` feature off.
    let canonical = serde_json::to_string(payload).unwrap_or_default();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

/// Idempotency key — typically a client-supplied UUID; falls back to the
/// payload checksum when the client omits it.
#[must_use]
pub fn resolve_key(client_key: Option<&str>, payload: &serde_json::Value) -> String {
    client_key
        .map(str::to_string)
        .unwrap_or_else(|| checksum(payload))
}
