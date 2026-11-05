//! Mapper for `users` — translates input models into domain entities.
//!
//! Columns the database generates are left at their default here; the
//! repository INSERT simply omits them and `RETURNING *` hands the real
//! values back.

use crate::domain::users::entity::Users;
use crate::domain::users::input::{CreateUsersInput, UpdateUsersInput};

/// Input ↔ entity translation for `users`.
pub struct UsersMapper;

impl UsersMapper {
    /// Builds a fresh entity from a validated create payload.
    pub fn to_entity(input: CreateUsersInput) -> Users {
        Users {
            email: input.email,
            full_name: input.full_name,
            metadata: input.metadata,
            tags: input.tags,
            ..Default::default()
        }
    }

    /// Applies a partial update onto the loaded entity — only fields
    /// present in the payload are overwritten.
    pub fn apply_update(mut current: Users, input: &UpdateUsersInput) -> Users {
        if let Some(v) = &input.email {
            current.email = v.clone();
        }
        if let Some(v) = &input.full_name {
            current.full_name = Some(v.clone());
        }
        if let Some(v) = &input.user_status {
            current.user_status = v.clone();
        }
        if let Some(v) = &input.is_active {
            current.is_active = v.clone();
        }
        if let Some(v) = &input.metadata {
            current.metadata = Some(v.clone());
        }
        if let Some(v) = &input.tags {
            current.tags = Some(v.clone());
        }
        current
    }
}
