//! Repository ports — outbound contracts owned by the application layer.
//!
//! The base `Repository<T, ID>` trait declares the six canonical CRUD
//! operations; each per-table trait extends it with context-specific
//! look-ups detected in the schema.

use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;

/// Error taxonomy shared by every repository adapter.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// The requested entity does not exist.
    #[error("entity not found")]
    NotFound,
    /// Input failed application-level validation.
    #[error("validation failed: {0}")]
    Validation(String),
    /// Database constraint was violated (unique / FK / not-null).
    #[error("constraint violation: {0}")]
    Constraint(String),
    /// Anything else raised by the driver.
    #[error("database error: {0}")]
    Database(String),
}

/// Maps a raw [`sqlx::Error`] into the repository taxonomy.
pub fn map_db_error(error: sqlx::Error) -> RepositoryError {
    use sqlx::Error;
    match error {
        Error::RowNotFound => RepositoryError::NotFound,
        Error::Database(ref db) => match db.code().as_deref() {
            Some("23505") | Some("23503") | Some("23502") => {
                RepositoryError::Constraint(db.message().to_string())
            }
            _ => RepositoryError::Database(error.to_string()),
        },
        other => RepositoryError::Database(other.to_string()),
    }
}

/// Pagination request. `page` is 1-based; `per_page` is clamped to 1..=200.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    /// 1-based page number.
    pub page: i64,
    /// Number of items per page.
    pub per_page: i64,
}

impl Page {
    /// Builds a clamped page from raw query parameters.
    #[must_use]
    pub fn new(page: i64, per_page: i64) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, 200),
        }
    }
    /// SQL OFFSET for this page.
    #[must_use]
    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.per_page
    }
    /// SQL LIMIT for this page.
    #[must_use]
    pub fn limit(&self) -> i64 {
        self.per_page
    }
}

/// Pagination envelope returned by [`Repository::find_all`].
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl From<RepositoryError> for crate::shared::error::AppError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::NotFound => crate::shared::error::AppError::NotFound("entity not found".into()),
            RepositoryError::Validation(m) => crate::shared::error::AppError::Validation(m),
            RepositoryError::Constraint(m) => crate::shared::error::AppError::Conflict(m),
            RepositoryError::Database(m) => crate::shared::error::AppError::Internal(m),
        }
    }
}

/// The enterprise repository contract.
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{
    async fn find_by_id(&self, id: ID) -> Result<Option<T>, RepositoryError>;
    async fn find_all(&self, page: Page) -> Result<Paginated<T>, RepositoryError>;
    async fn create(&self, entity: &T) -> Result<T, RepositoryError>;
    async fn update(&self, id: ID, entity: &T) -> Result<Option<T>, RepositoryError>;
    async fn delete(&self, id: ID) -> Result<bool, RepositoryError>;
    async fn count(&self) -> Result<i64, RepositoryError>;
}

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::entities::orders::Orders;
use crate::domain::entities::orders::OrderStatusEnum;
use crate::domain::entities::product_categories::ProductCategories;
use crate::domain::entities::users::Users;
use crate::domain::entities::users::UserStatusEnum;


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

/// Application service contract for `orders`.
#[async_trait]
pub trait OrdersServicePort: Send + Sync {
    async fn create(&self, input: crate::application::dto::orders_dto::CreateOrdersInput) -> Result<Orders, crate::shared::error::AppError>;
    async fn get(&self, id: Uuid) -> Result<Option<Orders>, crate::shared::error::AppError>;
    async fn list(&self, page: Page) -> Result<Paginated<Orders>, crate::shared::error::AppError>;
    async fn update(&self, id: Uuid, input: crate::application::dto::orders_dto::UpdateOrdersInput) -> Result<Option<Orders>, crate::shared::error::AppError>;
    async fn delete(&self, id: Uuid) -> Result<bool, crate::shared::error::AppError>;
    async fn list_by_status(&self, status: OrderStatusEnum, page: Page) -> Result<Vec<Orders>, crate::shared::error::AppError>;
    async fn set_status(&self, id: Uuid, status: OrderStatusEnum) -> Result<Option<Orders>, crate::shared::error::AppError>;
    async fn soft_delete(&self, id: Uuid) -> Result<bool, crate::shared::error::AppError>;
    async fn restore(&self, id: Uuid) -> Result<Option<Orders>, crate::shared::error::AppError>;
    async fn list_by_user(&self, user_id: Uuid, page: Page) -> Result<Vec<Orders>, crate::shared::error::AppError>;
    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, crate::shared::error::AppError>;
    async fn list_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, crate::shared::error::AppError>;
}


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

