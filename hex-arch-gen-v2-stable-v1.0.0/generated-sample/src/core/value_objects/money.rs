//! `Money` — fixed-point monetary value backed by `rust_decimal::Decimal`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A monetary amount. Serialises as a JSON string (rust_decimal default)
/// so floating-point precision is never lost in transit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(value_type = String)]
pub struct Money(pub Decimal);

impl Money {
    /// Constructs a new monetary value.
    #[must_use]
    pub fn new(amount: Decimal) -> Self {
        Self(amount)
    }
    /// Returns the inner amount.
    #[must_use]
    pub fn amount(&self) -> Decimal {
        self.0
    }
}

impl From<Decimal> for Money {
    fn from(d: Decimal) -> Self {
        Self(d)
    }
}

impl From<Money> for Decimal {
    fn from(m: Money) -> Self {
        m.0
    }
}
