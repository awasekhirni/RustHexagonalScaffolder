//! Result alias for the application — `Result<T, AppError>`.

use crate::shared::error::AppError;

/// Convenience alias used throughout the application and presentation layers.
pub type AppResult<T> = std::result::Result<T, AppError>;
