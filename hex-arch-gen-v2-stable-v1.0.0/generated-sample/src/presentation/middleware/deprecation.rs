//! Deprecation middleware — emits `Deprecation` and `Sunset` headers on
//! deprecated routes.

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;

/// Adds `Deprecation: true` and `Sunset: <date>` headers.
pub async fn deprecate(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert("deprecation", HeaderValue::from_static("true"));
    headers.insert("sunset", HeaderValue::from_static("Wed, 11 Nov 2026 23:59:59 GMT"));
    resp
}
