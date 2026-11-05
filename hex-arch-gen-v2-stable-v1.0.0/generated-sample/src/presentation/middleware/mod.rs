//! HTTP middleware — request id, logging, rate limit, in-flight tracker,
//! content type, deprecation, security headers.

pub mod content_type;
pub mod deprecation;
pub mod inflight;
pub mod logging;
pub mod rate_limit;
pub mod request_id;

pub use rate_limit::build_limiter;

// Re-export the security_headers middleware from the security module.
pub use crate::security::headers::security_headers;
