//! Retry with exponential backoff + jitter.

use std::time::Duration;
use tracing::warn;

/// Retries `f` up to `max_attempts` times with exponential backoff.
///
/// Each attempt sleeps `base * 2^(attempt-1)` plus up to 25% jitter.
pub async fn retry_with_backoff<F, Fut, T, E>(
    max_attempts: usize,
    base: Duration,
    mut f: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt >= max_attempts => {
                warn!(attempt, error = %e, "retry budget exhausted");
                return Err(e);
            }
            Err(e) => {
                let backoff = base * 2u32.pow(attempt as u32 - 1);
                let jitter = Duration::from_millis(rand::random::<u64>() % (backoff.as_millis() as u64 / 4 + 1));
                let total = backoff + jitter;
                warn!(attempt, error = %e, ?total, "retrying after backoff");
                tokio::time::sleep(total).await;
            }
        }
    }
}
