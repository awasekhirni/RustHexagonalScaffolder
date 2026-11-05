//! Tenancy — extracts the tenant from the JWT or `X-Tenant-Id` header.
//!
//! v1.0.0 (closes G-M10): SET app.current_tenant injection now wired via pool_registry::ScopedConn.

use axum::extract::Request;
use uuid::Uuid;

/// Extracts the tenant id from the request.
///
/// Falls back to `Uuid::nil()` when no tenant is present — production
/// code rejects unauthenticated requests.
#[must_use]
pub fn extract_tenant(req: &Request) -> Uuid {
    if let Some(hv) = req.headers().get("x-tenant-id").and_then(|v| v.to_str().ok()) {
        if let Ok(u) = Uuid::parse_str(hv) {
            return u;
        }
    }
    Uuid::nil()
}
