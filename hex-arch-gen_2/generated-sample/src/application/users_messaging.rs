//! Enterprise-bus wiring for `users` — commands (write side), queries
//! (read side) and the handler bundle that executes them.
//!
//! One handler struct implements [`CommandHandler`]/[`QueryHandler`] for
//! many message types; the bus erases each registration behind a
//! `TypeId`-keyed adapter, so HTTP handlers never see services directly.

use std::sync::Arc;

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::users::enums::UserStatusEnum;
use crate::domain::users::entity::Users;
use crate::domain::users::input::{CreateUsersInput, UpdateUsersInput};
use crate::domain::users::service::UsersService;
use crate::enterprise::bus::{BusError, Command, CommandHandler, Query, QueryHandler};
use crate::enterprise::repository::{Page, Paginated};

/// Handler bundle for the `users` context.
///
/// The same instance can be registered for every command and query of
/// this context because the bus wraps each registration individually.
#[derive(Clone)]
pub struct UsersHandlers {
    service: Arc<dyn UsersService>,
}

impl UsersHandlers {
    /// Creates the bundle; clone it per registration.
    pub fn new(service: Arc<dyn UsersService>) -> Self {
        Self { service }
    }
}

// ─── Commands (write side) ─────────────────────────────────────────────────

/// Persists a new `users` record.
pub struct CreateUsersCommand {
    /// Validated payload from the HTTP layer.
    pub input: CreateUsersInput,
}

impl Command for CreateUsersCommand {
    type Output = Users;
}

#[async_trait]
impl CommandHandler<CreateUsersCommand> for UsersHandlers {
    async fn handle(&self, command: CreateUsersCommand) -> Result<Users, BusError> {
        Ok(self.service.create(command.input).await?)
    }
}

/// Applies a partial update to an existing `users` record.
pub struct UpdateUsersCommand {
    /// Primary key of the record.
    pub id: Uuid,
    /// Fields to overwrite (absent fields keep their value).
    pub input: UpdateUsersInput,
}

impl Command for UpdateUsersCommand {
    type Output = Option<Users>;
}

#[async_trait]
impl CommandHandler<UpdateUsersCommand> for UsersHandlers {
    async fn handle(&self, command: UpdateUsersCommand) -> Result<Option<Users>, BusError> {
        Ok(self.service.update(command.id, command.input).await?)
    }
}

/// Removes a `users` record (hard delete).
pub struct DeleteUsersCommand {
    /// Primary key of the record.
    pub id: Uuid,
}

impl Command for DeleteUsersCommand {
    type Output = bool;
}

#[async_trait]
impl CommandHandler<DeleteUsersCommand> for UsersHandlers {
    async fn handle(&self, command: DeleteUsersCommand) -> Result<bool, BusError> {
        Ok(self.service.delete(command.id).await?)
    }
}

/// Changes the status of a `users` record.
pub struct SetUsersStatusCommand {
    /// Primary key of the record.
    pub id: Uuid,
    /// New status value.
    pub status: UserStatusEnum,
}

impl Command for SetUsersStatusCommand {
    type Output = Option<Users>;
}

#[async_trait]
impl CommandHandler<SetUsersStatusCommand> for UsersHandlers {
    async fn handle(&self, command: SetUsersStatusCommand) -> Result<Option<Users>, BusError> {
        Ok(self.service.set_status(command.id, command.status).await?)
    }
}

/// Fetches one `users` record by primary key.
pub struct GetUsersQuery {
    /// Primary key.
    pub id: Uuid,
}

impl Query for GetUsersQuery {
    type Output = Option<Users>;
}

#[async_trait]
impl QueryHandler<GetUsersQuery> for UsersHandlers {
    async fn handle(&self, query: GetUsersQuery) -> Result<Option<Users>, BusError> {
        Ok(self.service.get(query.id).await?)
    }
}

/// Lists `users` records page by page.
pub struct ListUsersQuery {
    /// Pagination request.
    pub page: Page,
}

impl Query for ListUsersQuery {
    type Output = Paginated<Users>;
}

#[async_trait]
impl QueryHandler<ListUsersQuery> for UsersHandlers {
    async fn handle(&self, query: ListUsersQuery) -> Result<Paginated<Users>, BusError> {
        Ok(self.service.list(query.page).await?)
    }
}

/// Lists `users` records sharing a status value.
pub struct ListUsersByStatusQuery {
    /// Status to filter on.
    pub status: UserStatusEnum,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListUsersByStatusQuery {
    type Output = Vec<Users>;
}

#[async_trait]
impl QueryHandler<ListUsersByStatusQuery> for UsersHandlers {
    async fn handle(&self, query: ListUsersByStatusQuery) -> Result<Vec<Users>, BusError> {
        Ok(self.service.list_by_status(query.status, query.page).await?)
    }
}

/// Lists `users` records created at or after a timestamp.
pub struct ListUsersCreatedAfterQuery {
    /// Inclusive lower bound.
    pub ts: DateTime<Utc>,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListUsersCreatedAfterQuery {
    type Output = Vec<Users>;
}

#[async_trait]
impl QueryHandler<ListUsersCreatedAfterQuery> for UsersHandlers {
    async fn handle(&self, query: ListUsersCreatedAfterQuery) -> Result<Vec<Users>, BusError> {
        Ok(self.service.list_created_after(query.ts, query.page).await?)
    }
}

/// Lists `users` records updated at or after a timestamp.
pub struct ListUsersUpdatedAfterQuery {
    /// Inclusive lower bound.
    pub ts: DateTime<Utc>,
    /// Pagination request.
    pub page: Page,
}

impl Query for ListUsersUpdatedAfterQuery {
    type Output = Vec<Users>;
}

#[async_trait]
impl QueryHandler<ListUsersUpdatedAfterQuery> for UsersHandlers {
    async fn handle(&self, query: ListUsersUpdatedAfterQuery) -> Result<Vec<Users>, BusError> {
        Ok(self.service.list_updated_after(query.ts, query.page).await?)
    }
}


