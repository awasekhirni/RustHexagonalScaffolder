//! Apollo Federation v2 entity resolver skeleton.
//!
//! v0.3.0 ships this module as a documented skeleton. The SDL
//! emitted by `sdl::render` already carries `@key(field: "id")`
//! directives on audit-log tables; the `_entities` resolver that
//! lets the Federation gateway fetch any entity by its primary
//! key lands in v0.4.0.

/// Marker type for the v0.4.0 Federation entity registry.
pub struct FederationRoot;

impl FederationRoot {
    /// v0.4.0 will expose `_entities(representations)` here.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for FederationRoot {
    fn default() -> Self {
        Self::new()
    }
}
