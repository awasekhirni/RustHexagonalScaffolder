//! HTTP logging middleware — logs every request with method, path, status, duration.

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::info;

/// Logs every request.
pub async fn logging(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();
    let resp = next.run(req).await;
    let status = resp.status();
    info!(
        %method,
        %path,
        status = status.as_u16(),
        elapsed_ms = start.elapsed().as_millis(),
        "request"
    );
    resp
}
