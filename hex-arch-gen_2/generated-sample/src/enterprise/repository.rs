//! # Enterprise Repository Pattern
//!
//! One generic, async, storage-agnostic repository contract shared by all
//! bounded contexts. The domain declares a thin port per aggregate that
//! *supertraits* this contract; infrastructure provides SQLx adapters.
//!
//! ```text
//! domain/orders/repository.rs   trait OrdersRepository: Repository<Orders, Uuid>
//!                                            │ implemented by
//!                                            ▼
//! infrastructure/orders_repository.rs   PostgresOrdersRepository (SQLx)
//! ```

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

/// Maps a raw [`sqlx::Error`] into the repository taxonomy, translating
/// common PostgreSQL SQLSTATE codes into semantic variants.
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
    pub fn new(page: i64, per_page: i64) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, 200),
        }
    }

    /// SQL OFFSET for this page.
    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.per_page
    }

    /// SQL LIMIT for this page.
    pub fn limit(&self) -> i64 {
        self.per_page
    }
}

/// Pagination envelope returned by [`Repository::find_all`].
#[derive(Debug, Clone, Serialize)]
pub struct Paginated<T> {
    /// Items of the requested page.
    pub items: Vec<T>,
    /// Total number of rows matching the query.
    pub total: i64,
    /// Current page number.
    pub page: i64,
    /// Requested page size.
    pub per_page: i64,
}

/// The enterprise repository contract.
///
/// `T` is the aggregate/entity type and `ID` its primary-key type
/// (`Uuid`, `i32`, …). Every generated repository port extends this
/// trait, so adapters always provide the six canonical operations.
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{
    /// Loads one entity by primary key.
    async fn find_by_id(&self, id: ID) -> Result<Option<T>, RepositoryError>;
    /// Loads a page of entities together with the total row count.
    async fn find_all(&self, page: Page) -> Result<Paginated<T>, RepositoryError>;
    /// Persists a new entity and returns it (including DB-generated cols).
    async fn create(&self, entity: &T) -> Result<T, RepositoryError>;
    /// Updates an entity; returns `None` when the ID does not exist.
    async fn update(&self, id: ID, entity: &T) -> Result<Option<T>, RepositoryError>;
    /// Hard-deletes an entity; returns whether a row was affected.
    async fn delete(&self, id: ID) -> Result<bool, RepositoryError>;
    /// Total number of rows.
    async fn count(&self) -> Result<i64, RepositoryError>;
}
