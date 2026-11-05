//! Query-string filtering parser — translates `?field[op]=value` into a
//! safe SQL fragment (parameterised, never concatenated).
//!
//! Stub: production code uses `url::Url` + a typed field map per entity;
//! here we expose a placeholder enum so handlers can declare the surface.

use serde_json::Value;

/// Filter operators understood by the parser.
#[derive(Debug, Clone, Copy)]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    In,
}

/// One parsed filter.
#[derive(Debug, Clone)]
pub struct Filter {
    pub field: String,
    pub op: FilterOp,
    pub value: Value,
}
