//! Ordered graceful shutdown — 12-factor disposability.
//!
//! A `ShutdownCoordinator` owns three cancellation tokens:
//!   * `http_token`     — cancelled first, stops accepting new HTTP requests.
//!   * `outbox_token`   — cancelled second, stops the outbox/DLQ/retention workers.
//!   * `retention_token`— cancelled third (same as outbox in this stub).
//!
//! `InFlightTracker` counts in-flight requests so the HTTP drain actually
//! waits for responses to finish instead of cutting them off.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::shared::error::AppError;

/// Tracks the number of in-flight HTTP requests for graceful drain.
#[derive(Debug, Default)]
pub struct InFlightTracker {
    current: AtomicI64,
}

impl InFlightTracker {
    /// Constructs a new tracker starting at zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Increments the in-flight counter.
    pub fn inc(&self) {
        self.current.fetch_add(1, Ordering::SeqCst);
    }
    /// Decrements the in-flight counter.
    pub fn dec(&self) {
        self.current.fetch_sub(1, Ordering::SeqCst);
    }
    /// Returns the current count.
    #[must_use]
    pub fn current(&self) -> i64 {
        self.current.load(Ordering::SeqCst)
    }
    /// v1.0.0 (closes G-M18): alias for `current()` used by the
    /// shutdown drain log line so the operator knows whether
    /// inflight requests were drained or force-dropped.
    #[must_use]
    pub fn inflight_count(&self) -> i64 {
        self.current()
    }
}

/// Coordinator for ordered graceful shutdown.
pub struct ShutdownCoordinator {
    /// HTTP listener token — cancelled first.
    pub http_token: CancellationToken,
    /// Outbox worker token — cancelled second.
    pub outbox_token: CancellationToken,
    /// Retention worker token — cancelled third.
    pub retention_token: CancellationToken,
}

impl ShutdownCoordinator {
    /// Constructs a new coordinator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            http_token: CancellationToken::new(),
            outbox_token: CancellationToken::new(),
            retention_token: CancellationToken::new(),
        }
    }

    /// Initiates the ordered shutdown sequence.
    ///
    /// 1. Cancel HTTP listener.
    /// 2. Wait for in-flight requests to drain (max 30s).
    /// 3. Cancel outbox workers.
    /// 4. Cancel retention workers.
    pub async fn shutdown(&self, inflight: &InFlightTracker) {
        warn!("shutdown: cancelling HTTP listener");
        self.http_token.cancel();

        // Drain in-flight requests up to 30s.
        let deadline = Duration::from_secs(30);
        let start = std::time::Instant::now();
        while inflight.current() > 0 && start.elapsed() < deadline {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        info!(remaining = inflight.current(), "shutdown: HTTP drain complete");

        warn!("shutdown: cancelling outbox workers");
        self.outbox_token.cancel();

        warn!("shutdown: cancelling retention workers");
        self.retention_token.cancel();
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type alias for shutdown operations.
pub type ShutdownResult = Result<(), AppError>;
