//! Axum handlers for the `orders` context.
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
use uuid::Uuid;
use crate::application::dto::orders_dto::{CreateOrdersInput, UpdateOrdersInput};
use crate::application::ports::repository::{Page, Paginated, OrdersServicePort};
use crate::domain::entities::orders::Orders;
use crate::presentation::state::AppState;
use crate::shared::error::AppError;
use crate::shared::pagination::Pagination;
use crate::domain::entities::orders::OrderStatusEnum;


/// `GET /api/v1/orders` — paginated listing.
#[utoipa::path(
    get,
    path = "/api/v1/orders",
    tag = "orders",
    operation_id = "list_orders",
    params(Pagination),
    responses((status = 200, body = Paginated<Orders>))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders"))]
pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Paginated<Orders>>, AppError> {
    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);
    let result = state.orders_service.list(page).await?;
    Ok(Json(result))
}

/// `POST /api/v1/orders` — create.
#[utoipa::path(
    post,
    path = "/api/v1/orders",
    tag = "orders",
    operation_id = "create_orders",
    request_body = CreateOrdersInput,
    responses((status = 201, body = Orders), (status = 400), (status = 409))
)]
#[instrument(skip(state, input), fields(path = "/api/v1/orders"))]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateOrdersInput>,
) -> Result<(StatusCode, Json<Orders>), AppError> {
    let created = state.orders_service.create(input).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `GET /api/v1/orders/{id}` — fetch one.
#[utoipa::path(
    get,
    path = "/api/v1/orders/{id}",
    tag = "orders",
    operation_id = "get_orders",
    responses((status = 200, body = Orders), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/{id}"))]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Orders>, AppError> {
    state
        .orders_service
        .get(id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Orders {id} not found")))
}

/// `PUT /api/v1/orders/{id}` — partial update.
#[utoipa::path(
    put,
    path = "/api/v1/orders/{id}",
    tag = "orders",
    operation_id = "update_orders",
    request_body = UpdateOrdersInput,
    responses((status = 200, body = Orders), (status = 404), (status = 409))
)]
#[instrument(skip(state, input), fields(path = "/api/v1/orders/{id}"))]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateOrdersInput>,
) -> Result<Json<Orders>, AppError> {
    state
        .orders_service
        .update(id, input)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Orders {id} not found")))
}

/// `DELETE /api/v1/orders/{id}` — delete (soft when available).
#[utoipa::path(
    delete,
    path = "/api/v1/orders/{id}",
    tag = "orders",
    operation_id = "delete_orders",
    responses((status = 204), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/{id}"))]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if state.orders_service.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Orders {id} not found")))
    }
}

/// `GET /api/v1/orders/status/{status}` — list by status.
#[utoipa::path(
    get,
    path = "/api/v1/orders/status/{status}",
    tag = "orders",
    operation_id = "list_orders_by_status",
    responses((status = 200, body = Vec<Orders>))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/status/{status}"))]
pub async fn list_by_status(
    State(state): State<AppState>,
    Path(status): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Orders>>, AppError> {
    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);
    let status = status.parse::<OrderStatusEnum>().map_err(|e: String| AppError::BadRequest(e))?;
    let result = state.orders_service.list_by_status(status, page).await?;
    Ok(Json(result))
}

/// `PATCH /api/v1/orders/{id}/status` — change status.
#[utoipa::path(
    patch,
    path = "/api/v1/orders/{id}/status",
    tag = "orders",
    operation_id = "set_orders_status",
    request_body = SetOrdersStatusInput,
    responses((status = 200, body = Orders), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/{id}/status"))]
pub async fn set_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SetOrdersStatusInput>,
) -> Result<Json<Orders>, AppError> {
    state
        .orders_service
        .set_status(id, body.status)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Orders {id} not found")))
}

/// Body of `PATCH /orders/{id}/status`.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct SetOrdersStatusInput {
    /// New status value.
    pub status: OrderStatusEnum,
}

/// `DELETE /api/v1/orders/{id}/soft` — soft delete.
#[utoipa::path(
    delete,
    path = "/api/v1/orders/{id}/soft",
    tag = "orders",
    operation_id = "soft_delete_orders",
    responses((status = 204), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/{id}/soft"))]
pub async fn soft_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if state.orders_service.soft_delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Orders {id} not found")))
    }
}

/// `POST /api/v1/orders/{id}/restore` — restore.
#[utoipa::path(
    post,
    path = "/api/v1/orders/{id}/restore",
    tag = "orders",
    operation_id = "restore_orders",
    responses((status = 200, body = Orders), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/{id}/restore"))]
pub async fn restore(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Orders>, AppError> {
    state
        .orders_service
        .restore(id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Orders {id} not found or not deleted")))
}

/// `GET /api/v1/orders/by-user/{user_id}` — list by owner.
#[utoipa::path(
    get,
    path = "/api/v1/orders/by-user/{user_id}",
    tag = "orders",
    operation_id = "list_orders_by_user",
    responses((status = 200, body = Vec<Orders>))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/by-user/{user_id}"))]
pub async fn list_by_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Orders>>, AppError> {
    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);
    let result = state.orders_service.list_by_user(user_id, page).await?;
    Ok(Json(result))
}

/// `GET /api/v1/orders/created-after?after=…` — created since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/orders/created-after",
    tag = "orders",
    operation_id = "list_orders_created_after",
    responses((status = 200, body = Vec<Orders>))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/created-after"))]
pub async fn created_after(
    State(state): State<AppState>,
    Query(params): Query<OrdersCreatedAfterParams>,
) -> Result<Json<Vec<Orders>>, AppError> {
    let page = Page::new(1, 50);
    let result = state.orders_service.list_created_after(params.after, page).await?;
    Ok(Json(result))
}

/// Query parameters for `GET /orders/created-after`.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct OrdersCreatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}

/// `GET /api/v1/orders/updated-after?after=…` — updated since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/orders/updated-after",
    tag = "orders",
    operation_id = "list_orders_updated_after",
    responses((status = 200, body = Vec<Orders>))
)]
#[instrument(skip(state), fields(path = "/api/v1/orders/updated-after"))]
pub async fn updated_after(
    State(state): State<AppState>,
    Query(params): Query<OrdersUpdatedAfterParams>,
) -> Result<Json<Vec<Orders>>, AppError> {
    let page = Page::new(1, 50);
    let result = state.orders_service.list_updated_after(params.after, page).await?;
    Ok(Json(result))
}

/// Query parameters for `GET /orders/updated-after`.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct OrdersUpdatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}
