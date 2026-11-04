//! Outbound port — repository contract for `product_categories`.
//!
//! Owned by the domain layer, implemented by `infrastructure` with SQLx.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::domain::product_categories::entity::ProductCategories;
use crate::enterprise::repository::{Page, Repository, RepositoryError};

/// Repository port for `product_categories` — extends the enterprise base repository
/// with the context-specific look-ups detected in the schema.
#[async_trait]
pub trait ProductCategoriesRepository: Repository<ProductCategories, i32> {
    /// Lists records sharing a status value.
    async fn find_by_status(&self, status: String, page: Page) -> Result<Vec<ProductCategories>, RepositoryError>;
    /// Updates the status column of one record.
    async fn set_status(&self, id: i32, status: String) -> Result<Option<ProductCategories>, RepositoryError>;
    /// Lists records created at or after `ts`.
    async fn find_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<ProductCategories>, RepositoryError>;
}
