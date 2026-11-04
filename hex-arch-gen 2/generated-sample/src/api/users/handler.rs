//! Axum handlers for the `users` context.
//!
//! Every handler is a thin inbound adapter: it deserializes transport
//! types, dispatches through the enterprise buses and maps errors onto
//! HTTP status codes. No business logic lives here.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;
use crate::api::state::AppState;
use crate::api::users::dto::{SetUsersStatusInput, UsersCreatedAfterParams, UsersPageParams, UsersUpdatedAfterParams};
use crate::application::users_messaging::{CreateUsersCommand, DeleteUsersCommand, GetUsersQuery, ListUsersByStatusQuery, ListUsersCreatedAfterQuery, ListUsersQuery, ListUsersUpdatedAfterQuery, SetUsersStatusCommand, UpdateUsersCommand};
use crate::domain::users::entity::Users;
use crate::domain::users::input::{CreateUsersInput, UpdateUsersInput};
use crate::error::ApiError;
use crate::domain::users::enums::UserStatusEnum;

/// `GET /api/v1/users` — paginated listing (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "users",
    operation_id = "list_users",
    responses((status = 200, body = Vec<Users>))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<UsersPageParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = state.qry.ask(ListUsersQuery { page: params.to_page() }).await?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
        "page": result.page,
        "per_page": result.per_page,
    })))
}

/// `POST /api/v1/users` — create (command bus).
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "users",
    operation_id = "create_users",
    request_body = CreateUsersInput,
    responses((status = 201, body = Users), (status = 400), (status = 409))
)]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateUsersInput>,
) -> Result<(StatusCode, Json<Users>), ApiError> {
    let created = state.cmd.send(CreateUsersCommand { input }).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `GET /api/v1/users/{id}` — fetch one (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "users",
    operation_id = "get_users",
    responses((status = 200, body = Users), (status = 404))
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Users>, ApiError> {
    state
        .qry
        .ask(GetUsersQuery { id })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `PUT /api/v1/users/{id}` — partial update (command bus).
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    tag = "users",
    operation_id = "update_users",
    request_body = UpdateUsersInput,
    responses((status = 200, body = Users), (status = 404), (status = 409))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUsersInput>,
) -> Result<Json<Users>, ApiError> {
    state
        .cmd
        .send(UpdateUsersCommand { id, input })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `DELETE /api/v1/users/{id}` — hard delete (command bus).
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "users",
    operation_id = "delete_users",
    responses((status = 204), (status = 404))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    if state.cmd.send(DeleteUsersCommand { id }).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

/// `GET /api/v1/users/status/{status}` — list by status (query bus).
#[utoipa::path(
    get,
    path = "/api/v1/users/status/{status}",
    tag = "users",
    operation_id = "list_users_by_status",
    responses((status = 200, body = Vec<Users>))
)]
pub async fn list_by_status(
    State(state): State<AppState>,
    Path(status): Path<String>,
    Query(params): Query<UsersPageParams>,
) -> Result<Json<Vec<Users>>, ApiError> {
    let status = status.parse::<UserStatusEnum>().map_err(ApiError::BadRequest)?;
    let result = state
        .qry
        .ask(ListUsersByStatusQuery { status, page: params.to_page() })
        .await?;
    Ok(Json(result))
}

/// `PATCH /api/v1/users/{id}/status` — change status (command bus).
#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}/status",
    tag = "users",
    operation_id = "set_users_status",
    request_body = SetUsersStatusInput,
    responses((status = 200, body = Users), (status = 404))
)]
pub async fn set_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SetUsersStatusInput>,
) -> Result<Json<Users>, ApiError> {
    state
        .cmd
        .send(SetUsersStatusCommand { id, status: body.status })
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// `GET /api/v1/users/created-after` — created since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/users/created-after",
    tag = "users",
    operation_id = "list_users_created_after",
    responses((status = 200, body = Vec<Users>))
)]
pub async fn created_after(
    State(state): State<AppState>,
    Query(params): Query<UsersCreatedAfterParams>,
) -> Result<Json<Vec<Users>>, ApiError> {
    let result = state
        .qry
        .ask(ListUsersCreatedAfterQuery { ts: params.after, page: crate::enterprise::repository::Page::new(1, 50) })
        .await?;
    Ok(Json(result))
}

/// `GET /api/v1/users/updated-after` — updated since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/users/updated-after",
    tag = "users",
    operation_id = "list_users_updated_after",
    responses((status = 200, body = Vec<Users>))
)]
pub async fn updated_after(
    State(state): State<AppState>,
    Query(params): Query<UsersUpdatedAfterParams>,
) -> Result<Json<Vec<Users>>, ApiError> {
    let result = state
        .qry
        .ask(ListUsersUpdatedAfterQuery { ts: params.after, page: crate::enterprise::repository::Page::new(1, 50) })
        .await?;
    Ok(Json(result))
}
