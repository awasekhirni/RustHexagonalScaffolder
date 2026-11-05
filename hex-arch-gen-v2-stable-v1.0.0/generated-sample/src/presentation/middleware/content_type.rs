//! Content-type guard — rejects requests with non-JSON bodies on write
//! methods (POST/PUT/PATCH).

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use crate::shared::error::AppError;

/// Rejects POST/PUT/PATCH requests whose `Content-Type` is not JSON.
pub async fn require_json(req: Request, next: Next) -> Result<Response, AppError> {
    let method = req.method().clone();
    if matches!(
        method,
        axum::http::Method::POST | axum::http::Method::PUT | axum::http::Method::PATCH
    ) {
        let ct = req
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !ct.contains("application/json") {
            return Err(AppError::BadRequest(
                "Content-Type must be application/json".into(),
            ));
        }
    }
    Ok(next.run(req).await)
}
