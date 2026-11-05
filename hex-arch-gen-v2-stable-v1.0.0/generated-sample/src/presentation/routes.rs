//! HTTP route table for the generated API.
//!
//! State is attached in `main` (after Swagger UI is merged), so this
//! function stays a pure description of the `/api/v1` surface.

use axum::Router;

use crate::presentation::state::AppState;

use crate::presentation::handlers::orders_handler as orders;
use crate::presentation::handlers::product_categories_handler as product_categories;
use crate::presentation::handlers::users_handler as users;

/// Builds the `/api/v1` routes.
#[must_use]
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", axum::routing::get(crate::presentation::handlers::health))
        .route("/orders", axum::routing::get(orders::list).post(orders::create))
        .route("/orders/{id}", axum::routing::get(orders::get_one).put(orders::update).delete(orders::delete))
        .route("/orders/status/{status}", axum::routing::get(orders::list_by_status))
        .route("/orders/{id}/status", axum::routing::patch(orders::set_status))
        .route("/orders/{id}/soft", axum::routing::delete(orders::soft_delete))
        .route("/orders/{id}/restore", axum::routing::post(orders::restore))
        .route("/orders/by-user/{user_id}", axum::routing::get(orders::list_by_user))
        .route("/orders/created-after", axum::routing::get(orders::created_after))
        .route("/orders/updated-after", axum::routing::get(orders::updated_after))
        .route("/product_categories", axum::routing::get(product_categories::list).post(product_categories::create))
        .route("/product_categories/{id}", axum::routing::get(product_categories::get_one).put(product_categories::update).delete(product_categories::delete))
        .route("/product_categories/status/{status}", axum::routing::get(product_categories::list_by_status))
        .route("/product_categories/{id}/status", axum::routing::patch(product_categories::set_status))
        .route("/product_categories/created-after", axum::routing::get(product_categories::created_after))
        .route("/users", axum::routing::get(users::list).post(users::create))
        .route("/users/{id}", axum::routing::get(users::get_one).put(users::update).delete(users::delete))
        .route("/users/status/{status}", axum::routing::get(users::list_by_status))
        .route("/users/{id}/status", axum::routing::patch(users::set_status))
        .route("/users/{id}/soft", axum::routing::delete(users::soft_delete))
        .route("/users/{id}/restore", axum::routing::post(users::restore))
        .route("/users/created-after", axum::routing::get(users::created_after))
        .route("/users/updated-after", axum::routing::get(users::updated_after))
}
