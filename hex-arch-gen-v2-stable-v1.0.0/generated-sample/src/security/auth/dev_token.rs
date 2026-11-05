//! Dev-token helper — issues a fixed JWT for development.
//!
//! Disabled in production by construction (`APP_ENV=production`).
//!
//! v0.4.0 (closes G-H01): updated to use the new 3-arg
//! `JwtService::issue` signature.

use uuid::Uuid;

use crate::security::auth::jwt::JwtService;
use crate::shared::error::AppError;

/// Issues a dev token for `user_id`.
pub fn dev_token(jwt: &JwtService, user_id: Uuid) -> Result<String, AppError> {
    let production = std::env::var("APP_ENV")
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false);
    if production {
        return Err(AppError::Internal(
            "dev_token refused in production".into(),
        ));
    }
    // No tenant, default role. v0.5 will let the caller override these.
    jwt.issue(user_id, None, Some("dev".into()))
}
