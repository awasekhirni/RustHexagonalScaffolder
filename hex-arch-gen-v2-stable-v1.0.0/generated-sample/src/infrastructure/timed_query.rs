//! `timed_query` — wraps a SQLx future and records its duration in the
//! tracing log.

use std::time::Instant;
use tracing::info;

/// Wraps an async future, logging the elapsed time when it completes.
pub async fn timed<F, Fut, T>(label: &str, f: F) -> Result<T, sqlx::Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    let start = Instant::now();
    let result = f().await;
    info!(label, elapsed_ms = start.elapsed().as_millis(), "timed query");
    result
}
