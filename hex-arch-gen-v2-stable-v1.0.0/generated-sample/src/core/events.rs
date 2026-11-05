//! Domain events — emitted by application services, published to the bus.
//!
//! Re-exports the `DomainEvent` trait from the application layer's bus port
//! so handlers can `impl DomainEvent for MyEvent` without crossing layers.

pub use crate::application::ports::bus::DomainEvent;

/// Type-erased event box.
pub type BoxedEvent = Box<dyn DomainEvent>;
