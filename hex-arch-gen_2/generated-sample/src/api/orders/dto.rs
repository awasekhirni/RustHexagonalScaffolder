//! Query/body parameter types for the `orders` HTTP surface.

use serde::Deserialize;
use utoipa::ToSchema;
use chrono::{DateTime, Utc};
use crate::domain::orders::enums::OrderStatusEnum;
use crate::enterprise::repository::Page;

/// Pagination query parameters for `orders` endpoints.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OrdersPageParams {
    /// 1-based page number (default 1).
    pub page: Option<i64>,
    /// Page size, clamped to 1..=200 (default 20).
    pub per_page: Option<i64>,
}

impl OrdersPageParams {
    /// Converts loose query parameters into an enterprise [`Page`].
    pub fn to_page(&self) -> Page {
        Page::new(self.page.unwrap_or(1), self.per_page.unwrap_or(20))
    }
}

/// Body of `PATCH /orders/{id}/status`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetOrdersStatusInput {
    /// New status value.
    pub status: OrderStatusEnum,
}

/// Query parameters for `GET /orders/created-after`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OrdersCreatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}

/// Query parameters for `GET /orders/updated-after`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OrdersUpdatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}