/// Application service contract for `product_categories`.
#[async_trait]
pub trait ProductCategoriesServicePort: Send + Sync {
    async fn create(&self, input: crate::application::dto::product_categories_dto::CreateProductCategoriesInput) -> Result<ProductCategories, crate::shared::error::AppError>;
    async fn get(&self, id: i32) -> Result<Option<ProductCategories>, crate::shared::error::AppError>;
    async fn list(&self, page: Page) -> Result<Paginated<ProductCategories>, crate::shared::error::AppError>;
    async fn update(&self, id: i32, input: crate::application::dto::product_categories_dto::UpdateProductCategoriesInput) -> Result<Option<ProductCategories>, crate::shared::error::AppError>;
    async fn delete(&self, id: i32) -> Result<bool, crate::shared::error::AppError>;
    async fn list_by_status(&self, status: String, page: Page) -> Result<Vec<ProductCategories>, crate::shared::error::AppError>;
    async fn set_status(&self, id: i32, status: String) -> Result<Option<ProductCategories>, crate::shared::error::AppError>;
    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<ProductCategories>, crate::shared::error::AppError>;
}


/// Repository port for `users` — extends the enterprise base repository
/// with the context-specific look-ups detected in the schema.
#[async_trait]
pub trait UsersRepository: Repository<Users, Uuid> {
    /// Lists records sharing a status value.
    async fn find_by_status(&self, status: UserStatusEnum, page: Page) -> Result<Vec<Users>, RepositoryError>;
    /// Updates the status column of one record.
    async fn set_status(&self, id: Uuid, status: UserStatusEnum) -> Result<Option<Users>, RepositoryError>;
    /// Tombstones the record (soft delete) instead of removing it.
    async fn soft_delete(&self, id: Uuid) -> Result<bool, RepositoryError>;
    /// Restores a soft-deleted record and returns it.
    async fn restore(&self, id: Uuid) -> Result<Option<Users>, RepositoryError>;
    /// Lists records created at or after `ts`.
    async fn find_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, RepositoryError>;
    /// Lists records updated at or after `ts`.
    async fn find_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, RepositoryError>;
}

/// Application service contract for `users`.
#[async_trait]
pub trait UsersServicePort: Send + Sync {
    async fn create(&self, input: crate::application::dto::users_dto::CreateUsersInput) -> Result<Users, crate::shared::error::AppError>;
    async fn get(&self, id: Uuid) -> Result<Option<Users>, crate::shared::error::AppError>;
    async fn list(&self, page: Page) -> Result<Paginated<Users>, crate::shared::error::AppError>;
    async fn update(&self, id: Uuid, input: crate::application::dto::users_dto::UpdateUsersInput) -> Result<Option<Users>, crate::shared::error::AppError>;
    async fn delete(&self, id: Uuid) -> Result<bool, crate::shared::error::AppError>;
    async fn list_by_status(&self, status: UserStatusEnum, page: Page) -> Result<Vec<Users>, crate::shared::error::AppError>;
    async fn set_status(&self, id: Uuid, status: UserStatusEnum) -> Result<Option<Users>, crate::shared::error::AppError>;
    async fn soft_delete(&self, id: Uuid) -> Result<bool, crate::shared::error::AppError>;
    async fn restore(&self, id: Uuid) -> Result<Option<Users>, crate::shared::error::AppError>;
    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, crate::shared::error::AppError>;
    async fn list_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, crate::shared::error::AppError>;
}

