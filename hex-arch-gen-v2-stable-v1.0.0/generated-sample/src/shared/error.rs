//! HTTP error taxonomy — translates domain and bus errors onto status
//! codes with a stable JSON body: `{"error": "…", "code": "…"}`.
//!
//! Every variant carries a stable string code so clients can branch on it
//! without parsing the human-readable message.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use sqlx::Error as SqlxError;

/// Errors surfaced by the generated HTTP handlers.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 400 — the request payload or parameter is invalid.
    #[error("{0}")]
    BadRequest(String),
    /// 401 — authentication is required or failed.
    #[error("unauthorized")]
    Unauthorized,
    /// 403 — the authenticated principal lacks the required role.
    #[error("forbidden")]
    Forbidden,
    /// 404 — the resource does not exist.
    #[error("{0}")]
    NotFound(String),
    /// 409 — unique/FK constraint violation or version conflict.
    #[error("{0}")]
    Conflict(String),
    /// 412 — ETag mismatch (optimistic concurrency).
    #[error("etag mismatch (stale version)")]
    PreconditionFailed,
    /// 422 — semantic validation failure (distinct from 400 malformed input).
    #[error("validation failed: {0}")]
    Validation(String),
    /// 429 — rate limit exceeded.
    #[error("rate limit exceeded")]
    RateLimited,
    /// 500 — unexpected internal failure.
    #[error("internal error: {0}")]
    Internal(String),
    /// 503 — the service is overloaded (load shed).
    #[error("service overloaded")]
    Overloaded,
}

impl AppError {
    /// Stable string code embedded in the JSON error body — clients can
    /// branch on it without parsing the human-readable message.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "bad_request",
            AppError::Unauthorized => "unauthorized",
            AppError::Forbidden => "forbidden",
            AppError::NotFound(_) => "not_found",
            AppError::Conflict(_) => "conflict",
            AppError::PreconditionFailed => "precondition_failed",
            AppError::Validation(_) => "validation_failed",
            AppError::RateLimited => "rate_limited",
            AppError::Internal(_) => "internal_error",
            AppError::Overloaded => "overloaded",
        }
    }
}

impl From<SqlxError> for AppError {
    fn from(error: SqlxError) -> Self {
        use sqlx::Error;
        match error {
            Error::RowNotFound => AppError::NotFound("row not found".into()),
            Error::Database(ref db) => match db.code().as_deref() {
                Some("23505") => AppError::Conflict("unique constraint violation".into()),
                Some("23503") => AppError::Conflict("foreign key violation".into()),
                Some("23502") => AppError::Validation("not-null constraint violation".into()),
                Some("23514") => AppError::Validation("check constraint violation".into()),
                _ => AppError::Internal(db.message().to_string()),
            },
            other => AppError::Internal(other.to_string()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::BadRequest(_) | AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::PreconditionFailed => StatusCode::PRECONDITION_FAILED,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Overloaded => StatusCode::SERVICE_UNAVAILABLE,
        };
        let body = json!({
            "error": self.to_string(),
            "code": self.code(),
        });
        (status, Json(body)).into_response()
    }
}
