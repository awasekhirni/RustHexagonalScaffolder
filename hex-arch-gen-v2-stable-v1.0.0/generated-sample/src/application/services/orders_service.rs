//! Application service adapter for `orders` — the concrete inbound adapter
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
use crate::domain::entities::orders::OrderStatusEnum;
use crate::application::dto::orders_dto::{CreateOrdersInput, UpdateOrdersInput};
use crate::application::ports::repository::{OrdersRepository, OrdersServicePort, Page, Paginated};
use crate::domain::entities::orders::Orders;
use crate::shared::error::AppError;

/// Concrete service for `orders`; injected into handlers by `main`.
#[derive(Clone)]
pub struct OrdersService {
    repo: Arc<dyn OrdersRepository>,
}

impl OrdersService {
    /// Wires the service to its repository port.
    #[must_use]
    pub fn new(repo: Arc<dyn OrdersRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl OrdersServicePort for OrdersService {
    #[instrument(skip(self, _input), fields(table = "orders", op = "create"))]
    async fn create(&self, _input: CreateOrdersInput) -> Result<Orders, AppError> {
        _input.validate().map_err(AppError::Validation)?;
        // Build entity from input via the trivial mapper.
        let mut entity = Orders::default();
        entity.user_id = _input.user_id;
        entity.tenant_id = _input.tenant_id;
        entity.amount = _input.amount;
        entity.notes = _input.notes;
        entity.deleted_at = _input.deleted_at;
        entity.validate().map_err(AppError::Validation)?;
        let created = self.repo.create(&entity).await?;
        Ok(created)
    }

    #[instrument(skip(self), fields(table = "orders", op = "get", id = ?id))]
    async fn get(&self, id: Uuid) -> Result<Option<Orders>, AppError> {
        Ok(self.repo.find_by_id(id).await?)
    }

    #[instrument(skip(self), fields(table = "orders", op = "list"))]
    async fn list(&self, page: Page) -> Result<Paginated<Orders>, AppError> {
        Ok(self.repo.find_all(page).await?)
    }

    #[instrument(skip(self, _input), fields(table = "orders", op = "update"))]
    async fn update(&self, id: Uuid, _input: UpdateOrdersInput) -> Result<Option<Orders>, AppError> {
        let mut current = match self.repo.find_by_id(id).await? {
            Some(c) => c,
            None => return Ok(None),
        };
        if let Some(v) = &_input.user_id {
            current.user_id = v.clone();
        }
        if let Some(v) = &_input.tenant_id {
            current.tenant_id = v.clone();
        }
        if let Some(v) = &_input.amount {
            current.amount = v.clone();
        }
        if let Some(v) = &_input.order_status {
            current.order_status = v.clone();
        }
        if let Some(v) = &_input.notes {
            current.notes = Some(v.clone());
        }
        let updated = self.repo.update(id, &current).await?;
        Ok(updated)
    }

    #[instrument(skip(self), fields(table = "orders", op = "delete"))]
    async fn delete(&self, id: Uuid) -> Result<bool, AppError> {
        Ok(self.repo.delete(id).await?)
    }

    async fn list_by_status(&self, status: OrderStatusEnum, page: Page) -> Result<Vec<Orders>, AppError> {
        Ok(self.repo.find_by_status(status, page).await?)
    }

    async fn set_status(&self, id: Uuid, status: OrderStatusEnum) -> Result<Option<Orders>, AppError> {
        Ok(self.repo.set_status(id, status).await?)
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, AppError> {
        Ok(self.repo.soft_delete(id).await?)
    }

    async fn restore(&self, id: Uuid) -> Result<Option<Orders>, AppError> {
        Ok(self.repo.restore(id).await?)
    }

    async fn list_by_user(&self, user_id: Uuid, page: Page) -> Result<Vec<Orders>, AppError> {
        Ok(self.repo.find_by_user(user_id, page).await?)
    }

    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, AppError> {
        Ok(self.repo.find_created_after(ts, page).await?)
    }

    async fn list_updated_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<Orders>, AppError> {
        Ok(self.repo.find_updated_after(ts, page).await?)
    }
}
