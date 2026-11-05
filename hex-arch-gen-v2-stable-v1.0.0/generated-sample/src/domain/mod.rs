//! Domain layer — pure business entities and value objects.
//!
//! This layer knows nothing about axum, SQLx or HTTP: it only declares
//! *what* the application can do, never *how* it is done.

pub mod entities;
pub mod value_objects;
