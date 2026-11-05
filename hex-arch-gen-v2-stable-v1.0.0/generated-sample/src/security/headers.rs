//! Security headers middleware — adds the OWASP-recommended header set to
//! every response.

use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use axum::extract::Request;

/// Adds security headers to every response.
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("x-xss-protection", HeaderValue::from_static("1; mode=block"));
    headers.insert("strict-transport-security", HeaderValue::from_static("max-age=31536000; includeSubDomains"));
    headers.insert("referrer-policy", HeaderValue::from_static("strict-origin-when-cross-origin"));
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'"));
    resp
}
