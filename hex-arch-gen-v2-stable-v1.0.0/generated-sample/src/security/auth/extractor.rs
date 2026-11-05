//! Auth extractor — pulls the principal from the JWT `Authorization` header.
//!
//! v0.4.0 (closes G-H01). The previous stub read `X-User-Id` and
//! `X-Role` headers set by an upstream gateway — a trust-the-gateway
//! model that fails open if the gateway is misconfigured or bypassed.
//! This extractor now verifies a real JWT signed by the local
//! `JwtService`, so the runtime never trusts an upstream claim it
//! cannot re-verify.
//!
//! The extractor depends on `AppState` to reach the shared
//! `JwtService`. Handlers that need the principal declare
//! `ReqAuth(auth): ReqAuth` in their signature; axum runs the
//! extractor before invoking the handler.

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use tracing::instrument;
use uuid::Uuid;

use crate::presentation::state::AppState;
use crate::security::auth::jwt::Claims;
use crate::shared::error::AppError;

/// The authenticated principal. Constructed only from a verified JWT.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User id from the JWT `sub` claim.
    pub user_id: Uuid,
    /// Tenant id from the JWT `tenant_id` claim (or `Uuid::nil()`).
    pub tenant_id: Option<String>,
    /// Role from the JWT `role` claim (or `"user"` if absent).
    pub role: String,
    /// All scopes from the JWT (access tokens only — refresh-scope
    /// tokens are rejected at verify time).
    pub scopes: Vec<String>,
}

impl AuthUser {
    /// Returns `true` when the principal has `scope`.
    #[must_use]
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// Returns `true` when the principal has `role`.
    #[must_use]
    pub fn has_role(&self, role: &str) -> bool {
        self.role == role
    }
}

/// Extractor wrapper — declare as `ReqAuth(auth): ReqAuth` in a
/// handler signature to require authentication.
pub struct ReqAuth(pub AuthUser);

#[async_trait]
impl FromRequestParts<AppState> for ReqAuth {
    type Rejection = AppError;

    #[instrument(skip(parts, state), fields(
        category = "security",
        function_id = "auth::extractor",
    ))]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let hdr = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!("missing Authorization header");
                AppError::Unauthorized
            })?;
        let token = hdr.strip_prefix("Bearer ").ok_or_else(|| {
            tracing::warn!("expected Bearer scheme");
            AppError::Unauthorized
        })?;
        let claims: Claims = state.jwt.verify(token)?;
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| AppError::Internal(format!("jwt sub not a uuid: {e}")))?;
        Ok(ReqAuth(AuthUser {
            user_id,
            tenant_id: claims.tenant_id,
            role: claims.role.unwrap_or_else(|| "user".into()),
            scopes: claims.scopes,
        }))
    }
}
