//! Application service adapter for `product_categories` — the concrete inbound adapter
//! implementing the [ProductCategoriesService] port on top of the repository port.
//!
//! The service is where business rules live: payload validation, invariant
//! checks, and orchestration of the repository port. Swap-in adapters for
//! other stores only need to implement the repository port.

use std::sync::Arc;

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use crate::application::product_categories_mapper::ProductCategoriesMapper;
use crate::domain::product_categories::entity::ProductCategories;
use crate::domain::product_categories::input::{CreateProductCategoriesInput, UpdateProductCategoriesInput};
use crate::domain::product_categories::repository::ProductCategoriesRepository;
use crate::domain::product_categories::service::ProductCategoriesService;
use crate::enterprise::repository::{Page, Paginated, RepositoryError};

/// Concrete service for `product_categories`; injected into the bus handlers by `main`.
pub struct ProductCategoriesServiceImpl {
    repo: Arc<dyn ProductCategoriesRepository>,
}

impl ProductCategoriesServiceImpl {
    /// Wires the service to its repository port.
    pub fn new(repo: Arc<dyn ProductCategoriesRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ProductCategoriesService for ProductCategoriesServiceImpl {
    async fn get(&self, id: i32) -> Result<Option<ProductCategories>, RepositoryError> {
        self.repo.find_by_id(id).await
    }

    async fn list(&self, page: Page) -> Result<Paginated<ProductCategories>, RepositoryError> {
        self.repo.find_all(page).await
    }

    async fn create(&self, input: CreateProductCategoriesInput) -> Result<ProductCategories, RepositoryError> {
        input.validate().map_err(RepositoryError::Validation)?;
        let entity = ProductCategoriesMapper::to_entity(input);
        entity.validate().map_err(RepositoryError::Validation)?;
        self.repo.create(&entity).await
    }

    async fn update(&self, id: i32, input: UpdateProductCategoriesInput) -> Result<Option<ProductCategories>, RepositoryError> {
        let current = match self.repo.find_by_id(id).await? {
            Some(current) => current,
            None => return Ok(None),
        };
        let merged = ProductCategoriesMapper::apply_update(current, &input);
        merged.validate().map_err(RepositoryError::Validation)?;
        self.repo.update(id, &merged).await
    }

    async fn delete(&self, id: i32) -> Result<bool, RepositoryError> {
        self.repo.delete(id).await
    }

    async fn list_by_status(&self, status: String, page: Page) -> Result<Vec<ProductCategories>, RepositoryError> {
        self.repo.find_by_status(status, page).await
    }

    async fn set_status(&self, id: i32, status: String) -> Result<Option<ProductCategories>, RepositoryError> {
        self.repo.set_status(id, status).await
    }

    async fn list_created_after(&self, ts: DateTime<Utc>, page: Page) -> Result<Vec<ProductCategories>, RepositoryError> {
        self.repo.find_created_after(ts, page).await
    }
}
