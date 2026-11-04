//! Infrastructure layer — outbound adapters (SQLx/PostgreSQL).
//!
//! Each adapter implements the repository port of its domain context;
//! nothing in this layer is referenced by the domain or application code
//! except through `main`'s constructor wiring.

pub mod orders_repository;
pub mod product_categories_repository;
pub mod users_repository;
