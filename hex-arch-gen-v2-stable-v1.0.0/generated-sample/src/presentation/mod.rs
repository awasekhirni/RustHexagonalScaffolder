//! Presentation layer — axum handlers, routes, middleware, state.
//!
//! Handlers are thin inbound adapters: they deserialize transport types,
//! call the application service, and map errors onto HTTP status codes.
//! No business logic lives here.

pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

use axum::middleware::from_fn;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::presentation::middleware::security_headers;
use crate::presentation::state::AppState;

/// Builds the full router (called from `main`).
#[must_use]
pub fn build_router(state: AppState) -> Router {
    let api = crate::presentation::routes::router();
    Router::new()
        .nest("/api/v1", api)
        .route("/health", axum::routing::get(health))
        .layer(from_fn(security_headers))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// `GET /health` — liveness probe.
async fn health() -> &'static str {
    "ok"
}
