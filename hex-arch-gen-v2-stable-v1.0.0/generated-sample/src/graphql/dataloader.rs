//! DataLoader skeleton — FK batching.
//!
//! v0.3.0 ships this module as a documented skeleton. Full
//! implementation (one `Loader<PK>` per FK relation, single
//! `SELECT ... WHERE id = ANY($1)` per batch) lands in v0.4.0
//! along with the `#[Object]` field resolvers that consume it.

use async_trait::async_trait;
use std::collections::HashMap;
use tracing::instrument;

/// Marker type for the v0.4.0 DataLoader registry.
///
/// Today the module exists only so the generated `mod.rs` can
/// reference it; the `Loader` impls will be emitted per-FK in the
/// next release.
pub struct DataLoaderRegistry;

impl DataLoaderRegistry {
    /// Returns a new empty registry. v0.4.0 will populate it.
    #[must_use]
    #[instrument(skip_all, fields(category = "runtime", function_id = "dataloader::new"))]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DataLoaderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

