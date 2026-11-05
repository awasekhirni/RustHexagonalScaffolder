//! Stable error code registry — every [`crate::shared::error::AppError`]
//! carries a code from this module so clients can branch deterministically.
//!
//! Codes are grouped by HTTP family: `4xx_client_*`, `5xx_server_*`.

/// 4xx — client errors.
pub mod client {
    pub const BAD_REQUEST: &str = "bad_request";
    pub const UNAUTHORIZED: &str = "unauthorized";
    pub const FORBIDDEN: &str = "forbidden";
    pub const NOT_FOUND: &str = "not_found";
    pub const CONFLICT: &str = "conflict";
    pub const PRECONDITION_FAILED: &str = "precondition_failed";
    pub const VALIDATION_FAILED: &str = "validation_failed";
    pub const RATE_LIMITED: &str = "rate_limited";
}

/// 5xx — server errors.
pub mod server {
    pub const INTERNAL_ERROR: &str = "internal_error";
    pub const OVERLOADED: &str = "overloaded";
    pub const DATABASE: &str = "database_error";
}
