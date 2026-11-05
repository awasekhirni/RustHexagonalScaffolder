//! Query/body parameter types for the `users` HTTP surface.

use serde::Deserialize;
use utoipa::ToSchema;
use chrono::{DateTime, Utc};
use crate::domain::users::enums::UserStatusEnum;
use crate::enterprise::repository::Page;

/// Pagination query parameters for `users` endpoints.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UsersPageParams {
    /// 1-based page number (default 1).
    pub page: Option<i64>,
    /// Page size, clamped to 1..=200 (default 20).
    pub per_page: Option<i64>,
}

impl UsersPageParams {
    /// Converts loose query parameters into an enterprise [`Page`].
    pub fn to_page(&self) -> Page {
        Page::new(self.page.unwrap_or(1), self.per_page.unwrap_or(20))
    }
}

/// Body of `PATCH /users/{id}/status`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetUsersStatusInput {
    /// New status value.
    pub status: UserStatusEnum,
}

/// Query parameters for `GET /users/created-after`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UsersCreatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}

/// Query parameters for `GET /users/updated-after`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UsersUpdatedAfterParams {
    /// Inclusive ISO-8601 lower bound.
    pub after: DateTime<Utc>,
}
