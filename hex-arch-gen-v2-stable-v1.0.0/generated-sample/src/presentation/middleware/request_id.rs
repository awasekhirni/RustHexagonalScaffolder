//! Request-id middleware — propagates or generates a request id.

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

/// Propagates `X-Request-Id` (or generates one) onto the response.
pub async fn request_id(req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut resp = next.run(req).await;
    if let Ok(hv) = HeaderValue::from_str(&id) {
        resp.headers_mut().insert("x-request-id", hv);
    }
    resp
}
