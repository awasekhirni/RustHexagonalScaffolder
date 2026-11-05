//! Application layer — use-case services, DTOs, ports (the hexagon core).
//!
//! Services depend only on port traits; the presentation layer calls them,
//! and infrastructure provides concrete adapters.

pub mod dto;
pub mod ports;
pub mod sagas;
pub mod services;
