//! Domain layer — entities, input models and ports (the hexagon core).
//!
//! This layer knows nothing about axum, SQLx or HTTP: it only declares
//! *what* the application can do, never *how* it is done.

pub mod orders;
pub mod product_categories;
pub mod users;
