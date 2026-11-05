//! Cross-cutting types shared by every layer.
//!
//! These types are intentionally framework-agnostic — they don't depend on
//! axum, sqlx, or utoipa — so the domain layer can use them without pulling
//! in transport concerns.

pub mod error;
pub mod pagination;
pub mod result;

pub use error::AppError;
pub use pagination::Pagination;
pub use result::AppResult;
