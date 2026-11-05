//! Per-context HTTP handlers — one module per bounded context.

pub mod orders_handler;
pub mod product_categories_handler;
pub mod users_handler;

/// `GET /api/v1/health` — readiness probe inside the API surface.
pub async fn health() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({"status": "ok"}))
}
