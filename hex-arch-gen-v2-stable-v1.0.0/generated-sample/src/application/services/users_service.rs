//! Application service adapter for `users` — the concrete inbound adapter
//! implementing the inbound use-case contract on top of the repository port.
//!
//! The service is where business rules live: payload validation, invariant
//! checks, and orchestration of the repository port. Swap-in adapters for
//! other stores only need to implement the repository port.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::instrument;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::entities::users::UserStatusEnum;
use crate::application::dto::users_dto::{CreateUsersInput, UpdateUsersInput};
use crate::application::ports::repository::{UsersRepository, UsersServicePort, Page, Paginated};
use crate::domain::entities::users::Users;
use crate::shared::error::AppError;

/// Concrete service for `users`; injected into handlers by `main`.
#[derive(Clone)]
pub struct UsersService {
    repo: Arc<dyn UsersRepository>,
}

impl UsersService {
    /// Wires the service to its repository port.
    #[must_use]
    pub fn new(repo: Arc<dyn UsersRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl UsersServicePort for UsersService {
    #[instrument(skip(self, _input), fields(table = "users", op = "create"))]
    async fn create(&self, _input: CreateUsersInput) -> Result<Users, AppError> {
        _input.validate().map_err(AppError::Validation)?;
        // Build entity from input via the trivial mapper.
        let mut entity = Users::default();
        entity.email = _input.email;
        entity.full_name = _input.full_name;
        entity.metadata = _input.metadata;
        entity.tags = _input.tags;
        entity.deleted_at = _input.deleted_at;
        entity.validate().map_err(AppError::Validation)?;
        let created = self.repo.create(&entity).await?;
        Ok(created)
    }

    #[instrument(skip(self), fields(table = "users", op = "get", id = ?id))]
    async fn get(&self, id: Uuid) -> Result<Option<Users>, AppError> {
        Ok(self.repo.find_by_id(id).await?)
    }

    #[instrument(skip(self), fields(table = "users", op = "list"))]
    async fn list(&self, page: Page) -> Result<Paginated<Users>, AppError> {
        Ok(self.repo.find_all(page).await?)
    }

    #[instrument(skip(self, _input), fields(table = "users", op = "update"))]
    async fn update(&self, id: Uuid, _input: UpdateUsersInput) -> Result<Option<Users>, AppError> {
        let mut current = match self.repo.find_by_id(id).await? {
            Some(c) => c,
            None => return Ok(None),
        };
        if let Some(v) = &_input.email {
            current.email = v.clone();
        }
        if let Some(v) = &_input.full_name {
            current.full_name = Some(v.clone());
        }
        if let Some(v) = &_input.user_status {
            current.user_status = v.clone();
        }
        if let Some(v) = &_input.is_active {
            current.is_active = v.clone();
        }
        if let Some(v) = &_input.metadata {
            current.metadata = Some(v.clone());
        }
        if let Some(v) = &_input.tags {
            current.tags = Some(v.clone());
        }
        let updated = self.repo.update(id, &current).await?;
        Ok(updated)
    }

    #[instrument(skip(self), fields(table = "users", op = "delete"))]
    async fn delete(&self, id: Uuid) -> Result<bool, AppError> {
        Ok(self.repo.delete(id).await?)
    }

    async fn list_by_status(&self, status: UserStatusEnum, page: Page) -> Result<Vec<Users>, AppError> {
        Ok(self.repo.find_by_status(status, page).await?)
    }

    async fn set_status(&self, id: Uuid, status: UserStatusEnum) -> Result<Option<Users>, AppError> {
        Ok(self.repo.set_status(id, status).await?)
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, AppError> {
        Ok(self.repo.soft_delete(id).await?)
    }

    async fn restore(&self, id: Uuid) -> Result<Option<Users>, AppError> {
        Ok(self.repo.restore(id).await?)
    }

    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, AppError> {
        Ok(self.repo.find_created_after(ts, page).await?)
    }

    async fn list_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, AppError> {
        Ok(self.repo.find_updated_after(ts, page).await?)
    }
}
