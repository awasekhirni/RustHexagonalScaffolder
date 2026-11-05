//! Refresh-token service — issues and rotates refresh tokens.
//!
//! v0.4.0 (closes G-H01). The refresh token is now a JWT itself
//! (carrying `scope=refresh`) rather than an opaque UUID stored in
//! a table — this simplifies horizontal scaling and removes the
//! dependency on a `refresh_tokens` table. Rotation happens on
//! every call to `refresh()`: the presented refresh token is
//! verified, a fresh `(access, refresh)` pair is issued, and the
//! caller is expected to discard the old refresh token. This is
//! the OAuth2-refresh-rotation pattern recommended by RFC 6749
//! §10.4.
//!
//! For deployments that want to revoke refresh tokens on logout,
//! a `revoked_jti` table keyed on the JWT `jti` claim is the
//! standard approach — out of scope for v0.4.0 but tracked as
//! G-H01-followup in the enhancement report.

use std::sync::Arc;
use uuid::Uuid;

use tracing::instrument;

use crate::security::auth::jwt::{Claims, JwtService};
use crate::shared::error::AppError;

/// Refresh-token service. Stateless — all state lives in the JWT
/// itself.
pub struct RefreshService {
    jwt: Arc<JwtService>,
}

impl RefreshService {
    /// Constructs a new service.
    #[must_use]
    pub fn new(jwt: Arc<JwtService>) -> Self {
        Self { jwt }
    }

    /// Issues a `(access, refresh)` pair for `user_id`. Wraps
    /// `JwtService::issue_refresh_pair` so callers do not need to
    /// import the JWT service directly.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` on encoding failure.
    #[instrument(skip(self), fields(
        category = "security",
        function_id = "refresh::issue",
        user = %user_id,
    ))]
    pub fn issue(
        &self,
        user_id: Uuid,
        tenant_id: Option<String>,
        role: Option<String>,
    ) -> Result<(String, String), AppError> {
        self.jwt.issue_refresh_pair(user_id, tenant_id, role)
    }

    /// Refreshes an access token. Verifies the presented refresh
    /// token, then issues a new `(access, refresh)` pair, rotating
    /// the refresh token on every use.
    ///
    /// # Errors
    ///
    /// * `AppError::Unauthorized` — refresh token is invalid,
    ///   expired, or has the wrong scope.
    #[instrument(skip(self, refresh_token), fields(
        category = "security",
        function_id = "refresh::refresh",
    ))]
    pub fn refresh(&self, refresh_token: &str) -> Result<(String, String), AppError> {
        let claims: Claims = self.jwt.verify_refresh(refresh_token)?;
        let user_id = uuid::Uuid::parse_str(&claims.sub)
            .map_err(|e| AppError::Internal(format!("jwt sub not a uuid: {e}")))?;
        // Re-issue with the same tenant and role — scope rotation
        // happens implicitly because issue_refresh_pair always
        // produces a refresh token with scope=refresh.
        self.jwt.issue_refresh_pair(user_id, claims.tenant_id, claims.role)
    }
}
