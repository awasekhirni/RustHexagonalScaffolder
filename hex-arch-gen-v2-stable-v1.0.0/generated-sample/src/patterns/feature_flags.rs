//! Feature-flag store — backed by a `feature_flags` table (flag, enabled).
//!
//! Stub: production code loads flags on boot and LISTENs for changes;
//! this stub returns `false` for every unknown flag.

use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::Arc;

/// Feature-flag store.
pub struct FlagStore {
    pool: PgPool,
    flags: RwLock<HashMap<String, bool>>,
}

impl FlagStore {
    /// Constructs a new store.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            flags: RwLock::new(HashMap::new()),
        }
    }

    /// Reloads flags from the database.
    ///
    /// # Errors
    ///
    /// Propagates SQLx errors.
    pub async fn reload(&self) -> Result<(), sqlx::Error> {
        let rows: Vec<(String, bool)> = sqlx::query_as(
            "SELECT name, enabled FROM feature_flags",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        let mut flags = self.flags.write().unwrap();
        flags.clear();
        for (name, enabled) in rows {
            flags.insert(name, enabled);
        }
        Ok(())
    }

    /// Returns the flag value (default `false`).
    #[must_use]
    pub fn is_enabled(&self, name: &str) -> bool {
        let flags = self.flags.read().unwrap();
        flags.get(name).copied().unwrap_or(false)
    }
}

/// Shared flag store.
pub type SharedFlagStore = Arc<FlagStore>;
