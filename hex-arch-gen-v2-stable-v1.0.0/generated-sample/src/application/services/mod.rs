//! Use-case services — implement the inbound port on top of repository + bus ports.

pub mod orders_service;
pub mod product_categories_service;
pub mod users_service;

use async_trait::async_trait;
use std::sync::Arc;

use crate::application::ports::bus::{DomainEvent, EventBus};

/// A logging event handler that prints every event to the tracing log.
#[derive(Default)]
pub struct LoggingEventHandler;

#[async_trait]
impl crate::application::ports::bus::EventHandler for LoggingEventHandler {
    async fn handle(&self, event: &dyn DomainEvent) {
        tracing::info!(event_name = event.name(), "event handled (log)");
    }
}

/// A canned `UserCreated` event — emitted by the user service after a
/// successful create.
#[derive(Debug, Clone)]
pub struct UserCreated {
    pub id: uuid::Uuid,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent for UserCreated {
    fn name(&self) -> &'static str {
        "user_created"
    }
}

/// A canned `ProductCreated` event for parity with the desired state.
#[derive(Debug, Clone)]
pub struct ProductCreated {
    pub id: uuid::Uuid,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent for ProductCreated {
    fn name(&self) -> &'static str {
        "product_created"
    }
}
