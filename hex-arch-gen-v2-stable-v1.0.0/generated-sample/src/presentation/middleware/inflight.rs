//! In-flight tracker middleware — increments/decrements the shared counter.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::core::shutdown::InFlightTracker;

/// Stateful middleware — needs the tracker in extensions.
pub async fn inflight(req: Request, next: Next) -> Response {
    let tracker = req
        .extensions()
        .get::<std::sync::Arc<InFlightTracker>>()
        .cloned();
    if let Some(t) = &tracker {
        t.inc();
    }
    let resp = next.run(req).await;
    if let Some(t) = &tracker {
        t.dec();
    }
    resp
}
