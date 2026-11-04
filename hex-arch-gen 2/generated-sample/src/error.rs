//! HTTP error taxonomy — translates domain and bus errors onto status
//! codes with a stable JSON body: `{"error": "…"}`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::enterprise::bus::BusError;
use crate::enterprise::repository::RepositoryError;

/// Errors surfaced by the generated HTTP handlers.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// 404 — the resource does not exist.
    #[error("resource not found")]
    NotFound,
    /// 400 — the request payload or parameter is invalid.
    #[error("{0}")]
    BadRequest(String),
    /// 409 — unique/FK constraint violation.
    #[error("{0}")]
    Conflict(String),
    /// 500 — unexpected internal failure.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<RepositoryError> for ApiError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::NotFound => ApiError::NotFound,
            RepositoryError::Validation(message) => ApiError::BadRequest(message),
            RepositoryError::Constraint(message) => ApiError::Conflict(message),
            RepositoryError::Database(message) => ApiError::Internal(message),
        }
    }
}

impl From<BusError> for ApiError {
    fn from(error: BusError) -> Self {
        match error {
            BusError::Repo(inner) => inner.into(),
            other => ApiError::Internal(other.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}
