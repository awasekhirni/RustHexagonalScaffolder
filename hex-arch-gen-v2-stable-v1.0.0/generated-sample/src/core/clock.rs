//! `Clock` — abstracted time source so tests can inject a fake clock.

use std::sync::Arc;

use chrono::{DateTime, Utc};

/// Clock trait — every use-case depends on this instead of `Utc::now()`.
pub trait Clock: Send + Sync {
    /// Returns the current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// Production clock — wraps `chrono::Utc::now()`.
#[derive(Debug, Clone, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Shared clock — `Arc<dyn Clock>` for handler injection.
pub type SharedClock = Arc<dyn Clock>;

/// Builds the default production clock.
#[must_use]
pub fn default_clock() -> SharedClock {
    Arc::new(SystemClock)
}
