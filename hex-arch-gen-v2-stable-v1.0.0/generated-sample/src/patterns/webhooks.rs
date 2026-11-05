//! Outbound webhook signer — HMAC-SHA256 over the canonical payload.
//!
//! Verification on the receiver side compares the `X-Signature-256`
//! header against `sha256=<hex>`.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Computes the `sha256=<hex>` signature for the given payload + secret.
#[must_use]
pub fn sign(secret: &str, payload: &[u8]) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(payload);
    let hex = hex::encode(mac.finalize().into_bytes());
    format!("sha256={hex}")
}
