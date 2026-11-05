//! Hexagonal REST + GraphQL API — library crate root.
//!
//! Re-exports the public modules composing the hexagonal architecture.
//! v0.3.0 adds `graphql` (closes G-C02) alongside the existing REST
//! surface; both share the same application service layer.

pub mod api;
pub mod application;
pub mod config;
pub mod core;
pub mod domain;
pub mod graphql;
pub mod infrastructure;
pub mod logging;
pub mod patterns;
pub mod presentation;
pub mod security;
pub mod shared;
pub mod telemetry;

/// Re-export of the primary presentation entrypoint.
pub use presentation::build_router;
