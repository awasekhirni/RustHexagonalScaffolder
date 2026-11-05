//! N+1 query detector — warns when the same lookup is called
//! repeatedly within a single request.
//!
//! v0.4.0 (closes G-H06). Wraps the repository port and logs a
//! warning whenever the same `find_by_*` method is called more
//! than `THRESHOLD` times within a single request span. The
//! detector is opt-in: repository code calls `record()` inside
//! every `find_by_*` method.
//!
//! # CMMI mapping
//!
//! * **CAR SG 1** — Determine root causes of defects: N+1 is the
//!   most common latency defect in ORM-backed APIs; this detector
//!   surfaces it at the first occurrence rather than waiting for a
//!   production latency incident.

use std::collections::HashMap;
use std::sync::RwLock;

use tracing::warn;

/// Number of repeated calls within a span before we log a warning.
const THRESHOLD: u32 = 10;

/// Process-wide N+1 detector.
pub struct NPlusOneDetector {
    /// `span_id -> (method_name -> call_count)`
    inner: RwLock<HashMap<String, HashMap<String, u32>>>,
}

impl NPlusOneDetector {
    /// Constructs a new empty detector.
    #[must_use]
    pub fn new() -> Self {
        Self { inner: RwLock::new(HashMap::new()) }
    }

    /// Records a call to `method` within `span_id`. When the call
    /// count for that `(span_id, method)` pair exceeds
    /// `THRESHOLD`, logs a warning with the count and the method
    /// name so the operator can locate the N+1.
    pub fn record(&self, span_id: &str, method: &str) {
        let mut inner = self.inner.write().expect("n+1 detector poisoned");
        let span_map = inner.entry(span_id.to_string()).or_default();
        let count = span_map.entry(method.to_string()).or_insert(0);
        *count += 1;
        if *count == THRESHOLD {
            warn!(
                span_id = span_id,
                method = method,
                count = *count,
                "potential N+1 query detected — same method called {} times in one span",
                *count
            );
        }
    }

    /// Clears the detector state for a span (call this when the
    /// request ends to avoid unbounded growth).
    pub fn clear_span(&self, span_id: &str) {
        let mut inner = self.inner.write().expect("n+1 detector poisoned");
        inner.remove(span_id);
    }
}

impl Default for NPlusOneDetector {
    fn default() -> Self { Self::new() }
}
