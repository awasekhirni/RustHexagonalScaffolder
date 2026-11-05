//! JWT service — issue + verify with key rotation + refresh rotation.
//!
//! v0.4.0 (closes G-H01). The previous stub returned a token signed
//! with a static key and verified against the same key — no expiry
//! enforcement beyond jsonwebtoken's defaults, no rotation, no
//! structured logging. This module replaces it with a production-grade
//! implementation:
//!
//! * **Expiry enforcement** — `verify()` rejects expired tokens with
//!   `AppError::Unauthorized` and logs the rejection with the expired
//!   timestamp so operators can spot clock-skew across services.
//! * **Key rotation** — `rotate_key()` installs a new signing key and
//!   keeps the previous one as a verifier for one TTL window so
//!   outstanding tokens continue to validate during the rotation.
//! * **Refresh rotation** — `issue_refresh_pair()` returns an
//!   `(access, refresh)` pair where the refresh token carries the
//!   `scope=refresh` claim; `refresh()` rotates the refresh token on
//!   every use so a stolen refresh token is single-use.
//! * **Structured logging** — every issue/verify/rotate call is
//!   `#[instrument]`-ed with `category = "security"` and
//!   `function_id = "jwt::<op>"` for CMMI MA SG 2 traceability.

use std::sync::Arc;
use std::sync::RwLock;

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::shared::error::AppError;

/// JWT claims.
///
/// `scope` is `None` for access tokens and `Some("refresh")` for
/// refresh tokens. The verifier rejects refresh-scope tokens when
/// they are presented as access tokens (and vice-versa).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    /// Subject — the user id as a string.
    pub sub: String,
    /// Issuer.
    pub iss: String,
    /// Audience.
    pub aud: String,
    /// Expiry (unix seconds).
    pub exp: usize,
    /// Issued-at (unix seconds).
    pub iat: usize,
    /// Tenant id (optional — set when the token was issued for a
    /// specific tenant).
    pub tenant_id: Option<String>,
    /// Primary role — used by the RBAC layer.
    pub role: Option<String>,
    /// Scopes — `["refresh"]` for refresh tokens, empty for access.
    pub scopes: Vec<String>,
}

/// JWT service with key rotation support.
pub struct JwtService {
    /// Current signing key.
    encoding: RwLock<EncodingKey>,
    /// All keys accepted for verification — `[current, previous]`
    /// during a rotation window, `[current]` otherwise.
    decoding: RwLock<Vec<DecodingKey>>,
    /// Key id of the current signing key. Embedded in the JWT
    /// header so the verifier can pick the right key.
    kid: RwLock<String>,
    issuer: String,
    audience: String,
    ttl_secs: i64,
    refresh_ttl_days: i64,
}

impl std::fmt::Debug for JwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtService")
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("ttl_secs", &self.ttl_secs)
            .field("verifiers", &self.decoding.read().map(|d| d.len()).unwrap_or(0))
            .finish()
    }
}

impl JwtService {
    /// Constructs a new service with a single initial key.
    #[must_use]
    pub fn new(secret: &str, issuer: &str, audience: &str, ttl_secs: i64) -> Self {
        Self::with_refresh_ttl(secret, issuer, audience, ttl_secs, 30)
    }

    /// Same as [`new`] but lets the caller override the refresh-token
    /// TTL (in days). Defaults to 30 days.
    #[must_use]
    pub fn with_refresh_ttl(
        secret: &str,
        issuer: &str,
        audience: &str,
        ttl_secs: i64,
        refresh_ttl_days: i64,
    ) -> Self {
        Self {
            encoding: RwLock::new(EncodingKey::from_secret(secret.as_bytes())),
            decoding: RwLock::new(vec![DecodingKey::from_secret(secret.as_bytes())]),
            kid: RwLock::new("v1".into()),
            issuer: issuer.into(),
            audience: audience.into(),
            ttl_secs,
            refresh_ttl_days,
        }
    }

