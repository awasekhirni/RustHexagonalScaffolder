//! Outbox DLQ drainer — moves outbox rows that exceeded the retry budget
//! into a `outbox_dlq` table for manual inspection.

use sqlx::PgPool;
use std::time::Duration;
use tracing::info;

/// Runs the DLQ drainer until the future is dropped.
///
/// Stub: production code reads `attempts` and a max-retry threshold; this
/// stub just logs that it's running.
pub async fn run_dlq_drainer(pool: PgPool, interval: Duration) {
    info!(?interval, "DLQ drainer started");
    loop {
        tokio::time::sleep(interval).await;
        let _ = pool.acquire().await; // keep the pool warm
    }
}
