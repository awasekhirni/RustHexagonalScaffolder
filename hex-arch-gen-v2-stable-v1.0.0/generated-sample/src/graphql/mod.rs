//! GraphQL surface — SDL + Query/Mutation roots.
//!
//! v0.3.0 (closes G-C02). The GraphQL schema mirrors the REST
//! surface and shares the same application service layer. The
//! SQLx adapters are reused unchanged — the hexagonal boundary
//! is preserved.

pub mod dataloader;
pub mod federation;
pub mod root;
pub mod resolvers {
    pub mod orders;
    pub mod product_categories;
    pub mod users;
}

pub use root::{MutationRoot, QueryRoot};

/// Embeds the SDL string built at codegen time.
pub const SCHEMA_STR: &str = include_str!("schema.graphql");

/// Builds the GraphQL schema. Wires the Query and Mutation roots
/// to the application state and embeds the SDL string built by
/// `sdl::render` at codegen time.
///
/// Subscriptions are empty for now — audit-table streaming lands
/// in v0.4.0 along with DataLoader batching.
#[must_use]
pub fn build_schema() -> async_graphql::Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription> {
    async_graphql::Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
        .finish()
}
