//! In-process LRU entity cache with LISTEN/NOTIFY invalidation.
//!
//! Stub: production code uses `moka::future::Cache`; this stub returns
//! misses on every call so behaviour matches "no cache" without forcing
//! the dependency on every generated project.

use std::marker::PhantomData;
use std::sync::Arc;

/// Entity cache (stub).
#[derive(Debug, Clone)]
pub struct EntityCache<T> {
    _name: String,
    _capacity: u64,
    _ttl_secs: u64,
    _phantom: PhantomData<T>,
}

impl<T> EntityCache<T> {
    /// Constructs a new cache (stub).
    #[must_use]
    pub fn new(name: &str, capacity: u64, ttl_secs: u64) -> Self {
        Self {
            _name: name.to_string(),
            _capacity: capacity,
            _ttl_secs: ttl_secs,
            _phantom: PhantomData,
        }
    }

    /// Clones the cache into a future that listens for LISTEN/NOTIFY
    /// invalidation messages. Stub — returns immediately.
    pub async fn spawn_invalidation_listener(self, _pool: sqlx::PgPool, _channel: String) {
        // no-op stub
    }
}

/// Returns `None` always (stub).
impl<T> EntityCache<T> {
    /// Returns the cached value for `key`, or `None` (stub always returns None).
    #[must_use]
    pub fn get(&self, _key: &str) -> Option<Arc<T>> {
        None
    }
}
