//! Bulkhead — limits the number of concurrent operations to a bounded
//! semaphore, isolating failures so a slow dependency cannot exhaust the
//! whole pool.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use crate::shared::error::AppError;

/// Bulkhead — bounded concurrency for a single dependency.
pub struct Bulkhead {
    name: &'static str,
    semaphore: Arc<Semaphore>,
    acquire_timeout: Duration,
}

impl Bulkhead {
    /// Constructs a bulkhead with `max_concurrent` permits and an acquire timeout.
    #[must_use]
    pub fn new(name: &'static str, max_concurrent: usize, acquire_timeout: Duration) -> Self {
        Self {
            name,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            acquire_timeout,
        }
    }

    /// Acquires a permit; returns `Err(Overloaded)` when the timeout elapses.
    pub async fn acquire(&self) -> Result<tokio::sync::OwnedSemaphorePermit, AppError> {
        match tokio::time::timeout(self.acquire_timeout, self.semaphore.clone().acquire_owned()).await {
            Ok(Ok(permit)) => Ok(permit),
            _ => {
                tracing::warn!(bulkhead = self.name, "bulkhead exhausted — shedding load");
                Err(AppError::Overloaded)
            }
        }
    }
}
