//! PostgreSQL pool construction.
//!
//! v0.3.0: `pool_registry` adds a multi-tenant pool registry keyed
//! by connection-profile id. The single-pool `build_pool` helper is
//! retained for backward compatibility and for fixture-mode projects
//! that do not declare multiple profiles.

pub mod pool;
pub mod pool_registry;

pub use pool::build_pool;
pub use pool_registry::PoolRegistry;
