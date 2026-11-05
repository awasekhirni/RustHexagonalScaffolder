//! Axum handlers for the `orders` context.
//!
//! Every handler is a thin inbound adapter: it deserializes transport
//! types, dispatches through the enterprise buses and maps errors onto
//! HTTP status codes. No business logic lives here.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;
use crate::api::state::AppState;
use crate::api::orders::dto::{OrdersCreatedAfterParams, OrdersPageParams, OrdersUpdatedAfterParams, SetOrdersStatusInput};
use crate::application::orders_messaging::{CreateOrdersCommand, DeleteOrdersCommand, GetOrdersQuery, ListOrdersByStatusQuery, ListOrdersByUserQuery, ListOrdersCreatedAfterQuery, ListOrdersQuery, ListOrdersUpdatedAfterQuery, RestoreOrdersCommand, SetOrdersStatusCommand, SoftDeleteOrdersCommand, UpdateOrdersCommand};
use crate::domain::orders::entity::Orders;
use crate::domain::orders::input::{CreateOrdersInput, UpdateOrdersInput};
use crate::error::ApiError;
use crate::domain::orders::enums::OrderStatusEnum;

/// `GET /api/v1/orders` — paginated listing (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/orders",
    tag = "orders",
    operation_id = "list_orders",
    responses((status = 200, body = Vec<Orders>))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<OrdersPageParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = state.qry.ask(ListOrdersQuery { page: params.to_page() }).await?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
        "page": result.page,
        "per_page": result.per_page,
    })))
}

/// `POST /api/v1/orders` — create (command bus).
#[utoipa::path(
    post,
    path = "/api/v1/orders",
    tag = "orders",
    operation_id = "create_orders",
    request_body = CreateOrdersInput,
    responses((status = 201, body = Orders), (status = 400), (status = 409))
)]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateOrdersInput>,
) -> Result<(StatusCode, Json<Orders>), ApiError> {
    let created = state.cmd.send(CreateOrdersCommand { input }).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `GET /api/v1/orders/{id}` — fetch one (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/orders/{id}",
    tag = "orders",
    operation_id = "get_orders",
    responses((status = 200, body = Orders), (status = 404))
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Orders>, ApiError> {
    state
        .qry
        .ask(GetOrdersQuery { id })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `PUT /api/v1/orders/{id}` — partial update (command bus).
#[utoipa::path(
    put,
    path = "/api/v1/orders/{id}",
    tag = "orders",
    operation_id = "update_orders",
    request_body = UpdateOrdersInput,
    responses((status = 200, body = Orders), (status = 404), (status = 409))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateOrdersInput>,
) -> Result<Json<Orders>, ApiError> {
    state
        .cmd
        .send(UpdateOrdersCommand { id, input })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `DELETE /api/v1/orders/{id}` — hard delete (command bus).
#[utoipa::path(
    delete,
    path = "/api/v1/orders/{id}",
    tag = "orders",
    operation_id = "delete_orders",
    responses((status = 204), (status = 404))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    if state.cmd.send(DeleteOrdersCommand { id }).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

/// `GET /api/v1/orders/status/{status}` — list by status (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/orders/status/{status}",
    tag = "orders",
    operation_id = "list_orders_by_status",
    responses((status = 200, body = Vec<Orders>))
)]
pub async fn list_by_status(
    State(state): State<AppState>,
    Path(status): Path<String>,
    Query(params): Query<OrdersPageParams>,
) -> Result<Json<Vec<Orders>>, ApiError> {
    let status = status.parse::<OrderStatusEnum>().map_err(ApiError::BadRequest)?;
    let result = state
        .qry
        .ask(ListOrdersByStatusQuery { status, page: params.to_page() })
        .await?;
    Ok(Json(result))
}

/// `PATCH /api/v1/orders/{id}/status` — change status (command bus).
#[utoipa::path(
    patch,
    path = "/api/v1/orders/{id}/status",
    tag = "orders",
    operation_id = "set_orders_status",
    request_body = SetOrdersStatusInput,
    responses((status = 200, body = Orders), (status = 404))
)]
pub async fn set_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SetOrdersStatusInput>,
) -> Result<Json<Orders>, ApiError> {
    state
        .cmd
        .send(SetOrdersStatusCommand { id, status: body.status })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `DELETE /api/v1/orders/{id}/soft` — soft delete (command bus).
#[utoipa::path(
    delete,
    path = "/api/v1/orders/{id}/soft",
    tag = "orders",
    operation_id = "soft_delete_orders",
    responses((status = 204), (status = 404))
)]
pub async fn soft_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    if state.cmd.send(SoftDeleteOrdersCommand { id }).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

/// `POST /api/v1/orders/{id}/restore` — restore (command bus).
#[utoipa::path(
    post,
    path = "/api/v1/orders/{id}/restore",
    tag = "orders",
    operation_id = "restore_orders",
    responses((status = 200, body = Orders), (status = 404))
)]
pub async fn restore(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Orders>, ApiError> {
    state
        .cmd
        .send(RestoreOrdersCommand { id })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `GET /api/v1/orders/by-user/{user_id}` — list by owner (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/orders/by-user/{user_id}",
    tag = "orders",
    operation_id = "list_orders_by_user",
    responses((status = 200, body = Vec<Orders>))
)]
pub async fn list_by_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(params): Query<OrdersPageParams>,
) -> Result<Json<Vec<Orders>>, ApiError> {
    let result = state
        .qry
        .ask(ListOrdersByUserQuery { user_id, page: params.to_page() })
        .await?;
    Ok(Json(result))
}

/// `GET /api/v1/orders/created-after` — created since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/orders/created-after",
    tag = "orders",
    operation_id = "list_orders_created_after",
    responses((status = 200, body = Vec<Orders>))
)]
pub async fn created_after(
    State(state): State<AppState>,
    Query(params): Query<OrdersCreatedAfterParams>,
) -> Result<Json<Vec<Orders>>, ApiError> {
    let result = state
        .qry
        .ask(ListOrdersCreatedAfterQuery { ts: params.after, page: crate::enterprise::repository::Page::new(1, 50) })
        .await?;
    Ok(Json(result))
}

/// `GET /api/v1/orders/updated-after` — updated since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/orders/updated-after",
    tag = "orders",
    operation_id = "list_orders_updated_after",
    responses((status = 200, body = Vec<Orders>))
)]
pub async fn updated_after(
    State(state): State<AppState>,
    Query(params): Query<OrdersUpdatedAfterParams>,
) -> Result<Json<Vec<Orders>>, ApiError> {
    let result = state
        .qry
        .ask(ListOrdersUpdatedAfterQuery { ts: params.after, page: crate::enterprise::repository::Page::new(1, 50) })
        .await?;
    Ok(Json(result))
}
