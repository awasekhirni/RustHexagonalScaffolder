//! Shared application state handed to every Axum handler.
//!
//! Each handler pulls the service it needs from this struct; nothing in
//! the presentation layer constructs services directly.
//!
//! v0.4.0 (closes G-H01, G-H02): added `jwt` and `rbac` fields so
//! every handler can verify tokens and enforce per-table authorization
//! without re-constructing the services on each request.

use std::sync::Arc;

use sqlx::PgPool;

use crate::application::ports::bus::EventBus;
use crate::core::shutdown::InFlightTracker;
use crate::security::auth::jwt::JwtService;
use crate::security::auth::rbac::RbacPolicy;

/// Dependencies shared across the whole HTTP surface.
#[derive(Clone)]
pub struct AppState {
    /// Shared PostgreSQL pool — used by handlers that need direct SQL
    /// access (audit endpoints, restore shortcuts, …).
    pub pool: PgPool,
    /// `orders` application service.
    pub orders_service: std::sync::Arc<dyn crate::application::ports::repository::OrdersServicePort>,
    /// `product_categories` application service.
    pub product_categories_service: std::sync::Arc<dyn crate::application::ports::repository::ProductCategoriesServicePort>,
    /// `users` application service.
    pub users_service: std::sync::Arc<dyn crate::application::ports::repository::UsersServicePort>,

    /// Event bus (in-process).
    pub bus: Arc<dyn EventBus>,
    /// In-flight tracker for graceful shutdown drain.
    pub inflight: Arc<InFlightTracker>,
    /// JWT service — used by the `ReqAuth` extractor to verify
    /// `Authorization: Bearer <token>` headers.
    pub jwt: Arc<JwtService>,
    /// RBAC policy — used by handlers via the `require!` macro to
    /// enforce per-table authorization.
    pub rbac: Arc<RbacPolicy>,
}
