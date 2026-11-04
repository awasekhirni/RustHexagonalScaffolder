//! Outbound port — repository contract for `orders`.
//!
//! Owned by the domain layer, implemented by `infrastructure` with SQLx.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::orders::entity::Orders;
use crate::enterprise::repository::{Page, Repository, RepositoryError};
use crate::domain::orders::enums::OrderStatusEnum;

/// Repository port for `orders` — extends the enterprise base repository
/// with the context-specific look-ups detected in the schema.
#[async_trait]
pub trait OrdersRepository: Repository<Orders, Uuid> {
    /// Lists records sharing a status value.
    async fn find_by_status(&self, status: OrderStatusEnum, page: Page) -> Result<Vec<Orders>, RepositoryError>;
    /// Updates the status column of one record.
    async fn set_status(&self, id: Uuid, status: OrderStatusEnum) -> Result<Option<Orders>, RepositoryError>;
    /// Tombstones the record (soft delete) instead of removing it.
    async fn soft_delete(&self, id: Uuid) -> Result<bool, RepositoryError>;
    /// Restores a soft-deleted record and returns it.
    async fn restore(&self, id: Uuid) -> Result<Option<Orders>, RepositoryError>;
    /// Lists records belonging to a user.
    async fn find_by_user(&self, user_id: Uuid, page: Page) -> Result<Vec<Orders>, RepositoryError>;
    /// Lists records created at or after `ts`.
    async fn find_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, RepositoryError>;
    /// Lists records updated at or after `ts`.
    async fn find_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, RepositoryError>;
}
