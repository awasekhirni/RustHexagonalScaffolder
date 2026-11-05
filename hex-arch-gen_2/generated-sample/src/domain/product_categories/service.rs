//! Inbound port — application service contract for `product_categories`.
//!
//! HTTP handlers reach this use-case through the enterprise bus; nothing
//! above this trait knows about SQLx or PostgreSQL.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::domain::product_categories::entity::ProductCategories;
use crate::domain::product_categories::input::{CreateProductCategoriesInput, UpdateProductCategoriesInput};
use crate::enterprise::repository::{Page, Paginated, RepositoryError};

/// Application service contract for `product_categories`.
#[async_trait]
pub trait ProductCategoriesService: Send + Sync {
    /// Fetches one record by primary key.
    async fn get(&self, id: i32) -> Result<Option<ProductCategories>, RepositoryError>;
    /// Lists records page by page.
    async fn list(&self, page: Page) -> Result<Paginated<ProductCategories>, RepositoryError>;
    /// Validates and persists a new record.
    async fn create(&self, input: CreateProductCategoriesInput) -> Result<ProductCategories, RepositoryError>;
    /// Applies a partial update; `None` when the record is missing.
    async fn update(&self, id: i32, input: UpdateProductCategoriesInput) -> Result<Option<ProductCategories>, RepositoryError>;
    /// Deletes a record (hard delete when soft delete is unavailable).
    async fn delete(&self, id: i32) -> Result<bool, RepositoryError>;
    /// Lists records by status.
    async fn list_by_status(&self, status: String, page: Page) -> Result<Vec<ProductCategories>, RepositoryError>;
    /// Changes the status of one record.
    async fn set_status(&self, id: i32, status: String) -> Result<Option<ProductCategories>, RepositoryError>;
    /// Lists records created at or after `ts`.
    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<ProductCategories>, RepositoryError>;
}
