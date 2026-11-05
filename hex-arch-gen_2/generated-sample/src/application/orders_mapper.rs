//! Mapper for `orders` — translates input models into domain entities.
//!
//! Columns the database generates are left at their default here; the
//! repository INSERT simply omits them and `RETURNING *` hands the real
//! values back.

use crate::domain::orders::entity::Orders;
use crate::domain::orders::input::{CreateOrdersInput, UpdateOrdersInput};

/// Input ↔ entity translation for `orders`.
pub struct OrdersMapper;

impl OrdersMapper {
    /// Builds a fresh entity from a validated create payload.
    pub fn to_entity(input: CreateOrdersInput) -> Orders {
        Orders {
            user_id: input.user_id,
            amount: input.amount,
            notes: input.notes,
            deleted_at: input.deleted_at,
            ..Default::default()
        }
    }

    /// Applies a partial update onto the loaded entity — only fields
    /// present in the payload are overwritten.
    pub fn apply_update(mut current: Orders, input: &UpdateOrdersInput) -> Orders {
        if let Some(v) = &input.user_id {
            current.user_id = v.clone();
        }
        if let Some(v) = &input.amount {
            current.amount = v.clone();
        }
        if let Some(v) = &input.order_status {
            current.order_status = v.clone();
        }
        if let Some(v) = &input.notes {
            current.notes = Some(v.clone());
        }
        current
    }
}
