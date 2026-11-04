//! Outbound port — repository contract for `users`.
//!
//! Owned by the domain layer, implemented by `infrastructure` with SQLx.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::users::entity::Users;
use crate::enterprise::repository::{Page, Repository, RepositoryError};
use crate::domain::users::enums::UserStatusEnum;

/// Repository port for `users` — extends the enterprise base repository
/// with the context-specific look-ups detected in the schema.
#[async_trait]
pub trait UsersRepository: Repository<Users, Uuid> {
    /// Lists records sharing a status value.
    async fn find_by_status(&self, status: UserStatusEnum, page: Page) -> Result<Vec<Users>, RepositoryError>;
    /// Updates the status column of one record.
    async fn set_status(&self, id: Uuid, status: UserStatusEnum) -> Result<Option<Users>, RepositoryError>;
    /// Lists records created at or after `ts`.
    async fn find_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, RepositoryError>;
    /// Lists records updated at or after `ts`.
    async fn find_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, RepositoryError>;
}
