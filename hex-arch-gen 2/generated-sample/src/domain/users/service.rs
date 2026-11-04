//! Inbound port — application service contract for `users`.
//!
//! HTTP handlers reach this use-case through the enterprise bus; nothing
//! above this trait knows about SQLx or PostgreSQL.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::users::entity::Users;
use crate::domain::users::input::{CreateUsersInput, UpdateUsersInput};
use crate::enterprise::repository::{Page, Paginated, RepositoryError};
use crate::domain::users::enums::UserStatusEnum;

/// Application service contract for `users`.
#[async_trait]
pub trait UsersService: Send + Sync {
    /// Fetches one record by primary key.
    async fn get(&self, id: Uuid) -> Result<Option<Users>, RepositoryError>;
    /// Lists records page by page.
    async fn list(&self, page: Page) -> Result<Paginated<Users>, RepositoryError>;
    /// Validates and persists a new record.
    async fn create(&self, input: CreateUsersInput) -> Result<Users, RepositoryError>;
    /// Applies a partial update; `None` when the record is missing.
    async fn update(&self, id: Uuid, input: UpdateUsersInput) -> Result<Option<Users>, RepositoryError>;
    /// Deletes a record (hard delete when soft delete is unavailable).
    async fn delete(&self, id: Uuid) -> Result<bool, RepositoryError>;
    /// Lists records by status.
    async fn list_by_status(&self, status: UserStatusEnum, page: Page) -> Result<Vec<Users>, RepositoryError>;
    /// Changes the status of one record.
    async fn set_status(&self, id: Uuid, status: UserStatusEnum) -> Result<Option<Users>, RepositoryError>;
    /// Lists records created at or after `ts`.
    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, RepositoryError>;
    /// Lists records updated at or after `ts`.
    async fn list_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, RepositoryError>;
}
