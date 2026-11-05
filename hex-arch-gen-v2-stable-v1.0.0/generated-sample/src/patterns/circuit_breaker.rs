//! Circuit breaker — protects against cascading failures by tripping open
//! after N consecutive errors.
//!
//! v0.4.0 (closes G-H07): adds `#[instrument]` to every state
//! transition so breaker operation is observable in the log stream
//! (CMMI MA SG 2). The impl itself was already functional in v0.3;
//! the gap was the missing observability.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, instrument, warn};

/// Breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Simple circuit breaker.
pub struct CircuitBreaker {
    failure_threshold: usize,
    reset_timeout: Duration,
    state: Mutex<BreakerState>,
    failures: AtomicUsize,
    opened_at: Mutex<Option<Instant>>,
}

impl CircuitBreaker {
    /// Constructs a new breaker with the given threshold and reset timeout.
    #[must_use]
    pub fn new(failure_threshold: usize, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            state: Mutex::new(BreakerState::Closed),
            failures: AtomicUsize::new(0),
            opened_at: Mutex::new(None),
        }
    }

    /// Returns `true` when the breaker allows a call through.
    #[instrument(skip(self), fields(
        category = "runtime",
        function_id = "circuit_breaker::allow",
    ))]
    pub async fn allow(&self) -> bool {
        let mut state = self.state.lock().await;
        match *state {
            BreakerState::Closed => true,
            BreakerState::Open => {
                let opened_at = self.opened_at.lock().await;
                if let Some(at) = *opened_at {
                    if at.elapsed() >= self.reset_timeout {
                        *state = BreakerState::HalfOpen;
                        info!("breaker transitioned Open -> HalfOpen");
                        return true;
                    }
                }
                warn!("breaker Open — call rejected");
                false
            }
            BreakerState::HalfOpen => true,
        }
    }

    /// Records a successful call.
    #[instrument(skip(self), fields(
        category = "runtime",
        function_id = "circuit_breaker::on_success",
    ))]
    pub async fn on_success(&self) {
        self.failures.store(0, Ordering::SeqCst);
        let mut state = self.state.lock().await;
        if *state != BreakerState::Closed {
            info!("breaker transitioned -> Closed (success)");
        }
        *state = BreakerState::Closed;
    }

    /// Records a failed call.
    #[instrument(skip(self), fields(
        category = "runtime",
        function_id = "circuit_breaker::on_error",
    ))]
    pub async fn on_error(&self) {
        let count = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
        if count >= self.failure_threshold {
            let mut state = self.state.lock().await;
            *state = BreakerState::Open;
            let mut opened_at = self.opened_at.lock().await;
            *opened_at = Some(Instant::now());
            warn!(failures = count, threshold = self.failure_threshold,
                  "breaker tripped Open");
        }
    }
}
