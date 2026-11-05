//! Native PostgreSQL enum types used by the `orders` table.
//!
//! Generated enums map to the database types through
//! `#[sqlx(type_name = …, rename_all = "snake_case")]` and to JSON through
//! serde with the identical casing, so labels round-trip losslessly.

use serde::{Deserialize, Serialize};
use sqlx::Type;
use utoipa::ToSchema;

/// Native PostgreSQL enum `order_status` (labels: pending, paid, shipped, delivered, cancelled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "order_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OrderStatusEnum {
    /// `pending`
    #[default]
    Pending,
    /// `paid`
    Paid,
    /// `shipped`
    Shipped,
    /// `delivered`
    Delivered,
    /// `cancelled`
    Cancelled,
}

impl std::str::FromStr for OrderStatusEnum {
    type Err = String;

    /// Parses a raw string (e.g. a path segment) into the enum.
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "pending" => Ok(Self::Pending),
            "paid" => Ok(Self::Paid),
            "shipped" => Ok(Self::Shipped),
            "delivered" => Ok(Self::Delivered),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(format!("unknown order_status value: {other}")),
        }
    }
}
