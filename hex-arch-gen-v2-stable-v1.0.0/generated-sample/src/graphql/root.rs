//! Root Query and Mutation types.
//!
//! Each field delegates to a per-table resolver module so the
//! root file stays small even on projects with many tables.

use async_graphql::Object;

/// Root Query — aggregates every per-table Query field set.
#[derive(Debug, Clone, Copy)]
pub struct QueryRoot;
impl Default for QueryRoot { fn default() -> Self { Self } }

/// Root Mutation — aggregates every per-table Mutation field set.
#[derive(Debug, Clone, Copy)]
pub struct MutationRoot;
impl Default for MutationRoot { fn default() -> Self { Self } }

#[Object]
impl QueryRoot {
    /// Returns the SDL string for introspection clients.
    async fn sdl(&self) -> String {
        crate::graphql::SCHEMA_STR.to_string()
    }
}

#[Object]
impl MutationRoot {
    /// Liveness probe — returns `true`. Useful for clients that
    /// need to verify mutation routing without side effects.
    async fn ping(&self) -> bool { true }
}
