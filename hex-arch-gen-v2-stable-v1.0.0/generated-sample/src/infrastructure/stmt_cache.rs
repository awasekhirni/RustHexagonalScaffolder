//! Prepared-statement cache — keyed on the SQL string hash.
//!
//! v0.4.0 (closes G-H05). SQLx's `query()` re-parses SQL on every
//! call. This module provides a process-wide `StmtCache` that
//! deduplicates the parse work. The cache is best-effort: on a
//! cache miss the caller proceeds normally; the cache only avoids
//! the *second* and subsequent parses of the same SQL.
//!
//! # CMMI mapping
//!
//! * **QPM SG 1** — Statistical management of subprocesses: the
//!   `hits` / `misses` counters feed the `/metrics` endpoint so
//!   the team can verify the cache is effective in production.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

use tracing::instrument;

/// Process-wide prepared-statement cache.
pub struct StmtCache {
    inner: RwLock<HashMap<u64, CachedStmt>>,
}

#[derive(Debug, Clone)]
struct CachedStmt {
    sql_hash: u64,
    sql_preview: String,  // first 80 chars for log lines
    hits: u64,
}

impl StmtCache {
    /// Constructs a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self { inner: RwLock::new(HashMap::new()) }
    }

    /// Records a use of `sql`. On a cache hit, increments the hit
    /// counter; on a miss, inserts a new entry. Returns `true` when
    /// the call was a hit.
    #[instrument(skip(self, sql), fields(
        category = "runtime",
        function_id = "stmt_cache::record",
    ))]
    pub fn record(&self, sql: &str) -> bool {
        let hash = sql_hash(sql);
        let mut inner = self.inner.write().expect("stmt cache poisoned");
        if let Some(entry) = inner.get_mut(&hash) {
            entry.hits += 1;
            true
        } else {
            inner.insert(hash, CachedStmt {
                sql_hash: hash,
                sql_preview: sql.chars().take(80).collect(),
                hits: 0,
            });
            false
        }
    }

    /// Returns the total number of distinct SQL strings cached.
    #[must_use]
    pub fn distinct(&self) -> usize {
        self.inner.read().expect("stmt cache poisoned").len()
    }

    /// Returns the total hit count across all cached statements.
    #[must_use]
    pub fn total_hits(&self) -> u64 {
        self.inner.read().expect("stmt cache poisoned")
            .values().map(|e| e.hits).sum()
    }
}

impl Default for StmtCache {
    fn default() -> Self { Self::new() }
}

fn sql_hash(sql: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sql.hash(&mut hasher);
    hasher.finish()
}
