//! Request/response DTOs for the `Orders` context.
//!
//! `CreateOrdersInput` covers every client-writable column of an INSERT
//! (server-generated columns are filled by the database); `UpdateOrdersInput`
//! is fully optional so `PUT` behaves as a partial update.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;
use serde::Deserialize;
use utoipa::ToSchema;
use crate::domain::entities::orders::OrderStatusEnum;

/// Payload accepted by `POST /orders` — validated before persistence.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateOrdersInput {
    /// Column `user_id`.
    pub user_id: Uuid,
    /// Column `tenant_id`.
    pub tenant_id: Uuid,
    /// Column `amount`.
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// Column `notes`.
    pub notes: Option<String>,
    /// Column `deleted_at`.
    pub deleted_at: Option<DateTime<Utc>>,
}

impl CreateOrdersInput {
    /// Bean-style validation invoked by the application service.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Payload accepted by `PUT /orders/{id}` — every field optional.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateOrdersInput {
    /// Optional new value for `user_id`.
    pub user_id: Option<Uuid>,
    /// Optional new value for `tenant_id`.
    pub tenant_id: Option<Uuid>,
    /// Optional new value for `amount`.
    #[schema(value_type = String)]
    pub amount: Option<Decimal>,
    /// Optional new value for `order_status`.
    pub order_status: Option<OrderStatusEnum>,
    /// Optional new value for `notes`.
    pub notes: Option<String>,
}
