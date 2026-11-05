//! Application service adapter for `users` — the concrete inbound adapter
//! implementing the [UsersService] port on top of the repository port.
//!
//! The service is where business rules live: payload validation, invariant
//! checks, and orchestration of the repository port. Swap-in adapters for
//! other stores only need to implement the repository port.

use std::sync::Arc;

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::users::enums::UserStatusEnum;
use crate::application::users_mapper::UsersMapper;
use crate::domain::users::entity::Users;
use crate::domain::users::input::{CreateUsersInput, UpdateUsersInput};
use crate::domain::users::repository::UsersRepository;
use crate::domain::users::service::UsersService;
use crate::enterprise::repository::{Page, Paginated, RepositoryError};

/// Concrete service for `users`; injected into the bus handlers by `main`.
pub struct UsersServiceImpl {
    repo: Arc<dyn UsersRepository>,
}

impl UsersServiceImpl {
    /// Wires the service to its repository port.
    pub fn new(repo: Arc<dyn UsersRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl UsersService for UsersServiceImpl {
    async fn get(&self, id: Uuid) -> Result<Option<Users>, RepositoryError> {
        self.repo.find_by_id(id).await
    }

    async fn list(&self, page: Page) -> Result<Paginated<Users>, RepositoryError> {
        self.repo.find_all(page).await
    }

    async fn create(&self, input: CreateUsersInput) -> Result<Users, RepositoryError> {
        input.validate().map_err(RepositoryError::Validation)?;
        let entity = UsersMapper::to_entity(input);
        entity.validate().map_err(RepositoryError::Validation)?;
        self.repo.create(&entity).await
    }

    async fn update(&self, id: Uuid, input: UpdateUsersInput) -> Result<Option<Users>, RepositoryError> {
        let current = match self.repo.find_by_id(id).await? {
            Some(current) => current,
            None => return Ok(None),
        };
        let merged = UsersMapper::apply_update(current, &input);
        merged.validate().map_err(RepositoryError::Validation)?;
        self.repo.update(id, &merged).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool, RepositoryError> {
        self.repo.delete(id).await
    }

    async fn list_by_status(&self, status: UserStatusEnum, page: Page) -> Result<Vec<Users>, RepositoryError> {
        self.repo.find_by_status(status, page).await
    }

    async fn set_status(&self, id: Uuid, status: UserStatusEnum) -> Result<Option<Users>, RepositoryError> {
        self.repo.set_status(id, status).await
    }

    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, RepositoryError> {
        self.repo.find_created_after(ts, page).await
    }

    async fn list_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Users>, RepositoryError> {
        self.repo.find_updated_after(ts, page).await
    }
}
