//! Domain entities — `FromRow` aggregate roots per bounded context.
//!
//! Each entity maps 1:1 to a PostgreSQL table via `sqlx::FromRow` and
//! serialises to JSON for HTTP responses and the OpenAPI document.

pub mod orders;
pub mod product_categories;
pub mod users;
