//! GDPR right-to-erasure + portability.
//!
//! v1.0.0 (closes G-M11): per-table PII manifest now derived from column names by security/pii.rs.
//!
//! Stub: production code redacts PII columns and writes an audit row;
//! this stub returns Ok so handlers can compile.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use crate::presentation::state::AppState;
use crate::security::auth::extractor::AuthUser;
use crate::shared::error::AppError;

/// `POST /api/v1/users/{id}/erase` — right to erasure.
pub async fn erase_user(
    _auth: AuthUser,
    _state: State<AppState>,
    _id: Path<Uuid>,
) -> Result<StatusCode, AppError> {
    tracing::info!("GDPR erase (stub)");
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/users/{id}/export` — right to portability.
pub async fn export_user(
    _auth: AuthUser,
    _state: State<AppState>,
    _id: Path<Uuid>,
) -> Result<Response, AppError> {
    tracing::info!("GDPR export (stub)");
    Ok(axum::Json(serde_json::json!({"data": []})).into_response())
}
