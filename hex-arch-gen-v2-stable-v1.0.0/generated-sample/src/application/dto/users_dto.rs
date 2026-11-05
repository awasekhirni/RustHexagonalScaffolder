//! Request/response DTOs for the `Users` context.
//!
//! `CreateUsersInput` covers every client-writable column of an INSERT
//! (server-generated columns are filled by the database); `UpdateUsersInput`
//! is fully optional so `PUT` behaves as a partial update.

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use serde::Deserialize;
use utoipa::ToSchema;
use crate::domain::entities::users::UserStatusEnum;

/// Payload accepted by `POST /users` — validated before persistence.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUsersInput {
    /// Column `email`.
    pub email: String,
    /// Column `full_name`.
    pub full_name: Option<String>,
    /// Column `metadata`.
    pub metadata: Option<JsonValue>,
    /// Column `tags`.
    pub tags: Option<Vec<String>>,
    /// Column `deleted_at`.
    pub deleted_at: Option<DateTime<Utc>>,
}

impl CreateUsersInput {
    /// Bean-style validation invoked by the application service.
    pub fn validate(&self) -> Result<(), String> {
        if self.email.trim().is_empty() {
            return Err("email must not be empty".to_string());
        }
        Ok(())
    }
}

/// Payload accepted by `PUT /users/{id}` — every field optional.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateUsersInput {
    /// Optional new value for `email`.
    pub email: Option<String>,
    /// Optional new value for `full_name`.
    pub full_name: Option<String>,
    /// Optional new value for `user_status`.
    pub user_status: Option<UserStatusEnum>,
    /// Optional new value for `is_active`.
    pub is_active: Option<bool>,
    /// Optional new value for `metadata`.
    pub metadata: Option<JsonValue>,
    /// Optional new value for `tags`.
    pub tags: Option<Vec<String>>,
}
