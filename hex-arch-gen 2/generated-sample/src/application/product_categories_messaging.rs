//! Enterprise-bus wiring for `product_categories` — commands (write side), queries
//! (read side) and the handler bundle that executes them.
//!
//! One handler struct implements [`CommandHandler`]/[`QueryHandler`] for
//! many message types; the bus erases each registration behind a
//! `TypeId`-keyed adapter, so HTTP handlers never see services directly.

use std::sync::Arc;

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use crate::domain::product_categories::entity::ProductCategories;
use crate::domain::product_categories::input::{CreateProductCategoriesInput, UpdateProductCategoriesInput};
use crate::domain::product_categories::service::ProductCategoriesService;
use crate::enterprise::bus::{BusError, Command, CommandHandler, Query, QueryHandler};
use crate::enterprise::repository::{Page, Paginated};

/// Handler bundle for the `product_categories` context.
///
/// The same instance can be registered for every command and query of
/// this context because the bus wraps each registration individually.
#[derive(Clone)]
pub struct ProductCategoriesHandlers {
    service: Arc<dyn ProductCategoriesService>,
}

impl ProductCategoriesHandlers {
    /// Creates the bundle; clone it per registration.
    pub fn new(service: Arc<dyn ProductCategoriesService>) -> Self {
        Self { service }
    }
}

// ─── Commands (write side) ─────────────────────────────────────────────────

/// Persists a new `product_categories` record.
pub struct CreateProductCategoriesCommand {
    /// Validated payload from the HTTP layer.
    pub input: CreateProductCategoriesInput,
}

impl Command for CreateProductCategoriesCommand {
    type Output = ProductCategories;
}

#[async_trait]
impl CommandHandler<CreateProductCategoriesCommand> for ProductCategoriesHandlers {
    async fn handle(&self, command: CreateProductCategoriesCommand) -> Result<ProductCategories, BusError> {
        Ok(self.service.create(command.input).await?)
    }
}

/// Applies a partial update to an existing `product_categories` record.
pub struct UpdateProductCategoriesCommand {
    /// Primary key of the record.
    pub id: i32,
    /// Fields to overwrite (absent fields keep their value).
    pub input: UpdateProductCategoriesInput,
}

impl Command for UpdateProductCategoriesCommand {
    type Output = Option<ProductCategories>;
}

#[async_trait]
impl CommandHandler<UpdateProductCategoriesCommand> for ProductCategoriesHandlers {
    async fn handle(&self, command: UpdateProductCategoriesCommand) -> Result<Option<ProductCategories>, BusError> {
        Ok(self.service.update(command.id, command.input).await?)
    }
}

/// Removes a `product_categories` record (hard delete).
pub struct DeleteProductCategoriesCommand {
    /// Primary key of the record.
    pub id: i32,
}

impl Command for DeleteProductCategoriesCommand {
    type Output = bool;
}

#[async_trait]
impl CommandHandler<DeleteProductCategoriesCommand> for ProductCategoriesHandlers {
    async fn handle(&self, command: DeleteProductCategoriesCommand) -> Result<bool, BusError> {
        Ok(self.service.delete(command.id).await?)
    }
}

/// Changes the status of a `product_categories` record.
pub struct SetProductCategoriesStatusCommand {
    /// Primary key of the record.
    pub id: i32,
    /// New status value.
    pub status: String,
}

impl Command for SetProductCategoriesStatusCommand {
    type Output = Option<ProductCategories>;
}

#[async_trait]
impl CommandHandler<SetProductCategoriesStatusCommand> for ProductCategoriesHandlers {
    async fn handle(&self, command: SetProductCategoriesStatusCommand) -> Result<Option<ProductCategories>, BusError> {
        Ok(self.service.set_status(command.id, command.status).await?)
    }
}

/// Fetches one `product_categories` record by primary key.
pub struct GetProductCategoriesQuery {
    /// Primary key.
    pub id: i32,
}

impl Query for GetProductCategoriesQuery {
    type Output = Option<ProductCategories>;
}

#[async_trait]
impl QueryHandler<GetProductCategoriesQuery> for ProductCategoriesHandlers {
    async fn handle(&self, query: GetProductCategoriesQuery) -> Result<Option<ProductCategories>, BusError> {
        Ok(self.service.get(query.id).await?)
    }
}

/// Lists `product_categories` records page by page.
pub struct ListProductCategoriesQuery {
    /// Pagination request.
    pub page: Page,
}

impl Query for ListProductCategoriesQuery {
    type Output = Paginated<ProductCategories>;
}

#[async_trait]
impl QueryHandler<ListProductCategoriesQuery> for ProductCategoriesHandlers {
    async fn handle(&self, query: ListProductCategoriesQuery) -> Result<Paginated<ProductCategories>, BusError> {
        Ok(self.service.list(query.page).await?)
    }
}

/// Lists `product_categories` records sharing a status value.
pub struct ListProductCategoriesByStatusQuery {
    /// Status to filter on.
    pub status: String,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListProductCategoriesByStatusQuery {
    type Output = Vec<ProductCategories>;
}

#[async_trait]
impl QueryHandler<ListProductCategoriesByStatusQuery> for ProductCategoriesHandlers {
    async fn handle(&self, query: ListProductCategoriesByStatusQuery) -> Result<Vec<ProductCategories>, BusError> {
        Ok(self.service.list_by_status(query.status, query.page).await?)
    }
}

/// Lists `product_categories` records created at or after a timestamp.
pub struct ListProductCategoriesCreatedAfterQuery {
    /// Inclusive lower bound.
    pub ts: DateTime<Utc>,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListProductCategoriesCreatedAfterQuery {
    type Output = Vec<ProductCategories>;
}

#[async_trait]
impl QueryHandler<ListProductCategoriesCreatedAfterQuery> for ProductCategoriesHandlers {
    async fn handle(&self, query: ListProductCategoriesCreatedAfterQuery) -> Result<Vec<ProductCategories>, BusError> {
        Ok(self.service.list_created_after(query.ts, query.page).await?)
    }
}


