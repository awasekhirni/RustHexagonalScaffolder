//! Axum handlers for the `users` context.
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
use crate::application::dto::users_dto::{CreateUsersInput, UpdateUsersInput};
use crate::application::ports::repository::{Page, Paginated, UsersServicePort};
use crate::domain::entities::users::Users;
use crate::presentation::state::AppState;
use crate::shared::error::AppError;
use crate::shared::pagination::Pagination;
use crate::domain::entities::users::UserStatusEnum;


/// `GET /api/v1/users` — paginated listing.
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "users",
    operation_id = "list_users",
    params(Pagination),
    responses((status = 200, body = Paginated<Users>))
)]
#[instrument(skip(state), fields(path = "/api/v1/users"))]
pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Paginated<Users>>, AppError> {
    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);
    let result = state.users_service.list(page).await?;
    Ok(Json(result))
}

/// `POST /api/v1/users` — create.
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "users",
    operation_id = "create_users",
    request_body = CreateUsersInput,
    responses((status = 201, body = Users), (status = 400), (status = 409))
)]
#[instrument(skip(state, input), fields(path = "/api/v1/users"))]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateUsersInput>,
) -> Result<(StatusCode, Json<Users>), AppError> {
    let created = state.users_service.create(input).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `GET /api/v1/users/{id}` — fetch one.
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "users",
    operation_id = "get_users",
    responses((status = 200, body = Users), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/users/{id}"))]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Users>, AppError> {
    state
        .users_service
        .get(id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Users {id} not found")))
}

/// `PUT /api/v1/users/{id}` — partial update.
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    tag = "users",
    operation_id = "update_users",
    request_body = UpdateUsersInput,
    responses((status = 200, body = Users), (status = 404), (status = 409))
)]
#[instrument(skip(state, input), fields(path = "/api/v1/users/{id}"))]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUsersInput>,
) -> Result<Json<Users>, AppError> {
    state
        .users_service
        .update(id, input)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Users {id} not found")))
}

/// `DELETE /api/v1/users/{id}` — delete (soft when available).
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "users",
    operation_id = "delete_users",
    responses((status = 204), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/users/{id}"))]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if state.users_service.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Users {id} not found")))
    }
}

/// `GET /api/v1/users/status/{status}` — list by status.
#[utoipa::path(
    get,
    path = "/api/v1/users/status/{status}",
    tag = "users",
    operation_id = "list_users_by_status",
    responses((status = 200, body = Vec<Users>))
)]
#[instrument(skip(state), fields(path = "/api/v1/users/status/{status}"))]
pub async fn list_by_status(
    State(state): State<AppState>,
    Path(status): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Users>>, AppError> {
    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);
    let status = status.parse::<UserStatusEnum>().map_err(|e: String| AppError::BadRequest(e))?;
    let result = state.users_service.list_by_status(status, page).await?;
    Ok(Json(result))
}

/// `PATCH /api/v1/users/{id}/status` — change status.
#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}/status",
    tag = "users",
    operation_id = "set_users_status",
    request_body = SetUsersStatusInput,
    responses((status = 200, body = Users), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/users/{id}/status"))]
pub async fn set_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SetUsersStatusInput>,
) -> Result<Json<Users>, AppError> {
    state
        .users_service
        .set_status(id, body.status)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Users {id} not found")))
}

/// Body of `PATCH /users/{id}/status`.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct SetUsersStatusInput {
    /// New status value.
    pub status: UserStatusEnum,
}

/// `DELETE /api/v1/users/{id}/soft` — soft delete.
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/soft",
    tag = "users",
    operation_id = "soft_delete_users",
    responses((status = 204), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/users/{id}/soft"))]
pub async fn soft_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if state.users_service.soft_delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Users {id} not found")))
    }
}

/// `POST /api/v1/users/{id}/restore` — restore.
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/restore",
    tag = "users",
    operation_id = "restore_users",
    responses((status = 200, body = Users), (status = 404))
)]
#[instrument(skip(state), fields(path = "/api/v1/users/{id}/restore"))]
pub async fn restore(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Users>, AppError> {
    state
        .users_service
        .restore(id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Users {id} not found or not deleted")))
}

/// `GET /api/v1/users/created-after?after=…` — created since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/users/created-after",
    tag = "users",
    operation_id = "list_users_created_after",
    responses((status = 200, body = Vec<Users>))
)]
#[instrument(skip(state), fields(path = "/api/v1/users/created-after"))]
pub async fn created_after(
    State(state): State<AppState>,
    Query(params): Query<UsersCreatedAfterParams>,
) -> Result<Json<Vec<Users>>, AppError> {
    let page = Page::new(1, 50);
    let result = state.users_service.list_created_after(params.after, page).await?;
    Ok(Json(result))
}

/// Query parameters for `GET /users/created-after`.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct UsersCreatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}

/// `GET /api/v1/users/updated-after?after=…` — updated since a timestamp.
#[utoipa::path(
    get,
    path = "/api/v1/users/updated-after",
    tag = "users",
    operation_id = "list_users_updated_after",
    responses((status = 200, body = Vec<Users>))
)]
#[instrument(skip(state), fields(path = "/api/v1/users/updated-after"))]
pub async fn updated_after(
    State(state): State<AppState>,
    Query(params): Query<UsersUpdatedAfterParams>,
) -> Result<Json<Vec<Users>>, AppError> {
    let page = Page::new(1, 50);
    let result = state.users_service.list_updated_after(params.after, page).await?;
    Ok(Json(result))
}

/// Query parameters for `GET /users/updated-after`.
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct UsersUpdatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}
