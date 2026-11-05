//! Ports — outbound contracts owned by the application layer.
//!
//! * `bus`        — `EventBus` for in-process domain event publishing.
//! * `repository`  — per-entity repository port traits.

pub mod bus;
pub mod repository;
