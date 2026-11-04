//! Query/body parameter types for the `product_categories` HTTP surface.

use serde::Deserialize;
use utoipa::ToSchema;
use chrono::{DateTime, Utc};
use crate::enterprise::repository::Page;

/// Pagination query parameters for `product_categories` endpoints.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ProductCategoriesPageParams {
    /// 1-based page number (default 1).
    pub page: Option<i64>,
    /// Page size, clamped to 1..=200 (default 20).
    pub per_page: Option<i64>,
}

impl ProductCategoriesPageParams {
    /// Converts loose query parameters into an enterprise [`Page`].
    pub fn to_page(&self) -> Page {
        Page::new(self.page.unwrap_or(1), self.per_page.unwrap_or(20))
    }
}

/// Body of `PATCH /product_categories/{id}/status`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetProductCategoriesStatusInput {
    /// New status value.
    pub status: String,
}

/// Query parameters for `GET /product_categories/created-after`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ProductCategoriesCreatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}
