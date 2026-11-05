//! API-key authentication — Argon2id-hashed keys stored in `api_keys`.

use sqlx::PgPool;
use uuid::Uuid;

use crate::shared::error::AppError;

/// Verifies an API key and returns the owning user id.
pub async fn verify_api_key(pool: &PgPool, key: &str) -> Result<Uuid, AppError> {
    // Stub: production code hashes the key with Argon2id and looks up
    // the matching row. Here we just return a nil UUID when the key
    // is empty.
    if key.is_empty() {
        return Err(AppError::Unauthorized);
    }
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;
    user_id.ok_or(AppError::Unauthorized)
}
