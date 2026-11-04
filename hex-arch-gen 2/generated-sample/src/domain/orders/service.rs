//! Inbound port — application service contract for `orders`.
//!
//! HTTP handlers reach this use-case through the enterprise bus; nothing
//! above this trait knows about SQLx or PostgreSQL.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::orders::entity::Orders;
use crate::domain::orders::input::{CreateOrdersInput, UpdateOrdersInput};
use crate::enterprise::repository::{Page, Paginated, RepositoryError};
use crate::domain::orders::enums::OrderStatusEnum;

/// Application service contract for `orders`.
#[async_trait]
pub trait OrdersService: Send + Sync {
    /// Fetches one record by primary key.
    async fn get(&self, id: Uuid) -> Result<Option<Orders>, RepositoryError>;
    /// Lists records page by page.
    async fn list(&self, page: Page) -> Result<Paginated<Orders>, RepositoryError>;
    /// Validates and persists a new record.
    async fn create(&self, input: CreateOrdersInput) -> Result<Orders, RepositoryError>;
    /// Applies a partial update; `None` when the record is missing.
    async fn update(&self, id: Uuid, input: UpdateOrdersInput) -> Result<Option<Orders>, RepositoryError>;
    /// Deletes a record (hard delete when soft delete is unavailable).
    async fn delete(&self, id: Uuid) -> Result<bool, RepositoryError>;
    /// Lists records by status.
    async fn list_by_status(&self, status: OrderStatusEnum, page: Page) -> Result<Vec<Orders>, RepositoryError>;
    /// Changes the status of one record.
    async fn set_status(&self, id: Uuid, status: OrderStatusEnum) -> Result<Option<Orders>, RepositoryError>;
    /// Soft-deletes a record.
    async fn soft_delete(&self, id: Uuid) -> Result<bool, RepositoryError>;
    /// Restores a soft-deleted record.
    async fn restore(&self, id: Uuid) -> Result<Option<Orders>, RepositoryError>;
    /// Lists records belonging to a user.
    async fn list_by_user(&self, user_id: Uuid, page: Page) -> Result<Vec<Orders>, RepositoryError>;
    /// Lists records created at or after `ts`.
    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, RepositoryError>;
    /// Lists records updated at or after `ts`.
    async fn list_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, RepositoryError>;
}
