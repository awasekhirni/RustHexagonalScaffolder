//! Load shedder — rejects requests when the queue depth exceeds a threshold.

use std::sync::atomic::{AtomicI64, Ordering};

use crate::shared::error::AppError;

/// Load shedder — counts in-flight requests and rejects when over capacity.
pub struct LoadShedder {
    capacity: i64,
    current: AtomicI64,
}

impl LoadShedder {
    /// Constructs a shedder with the given capacity.
    #[must_use]
    pub fn new(capacity: i64) -> Self {
        Self {
            capacity,
            current: AtomicI64::new(0),
        }
    }

    /// Attempts to admit a request; returns `Ok(())` if admitted, `Err` if shed.
    pub fn admit(&self) -> Result<LoadShedPermit<'_>, AppError> {
        let prev = self.current.fetch_add(1, Ordering::SeqCst);
        if prev >= self.capacity {
            self.current.fetch_sub(1, Ordering::SeqCst);
            tracing::warn!(capacity = self.capacity, current = prev, "load shedder at capacity");
            return Err(AppError::Overloaded);
        }
        Ok(LoadShedPermit { shedder: self })
    }
}

/// RAII permit — decrements the counter on drop.
pub struct LoadShedPermit<'a> {
    shedder: &'a LoadShedder,
}

impl Drop for LoadShedPermit<'_> {
    fn drop(&mut self) {
        self.shedder.current.fetch_sub(1, Ordering::SeqCst);
    }
}
