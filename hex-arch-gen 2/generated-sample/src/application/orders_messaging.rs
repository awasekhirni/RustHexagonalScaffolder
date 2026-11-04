//! Enterprise-bus wiring for `orders` — commands (write side), queries
//! (read side) and the handler bundle that executes them.
//!
//! One handler struct implements [`CommandHandler`]/[`QueryHandler`] for
//! many message types; the bus erases each registration behind a
//! `TypeId`-keyed adapter, so HTTP handlers never see services directly.

use std::sync::Arc;

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::orders::enums::OrderStatusEnum;
use crate::domain::orders::entity::Orders;
use crate::domain::orders::input::{CreateOrdersInput, UpdateOrdersInput};
use crate::domain::orders::service::OrdersService;
use crate::enterprise::bus::{BusError, Command, CommandHandler, Query, QueryHandler};
use crate::enterprise::repository::{Page, Paginated};

/// Handler bundle for the `orders` context.
///
/// The same instance can be registered for every command and query of
/// this context because the bus wraps each registration individually.
#[derive(Clone)]
pub struct OrdersHandlers {
    service: Arc<dyn OrdersService>,
}

impl OrdersHandlers {
    /// Creates the bundle; clone it per registration.
    pub fn new(service: Arc<dyn OrdersService>) -> Self {
        Self { service }
    }
}

// ─── Commands (write side) ─────────────────────────────────────────────────

/// Persists a new `orders` record.
pub struct CreateOrdersCommand {
    /// Validated payload from the HTTP layer.
    pub input: CreateOrdersInput,
}

impl Command for CreateOrdersCommand {
    type Output = Orders;
}

#[async_trait]
impl CommandHandler<CreateOrdersCommand> for OrdersHandlers {
    async fn handle(&self, command: CreateOrdersCommand) -> Result<Orders, BusError> {
        Ok(self.service.create(command.input).await?)
    }
}

/// Applies a partial update to an existing `orders` record.
pub struct UpdateOrdersCommand {
    /// Primary key of the record.
    pub id: Uuid,
    /// Fields to overwrite (absent fields keep their value).
    pub input: UpdateOrdersInput,
}

impl Command for UpdateOrdersCommand {
    type Output = Option<Orders>;
}

#[async_trait]
impl CommandHandler<UpdateOrdersCommand> for OrdersHandlers {
    async fn handle(&self, command: UpdateOrdersCommand) -> Result<Option<Orders>, BusError> {
        Ok(self.service.update(command.id, command.input).await?)
    }
}

/// Removes a `orders` record (hard delete).
pub struct DeleteOrdersCommand {
    /// Primary key of the record.
    pub id: Uuid,
}

impl Command for DeleteOrdersCommand {
    type Output = bool;
}

#[async_trait]
impl CommandHandler<DeleteOrdersCommand> for OrdersHandlers {
    async fn handle(&self, command: DeleteOrdersCommand) -> Result<bool, BusError> {
        Ok(self.service.delete(command.id).await?)
    }
}

/// Changes the status of a `orders` record.
pub struct SetOrdersStatusCommand {
    /// Primary key of the record.
    pub id: Uuid,
    /// New status value.
    pub status: OrderStatusEnum,
}

impl Command for SetOrdersStatusCommand {
    type Output = Option<Orders>;
}

#[async_trait]
impl CommandHandler<SetOrdersStatusCommand> for OrdersHandlers {
    async fn handle(&self, command: SetOrdersStatusCommand) -> Result<Option<Orders>, BusError> {
        Ok(self.service.set_status(command.id, command.status).await?)
    }
}

/// Soft-deletes (tombstones) a `orders` record.
pub struct SoftDeleteOrdersCommand {
    /// Primary key of the record.
    pub id: Uuid,
}

impl Command for SoftDeleteOrdersCommand {
    type Output = bool;
}

