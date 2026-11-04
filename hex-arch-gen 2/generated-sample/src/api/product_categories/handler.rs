//! Axum handlers for the `product_categories` context.
//!
//! Every handler is a thin inbound adapter: it deserializes transport
//! types, dispatches through the enterprise buses and maps errors onto
//! HTTP status codes. No business logic lives here.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use crate::api::state::AppState;
use crate::api::product_categories::dto::{ProductCategoriesCreatedAfterParams, ProductCategoriesPageParams, SetProductCategoriesStatusInput};
use crate::application::product_categories_messaging::{CreateProductCategoriesCommand, DeleteProductCategoriesCommand, GetProductCategoriesQuery, ListProductCategoriesByStatusQuery, ListProductCategoriesCreatedAfterQuery, ListProductCategoriesQuery, SetProductCategoriesStatusCommand, UpdateProductCategoriesCommand};
use crate::domain::product_categories::entity::ProductCategories;
use crate::domain::product_categories::input::{CreateProductCategoriesInput, UpdateProductCategoriesInput};
use crate::error::ApiError;

/// `GET /api/v1/product_categories` — paginated listing (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/product_categories",
    tag = "product_categories",
    operation_id = "list_product_categories",
    responses((status = 200, body = Vec<ProductCategories>))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<ProductCategoriesPageParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = state.qry.ask(ListProductCategoriesQuery { page: params.to_page() }).await?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
        "page": result.page,
        "per_page": result.per_page,
    })))
}

/// `POST /api/v1/product_categories` — create (command bus).
#[utoipa::path(
    post,
    path = "/api/v1/product_categories",
    tag = "product_categories",
    operation_id = "create_product_categories",
    request_body = CreateProductCategoriesInput,
    responses((status = 201, body = ProductCategories), (status = 400), (status = 409))
)]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateProductCategoriesInput>,
) -> Result<(StatusCode, Json<ProductCategories>), ApiError> {
    let created = state.cmd.send(CreateProductCategoriesCommand { input }).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `GET /api/v1/product_categories/{id}` — fetch one (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/product_categories/{id}",
    tag = "product_categories",
    operation_id = "get_product_categories",
    responses((status = 200, body = ProductCategories), (status = 404))
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ProductCategories>, ApiError> {
    state
        .qry
        .ask(GetProductCategoriesQuery { id })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `PUT /api/v1/product_categories/{id}` — partial update (command bus).
#[utoipa::path(
    put,
    path = "/api/v1/product_categories/{id}",
    tag = "product_categories",
    operation_id = "update_product_categories",
    request_body = UpdateProductCategoriesInput,
    responses((status = 200, body = ProductCategories), (status = 404), (status = 409))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(input): Json<UpdateProductCategoriesInput>,
) -> Result<Json<ProductCategories>, ApiError> {
    state
        .cmd
        .send(UpdateProductCategoriesCommand { id, input })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `DELETE /api/v1/product_categories/{id}` — hard delete (command bus).
#[utoipa::path(
    delete,
    path = "/api/v1/product_categories/{id}",
    tag = "product_categories",
    operation_id = "delete_product_categories",
    responses((status = 204), (status = 404))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    if state.cmd.send(DeleteProductCategoriesCommand { id }).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

/// `GET /api/v1/product_categories/status/{status}` — list by status (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/product_categories/status/{status}",
    tag = "product_categories",
    operation_id = "list_product_categories_by_status",
    responses((status = 200, body = Vec<ProductCategories>))
)]
pub async fn list_by_status(
    State(state): State<AppState>,
    Path(status): Path<String>,
    Query(params): Query<ProductCategoriesPageParams>,
) -> Result<Json<Vec<ProductCategories>>, ApiError> {
    let result = state
        .qry
        .ask(ListProductCategoriesByStatusQuery { status, page: params.to_page() })
        .await?;
    Ok(Json(result))
}

/// `PATCH /api/v1/product_categories/{id}/status` — change status (command bus).
#[utoipa::path(
    patch,
    path = "/api/v1/product_categories/{id}/status",
    tag = "product_categories",
    operation_id = "set_product_categories_status",
    request_body = SetProductCategoriesStatusInput,
    responses((status = 200, body = ProductCategories), (status = 404))
)]
pub async fn set_status(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<SetProductCategoriesStatusInput>,
) -> Result<Json<ProductCategories>, ApiError> {
    state
        .cmd
        .send(SetProductCategoriesStatusCommand { id, status: body.status })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `GET /api/v1/product_categories/created-after` — created since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/product_categories/created-after",
    tag = "product_categories",
    operation_id = "list_product_categories_created_after",
    responses((status = 200, body = Vec<ProductCategories>))
)]
pub async fn created_after(
    State(state): State<AppState>,
    Query(params): Query<ProductCategoriesCreatedAfterParams>,
) -> Result<Json<Vec<ProductCategories>>, ApiError> {
    let result = state
        .qry
        .ask(ListProductCategoriesCreatedAfterQuery { ts: params.after, page: crate::enterprise::repository::Page::new(1, 50) })
        .await?;
    Ok(Json(result))
}
