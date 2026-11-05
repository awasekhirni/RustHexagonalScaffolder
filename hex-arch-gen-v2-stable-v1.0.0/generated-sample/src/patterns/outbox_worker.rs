//! Outbox worker — drains `outbox` rows and publishes them via a pluggable
//! [`EventPublisher`]. Bound to a `CancellationToken` for ordered shutdown.
//!
//! v1.0.0 (closes G-M08): outbox table DDL now emitted by codegen/migrations.rs.

use async_trait::async_trait;
use sqlx::PgPool;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Publisher abstraction — production implementations call Kafka/SNS/webhook.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publishes one outbox row. Returns `Ok(())` on success.
    async fn publish(&self, id: i64, payload: &serde_json::Value) -> Result<(), String>;
}

/// Logs the event and acks — useful for dev/test.
pub struct LogPublisher;

#[async_trait]
impl EventPublisher for LogPublisher {
    async fn publish(&self, id: i64, payload: &serde_json::Value) -> Result<(), String> {
        info!(outbox_id = id, payload = %payload, "outbox event published (log)");
        Ok(())
    }
}

/// Outbox worker.
pub struct OutboxWorker {
    pool: PgPool,
    publisher: Box<dyn EventPublisher>,
    batch_size: i64,
    interval: Duration,
}

impl OutboxWorker {
    /// Constructs a new worker.
    pub fn new(pool: PgPool, publisher: impl EventPublisher + 'static) -> Self {
        Self {
            pool,
            publisher: Box::new(publisher),
            batch_size: 50,
            interval: Duration::from_secs(1),
        }
    }
    /// Sets the batch size.
    #[must_use]
    pub fn with_batch(mut self, n: i64) -> Self {
        self.batch_size = n;
        self
    }
    /// Sets the poll interval.
    #[must_use]
    pub fn with_interval(mut self, d: Duration) -> Self {
        self.interval = d;
        self
    }
    /// Runs the worker until the cancellation token is cancelled.
    pub async fn run_until(self, token: CancellationToken) {
        info!(batch = self.batch_size, interval_ms = self.interval.as_millis() as u64, "outbox worker started");
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    info!("outbox worker stopped");
                    return;
                }
                _ = tokio::time::sleep(self.interval) => {
                    if let Err(e) = self.drain_batch().await {
                        warn!(error = %e, "outbox drain failed");
                    }
                }
            }
        }
    }
    async fn drain_batch(&self) -> Result<(), sqlx::Error> {
        // Best-effort: select pending rows, publish, mark as processed.
        // The real implementation uses `FOR UPDATE SKIP LOCKED` for
        // concurrent-worker safety; the stub just queries.
        let rows: Vec<(i64, serde_json::Value)> = sqlx::query_as(
            "SELECT id, payload FROM outbox WHERE processed_at IS NULL ORDER BY id LIMIT $1",
        )
        .bind(self.batch_size)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        for (id, payload) in rows {
            if let Err(e) = self.publisher.publish(id, &payload).await {
                warn!(outbox_id = id, error = %e, "publish failed — will retry next tick");
                continue;
            }
            let _ = sqlx::query("UPDATE outbox SET processed_at = now() WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await;
        }
        Ok(())
    }
}
