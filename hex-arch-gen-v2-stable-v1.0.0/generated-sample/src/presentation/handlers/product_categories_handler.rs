//! Axum handlers for the `product_categories` context.
//!
//! Every handler is a thin inbound adapter: it deserializes transport
//! types, calls the application service, and maps errors onto HTTP status
//! codes. No business logic lives here.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use tracing::instrument;
use validator::Validate;
use chrono::{DateTime, Utc};
use crate::application::dto::product_categories_dto::{CreateProductCategoriesInput, UpdateProductCategoriesInput};
use crate::application::ports::repository::{Page, Paginated, ProductCategoriesServicePort};
use crate::domain::entities::product_categories::ProductCategories;
use crate::presentation::state::AppState;
use crate::shared::error::AppError;
use crate::shared::pagination::Pagination;


/// `GET /api/v1/product_categories` — paginated listing.
#[utoipa::path(
    get,
    path = "/api/v1/product_categories",
    tag = "product_categories",
    operation_id = "list_product_categories",
    params(Pagination),
    responses((status = 200, body = Paginated<ProductCategories>))
)]
#[instrument(skip(state), fields(path = "/api/v1/product_categories"))]
pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Paginated<ProductCategories>>, AppError> {
    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);
    let result = state.product_categories_service.list(page).await?;
    Ok(Json(result))
}

/// `POST /api/v1/product_categories` — create.
#[utoipa::path(
    post,
    path = "/api/v1/product_categories",
    tag = "product_categories",
    operation_id = "create_product_categories",
    request_body = CreateProductCategoriesInput,
    responses((status = 201, body = ProductCategories), (status = 400), (status = 409))
)]
#[instrument(skip(state, input), fields(path = "/api/v1/product_categories"))]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateProductCategoriesInput>,
) -> Result<(StatusCode, Json<ProductCategories>), AppError> {
    let created = state.product_categories_service.create(input).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `GET /api/v1/product_categories/{id}` — fetch one.
#[utoipa::path(
    get,
    path = "/api/v1/product_categories/{id}",
    tag = "product_categories",
    operation_id = "get_product_categories",
    responses((status = 200, body = ProductCategories), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/product_categories/{id}"))]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ProductCategories>, AppError> {
    state
        .product_categories_service
        .get(id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("ProductCategories {id} not found")))
}

/// `PUT /api/v1/product_categories/{id}` — partial update.
#[utoipa::path(
    put,
    path = "/api/v1/product_categories/{id}",
    tag = "product_categories",
    operation_id = "update_product_categories",
    request_body = UpdateProductCategoriesInput,
    responses((status = 200, body = ProductCategories), (status = 404), (status = 409))
)]
#[instrument(skip(state, input), fields(path = "/api/v1/product_categories/{id}"))]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(input): Json<UpdateProductCategoriesInput>,
) -> Result<Json<ProductCategories>, AppError> {
    state
        .product_categories_service
        .update(id, input)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("ProductCategories {id} not found")))
}

/// `DELETE /api/v1/product_categories/{id}` — delete (soft when available).
#[utoipa::path(
    delete,
    path = "/api/v1/product_categories/{id}",
    tag = "product_categories",
    operation_id = "delete_product_categories",
    responses((status = 204), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/product_categories/{id}"))]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    if state.product_categories_service.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("ProductCategories {id} not found")))
    }
}

/// `GET /api/v1/product_categories/status/{status}` — list by status.
#[utoipa::path(
    get,
    path = "/api/v1/product_categories/status/{status}",
    tag = "product_categories",
    operation_id = "list_product_categories_by_status",
    responses((status = 200, body = Vec<ProductCategories>))
)]
#[instrument(skip(state), fields(path = "/api/v1/product_categories/status/{status}"))]
pub async fn list_by_status(
    State(state): State<AppState>,
    Path(status): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<ProductCategories>>, AppError> {
    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);
    let result = state.product_categories_service.list_by_status(status, page).await?;
    Ok(Json(result))
}

/// `PATCH /api/v1/product_categories/{id}/status` — change status.
#[utoipa::path(
    patch,
    path = "/api/v1/product_categories/{id}/status",
    tag = "product_categories",
    operation_id = "set_product_categories_status",
    request_body = SetProductCategoriesStatusInput,
    responses((status = 200, body = ProductCategories), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/product_categories/{id}/status"))]
pub async fn set_status(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<SetProductCategoriesStatusInput>,
) -> Result<Json<ProductCategories>, AppError> {
    state
        .product_categories_service
        .set_status(id, body.status)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("ProductCategories {id} not found")))
}

/// Body of `PATCH /product_categories/{id}/status`.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct SetProductCategoriesStatusInput {
    /// New status value.
    pub status: String,
}

/// `GET /api/v1/product_categories/created-after?after=…` — created since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/product_categories/created-after",
    tag = "product_categories",
    operation_id = "list_product_categories_created_after",
    responses((status = 200, body = Vec<ProductCategories>))
)]
#[instrument(skip(state), fields(path = "/api/v1/product_categories/created-after"))]
pub async fn created_after(
    State(state): State<AppState>,
    Query(params): Query<ProductCategoriesCreatedAfterParams>,
) -> Result<Json<Vec<ProductCategories>>, AppError> {
    let page = Page::new(1, 50);
    let result = state.product_categories_service.list_created_after(params.after, page).await?;
    Ok(Json(result))
}

/// Query parameters for `GET /product_categories/created-after`.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct ProductCategoriesCreatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}
