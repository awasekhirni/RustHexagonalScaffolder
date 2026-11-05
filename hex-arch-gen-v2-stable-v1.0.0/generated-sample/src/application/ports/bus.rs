//! Event bus port — publishes domain events to in-process subscribers.
//!
//! The trait is dyn-compatible: every method takes concrete (non-generic)
//! arguments so `Arc<dyn EventBus>` works.

use async_trait::async_trait;
use std::sync::Arc;

/// Type-erased domain event — every concrete event box-into this.
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
    /// Stable event name (used for routing and logging).
    fn name(&self) -> &'static str;
}

/// Event handler trait — called when a matching event is published.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handles the event.
    async fn handle(&self, event: &(dyn DomainEvent));
}

/// Event bus port — publish/subscribe for domain events.
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publishes an event to all subscribers.
    async fn publish(&self, event: Box<dyn DomainEvent>);

    /// Subscribes a handler to all events of a given name.
    async fn subscribe(&self, event_name: &'static str, handler: Arc<dyn EventHandler>);
}

/// Type alias for shared bus references.
pub type SharedBus = Arc<dyn EventBus>;
