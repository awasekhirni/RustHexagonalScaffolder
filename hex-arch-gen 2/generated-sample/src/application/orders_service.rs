//! Application service adapter for `orders` — the concrete inbound adapter
//! implementing the [OrdersService] port on top of the repository port.
//!
//! The service is where business rules live: payload validation, invariant
//! checks, and orchestration of the repository port. Swap-in adapters for
//! other stores only need to implement the repository port.

use std::sync::Arc;

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::orders::enums::OrderStatusEnum;
use crate::application::orders_mapper::OrdersMapper;
use crate::domain::orders::entity::Orders;
use crate::domain::orders::input::{CreateOrdersInput, UpdateOrdersInput};
use crate::domain::orders::repository::OrdersRepository;
use crate::domain::orders::service::OrdersService;
use crate::enterprise::repository::{Page, Paginated, RepositoryError};

/// Concrete service for `orders`; injected into the bus handlers by `main`.
pub struct OrdersServiceImpl {
    repo: Arc<dyn OrdersRepository>,
}

impl OrdersServiceImpl {
    /// Wires the service to its repository port.
    pub fn new(repo: Arc<dyn OrdersRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl OrdersService for OrdersServiceImpl {
    async fn get(&self, id: Uuid) -> Result<Option<Orders>, RepositoryError> {
        self.repo.find_by_id(id).await
    }

    async fn list(&self, page: Page) -> Result<Paginated<Orders>, RepositoryError> {
        self.repo.find_all(page).await
    }

    async fn create(&self, input: CreateOrdersInput) -> Result<Orders, RepositoryError> {
        input.validate().map_err(RepositoryError::Validation)?;
        let entity = OrdersMapper::to_entity(input);
        entity.validate().map_err(RepositoryError::Validation)?;
        self.repo.create(&entity).await
    }

    async fn update(&self, id: Uuid, input: UpdateOrdersInput) -> Result<Option<Orders>, RepositoryError> {
        let current = match self.repo.find_by_id(id).await? {
            Some(current) => current,
            None => return Ok(None),
        };
        let merged = OrdersMapper::apply_update(current, &input);
        merged.validate().map_err(RepositoryError::Validation)?;
        self.repo.update(id, &merged).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool, RepositoryError> {
        self.repo.delete(id).await
    }

    async fn list_by_status(&self, status: OrderStatusEnum, page: Page) -> Result<Vec<Orders>, RepositoryError> {
        self.repo.find_by_status(status, page).await
    }

    async fn set_status(&self, id: Uuid, status: OrderStatusEnum) -> Result<Option<Orders>, RepositoryError> {
        self.repo.set_status(id, status).await
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, RepositoryError> {
        self.repo.soft_delete(id).await
    }

    async fn restore(&self, id: Uuid) -> Result<Option<Orders>, RepositoryError> {
        self.repo.restore(id).await
    }

    async fn list_by_user(&self, user_id: Uuid, page: Page) -> Result<Vec<Orders>, RepositoryError> {
        self.repo.find_by_user(user_id, page).await
    }

    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, RepositoryError> {
        self.repo.find_created_after(ts, page).await
    }

    async fn list_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, RepositoryError> {
        self.repo.find_updated_after(ts, page).await
    }
}