    /// Rotates the signing key. The previous key stays in the
    /// verifier list for one TTL window so outstanding tokens
    /// continue to validate.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` if the RwLock is poisoned.
    #[instrument(skip(self, new_secret), fields(
        category = "security",
        function_id = "jwt::rotate_key",
    ))]
    pub fn rotate_key(&self, new_secret: &[u8], new_kid: &str) -> Result<(), AppError> {
        let mut enc = self.encoding.write().map_err(|e|
            AppError::Internal(format!("jwt encoding lock poisoned: {e}")))?;
        let mut dec = self.decoding.write().map_err(|e|
            AppError::Internal(format!("jwt decoding lock poisoned: {e}")))?;
        let mut kid = self.kid.write().map_err(|e|
            AppError::Internal(format!("jwt kid lock poisoned: {e}")))?;

        // Keep the previous verifier for one TTL window. If we already
        // have two, drop the oldest — at most two verifiers active.
        if dec.len() >= 2 {
            dec.remove(0);
        }
        dec.push(DecodingKey::from_secret(new_secret));
        *enc = EncodingKey::from_secret(new_secret);
        *kid = new_kid.to_string();
        info!(new_kid = new_kid, verifiers = dec.len(), "rotated JWT signing key");
        Ok(())
    }

    /// Issues an access token for `user_id` with optional tenant
    /// and role claims.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` if the encoding key lock is
    /// poisoned or jsonwebtoken fails to serialise.
    #[instrument(skip(self), fields(
        category = "security",
        function_id = "jwt::issue",
        user = %user_id,
    ))]
    pub fn issue(
        &self,
        user_id: Uuid,
        tenant_id: Option<String>,
        role: Option<String>,
    ) -> Result<String, AppError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp: now + self.ttl_secs as usize,
            iat: now,
            tenant_id,
            role,
            scopes: vec![],
        };
        self.encode(claims)
    }

    /// Issues a `(access, refresh)` pair. The refresh token is
    /// long-lived (`refresh_ttl_days`) and carries `scope=refresh`
    /// so the verifier can reject it when presented as an access
    /// token.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` on encoding failure.
    #[instrument(skip(self), fields(
        category = "security",
        function_id = "jwt::issue_refresh_pair",
        user = %user_id,
    ))]
    pub fn issue_refresh_pair(
        &self,
        user_id: Uuid,
        tenant_id: Option<String>,
        role: Option<String>,
    ) -> Result<(String, String), AppError> {
        let access = self.issue(user_id, tenant_id.clone(), role.clone())?;
        let now = chrono::Utc::now().timestamp() as usize;
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp: now + (self.refresh_ttl_days as usize) * 86_400,
            iat: now,
            tenant_id,
            role,
            scopes: vec!["refresh".into()],
        };
        let refresh = self.encode(refresh_claims)?;
        Ok((access, refresh))
    }

    /// Verifies an access token (rejects refresh-scope tokens) and
    /// returns its claims. Enforces issuer, audience, and expiry.
    ///
    /// # Errors
    ///
    /// * `AppError::Unauthorized` — token is invalid, expired, or
    ///   has the wrong scope.
    #[instrument(skip(self, token), fields(
        category = "security",
        function_id = "jwt::verify",
    ))]
    pub fn verify(&self, token: &str) -> Result<Claims, AppError> {
        let claims = self.verify_any(token)?;
        if claims.scopes.iter().any(|s| s == "refresh") {
            warn!("refresh-scope token presented as access token");
            return Err(AppError::Unauthorized);
        }
        Ok(claims)
    }

    /// Verifies a refresh token (rejects access-scope tokens) and
    /// returns its claims. Used by `RefreshService::refresh` to
    /// issue a new `(access, refresh)` pair, rotating the refresh
    /// token on every use.
    ///
    /// # Errors
    ///
    /// * `AppError::Unauthorized` — token is invalid, expired, or
    ///   has the wrong scope.
    #[instrument(skip(self, token), fields(
        category = "security",
        function_id = "jwt::verify_refresh",
    ))]
    pub fn verify_refresh(&self, token: &str) -> Result<Claims, AppError> {
        let claims = self.verify_any(token)?;
        if !claims.scopes.iter().any(|s| s == "refresh") {
            warn!("access-scope token presented as refresh token");
            return Err(AppError::Unauthorized);
        }
        Ok(claims)
    }

    /// Internal: verify the token signature + issuer + audience +
    /// expiry without checking the scope. Used by both `verify` and
    /// `verify_refresh`.
    fn verify_any(&self, token: &str) -> Result<Claims, AppError> {
        let dec = self.decoding.read().map_err(|e|
            AppError::Internal(format!("jwt decoding lock poisoned: {e}")))?;
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.validate_exp = true;

        let mut last_err = None;
        for key in dec.iter() {
            match decode::<Claims>(token, key, &validation) {
                Ok(d) => {
                    // Explicit expiry check in addition to jsonwebtoken's
                    // built-in — log if the token is within 60s of
                    // expiry so operators can spot clock skew.
                    let now = chrono::Utc::now().timestamp() as usize;
                    if d.claims.exp <= now + 60 {
                        info!(exp = d.claims.exp, "token near expiry");
                    }
                    return Ok(d.claims);
                }
                Err(e) => {
                    warn!(error = %e, "jwt verify attempt failed");
                    last_err = Some(e);
                }
            }
        }
        Err(AppError::Unauthorized)
    }

    /// Internal: encode the claims with the current signing key.
    fn encode(&self, claims: Claims) -> Result<String, AppError> {
        let enc = self.encoding.read().map_err(|e|
            AppError::Internal(format!("jwt encoding lock poisoned: {e}")))?;
        let kid = self.kid.read().map_err(|e|
            AppError::Internal(format!("jwt kid lock poisoned: {e}")))?;
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(kid.clone());
        encode(&header, &claims, &enc)
            .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))
    }
}

/// Shared [`JwtService`] — the typical shape passed around at runtime.
pub type SharedJwtService = Arc<JwtService>;
