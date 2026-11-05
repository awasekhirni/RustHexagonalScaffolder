//! PostgreSQL pool construction.

use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Builds a pool from the given URL and limits.
///
/// # Errors
///
/// Propagates SQLx connection errors.
pub async fn build_pool(url: &str, max: u32, min: u32, acquire_timeout: u64) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max)
        .min_connections(min)
        .acquire_timeout(Duration::from_secs(acquire_timeout))
        .connect(url)
        .await
}
