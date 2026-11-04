//! Native PostgreSQL enum types used by the `users` table.
//!
//! Generated enums map to the database types through
//! `#[sqlx(type_name = …, rename_all = "snake_case")]` and to JSON through
//! serde with the identical casing, so labels round-trip losslessly.

use serde::{Deserialize, Serialize};
use sqlx::Type;
use utoipa::ToSchema;

/// Native PostgreSQL enum `user_status` (labels: active, inactive, suspended).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "user_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserStatusEnum {
    /// `active`
    #[default]
    Active,
    /// `inactive`
    Inactive,
    /// `suspended`
    Suspended,
}

impl std::str::FromStr for UserStatusEnum {
    type Err = String;

    /// Parses a raw string (e.g. a path segment) into the enum.
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "suspended" => Ok(Self::Suspended),
            other => Err(format!("unknown user_status value: {other}")),
        }
    }
}
