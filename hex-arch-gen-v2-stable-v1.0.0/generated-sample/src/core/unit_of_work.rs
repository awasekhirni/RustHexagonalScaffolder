//! `UnitOfWork` — atomic persistence boundary.
//!
//! The v0.6.0 desired-state implements this with a `PgTransaction`; here
//! we expose a thin trait that the application can use without depending
//! on SQLx directly.

use async_trait::async_trait;
use std::sync::Arc;

use crate::shared::error::AppError;

/// Unit of Work — owns a transaction, commits on success, rolls back on error.
#[async_trait]
pub trait UnitOfWork: Send + Sync {
    /// Commits the unit of work.
    async fn commit(self: Box<Self>) -> Result<(), AppError>;
    /// Rolls the unit of work back.
    async fn rollback(self: Box<Self>) -> Result<(), AppError>;
}

/// No-op UoW — used when the caller does not need a transaction.
pub struct NoopUnitOfWork;

#[async_trait]
impl UnitOfWork for NoopUnitOfWork {
    async fn commit(self: Box<Self>) -> Result<(), AppError> {
        Ok(())
    }
    async fn rollback(self: Box<Self>) -> Result<(), AppError> {
        Ok(())
    }
}

/// Shared UoW factory.
pub type SharedUowFactory = Arc<dyn UowFactory + Send + Sync>;

/// Factory that produces new units of work on demand.
#[async_trait]
pub trait UowFactory {
    /// Begins a new unit of work.
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, AppError>;
}