#[async_trait]
impl CommandHandler<SoftDeleteOrdersCommand> for OrdersHandlers {
    async fn handle(&self, command: SoftDeleteOrdersCommand) -> Result<bool, BusError> {
        Ok(self.service.soft_delete(command.id).await?)
    }
}

/// Restores a soft-deleted `orders` record.
pub struct RestoreOrdersCommand {
    /// Primary key of the record.
    pub id: Uuid,
}

impl Command for RestoreOrdersCommand {
    type Output = Option<Orders>;
}

#[async_trait]
impl CommandHandler<RestoreOrdersCommand> for OrdersHandlers {
    async fn handle(&self, command: RestoreOrdersCommand) -> Result<Option<Orders>, BusError> {
        Ok(self.service.restore(command.id).await?)
    }
}

/// Fetches one `orders` record by primary key.
pub struct GetOrdersQuery {
    /// Primary key.
    pub id: Uuid,
}

impl Query for GetOrdersQuery {
    type Output = Option<Orders>;
}

#[async_trait]
impl QueryHandler<GetOrdersQuery> for OrdersHandlers {
    async fn handle(&self, query: GetOrdersQuery) -> Result<Option<Orders>, BusError> {
        Ok(self.service.get(query.id).await?)
    }
}

/// Lists `orders` records page by page.
pub struct ListOrdersQuery {
    /// Pagination request.
    pub page: Page,
}

impl Query for ListOrdersQuery {
    type Output = Paginated<Orders>;
}

#[async_trait]
impl QueryHandler<ListOrdersQuery> for OrdersHandlers {
    async fn handle(&self, query: ListOrdersQuery) -> Result<Paginated<Orders>, BusError> {
        Ok(self.service.list(query.page).await?)
    }
}

/// Lists `orders` records sharing a status value.
pub struct ListOrdersByStatusQuery {
    /// Status to filter on.
    pub status: OrderStatusEnum,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListOrdersByStatusQuery {
    type Output = Vec<Orders>;
}

#[async_trait]
impl QueryHandler<ListOrdersByStatusQuery> for OrdersHandlers {
    async fn handle(&self, query: ListOrdersByStatusQuery) -> Result<Vec<Orders>, BusError> {
        Ok(self.service.list_by_status(query.status, query.page).await?)
    }
}

/// Lists `orders` records belonging to a user.
pub struct ListOrdersByUserQuery {
    /// Owning user's key.
    pub user_id: Uuid,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListOrdersByUserQuery {
    type Output = Vec<Orders>;
}

#[async_trait]
impl QueryHandler<ListOrdersByUserQuery> for OrdersHandlers {
    async fn handle(&self, query: ListOrdersByUserQuery) -> Result<Vec<Orders>, BusError> {
        Ok(self.service.list_by_user(query.user_id, query.page).await?)
    }
}

/// Lists `orders` records created at or after a timestamp.
pub struct ListOrdersCreatedAfterQuery {
    /// Inclusive lower bound.
    pub ts: DateTime<Utc>,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListOrdersCreatedAfterQuery {
    type Output = Vec<Orders>;
}

#[async_trait]
impl QueryHandler<ListOrdersCreatedAfterQuery> for OrdersHandlers {
    async fn handle(&self, query: ListOrdersCreatedAfterQuery) -> Result<Vec<Orders>, BusError> {
        Ok(self.service.list_created_after(query.ts, query.page).await?)
    }
}

/// Lists `orders` records updated at or after a timestamp.
pub struct ListOrdersUpdatedAfterQuery {
    /// Inclusive lower bound.
    pub ts: DateTime<Utc>,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListOrdersUpdatedAfterQuery {
    type Output = Vec<Orders>;
}

#[async_trait]
impl QueryHandler<ListOrdersUpdatedAfterQuery> for OrdersHandlers {
    async fn handle(&self, query: ListOrdersUpdatedAfterQuery) -> Result<Vec<Orders>, BusError> {
        Ok(self.service.list_updated_after(query.ts, query.page).await?)
    }
}


