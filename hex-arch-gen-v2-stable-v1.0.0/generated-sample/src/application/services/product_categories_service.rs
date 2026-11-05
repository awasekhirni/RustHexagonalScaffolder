//! Application service adapter for `product_categories` — the concrete inbound adapter
//! implementing the inbound use-case contract on top of the repository port.
//!
//! The service is where business rules live: payload validation, invariant
//! checks, and orchestration of the repository port. Swap-in adapters for
//! other stores only need to implement the repository port.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::instrument;

use chrono::{DateTime, Utc};
use crate::application::dto::product_categories_dto::{CreateProductCategoriesInput, UpdateProductCategoriesInput};
use crate::application::ports::repository::{ProductCategoriesRepository, ProductCategoriesServicePort, Page, Paginated};
use crate::domain::entities::product_categories::ProductCategories;
use crate::shared::error::AppError;

/// Concrete service for `product_categories`; injected into handlers by `main`.
#[derive(Clone)]
pub struct ProductCategoriesService {
    repo: Arc<dyn ProductCategoriesRepository>,
}

impl ProductCategoriesService {
    /// Wires the service to its repository port.
    #[must_use]
    pub fn new(repo: Arc<dyn ProductCategoriesRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ProductCategoriesServicePort for ProductCategoriesService {
    #[instrument(skip(self, _input), fields(table = "product_categories", op = "create"))]
    async fn create(&self, _input: CreateProductCategoriesInput) -> Result<ProductCategories, AppError> {
        _input.validate().map_err(AppError::Validation)?;
        // Build entity from input via the trivial mapper.
        let mut entity = ProductCategories::default();
        entity.name = _input.name;
        entity.r#type = _input.r#type;
        entity.status = _input.status;
        entity.validate().map_err(AppError::Validation)?;
        let created = self.repo.create(&entity).await?;
        Ok(created)
    }

    #[instrument(skip(self), fields(table = "product_categories", op = "get", id = ?id))]
    async fn get(&self, id: i32) -> Result<Option<ProductCategories>, AppError> {
        Ok(self.repo.find_by_id(id).await?)
    }

    #[instrument(skip(self), fields(table = "product_categories", op = "list"))]
    async fn list(&self, page: Page) -> Result<Paginated<ProductCategories>, AppError> {
        Ok(self.repo.find_all(page).await?)
    }

    #[instrument(skip(self, _input), fields(table = "product_categories", op = "update"))]
    async fn update(&self, id: i32, _input: UpdateProductCategoriesInput) -> Result<Option<ProductCategories>, AppError> {
        let mut current = match self.repo.find_by_id(id).await? {
            Some(c) => c,
            None => return Ok(None),
        };
        if let Some(v) = &_input.name {
            current.name = v.clone();
        }
        if let Some(v) = &_input.r#type {
            current.r#type = v.clone();
        }
        if let Some(v) = &_input.status {
            current.status = v.clone();
        }
        let updated = self.repo.update(id, &current).await?;
        Ok(updated)
    }

    #[instrument(skip(self), fields(table = "product_categories", op = "delete"))]
    async fn delete(&self, id: i32) -> Result<bool, AppError> {
        Ok(self.repo.delete(id).await?)
    }

    async fn list_by_status(&self, status: String, page: Page) -> Result<Vec<ProductCategories>, AppError> {
        Ok(self.repo.find_by_status(status, page).await?)
    }

    async fn set_status(&self, id: i32, status: String) -> Result<Option<ProductCategories>, AppError> {
        Ok(self.repo.set_status(id, status).await?)
    }

    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<ProductCategories>, AppError> {
        Ok(self.repo.find_created_after(ts, page).await?)
    }
}
