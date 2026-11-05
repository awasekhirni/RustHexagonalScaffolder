//! In-process `EventBus` implementation.
//!
//! Subscribers are stored in a `HashMap<&'static str, Vec<Arc<dyn EventHandler>>>`
//! keyed by the event's `name()` so a single event name can have many subscribers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::application::ports::bus::{DomainEvent, EventBus, EventHandler};

/// In-process event bus.
#[derive(Default)]
pub struct EnterpriseBus {
    subscribers: Mutex<HashMap<&'static str, Vec<Arc<dyn EventHandler>>>>,
}

impl EnterpriseBus {
    /// Constructs a new bus.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventBus for EnterpriseBus {
    async fn publish(&self, event: Box<dyn DomainEvent>) {
        let name = event.name();
        // Clone the handler list out of the Mutex before awaiting so the
        // guard is not held across the await boundary (Send safety).
        let handlers = {
            let subscribers = self.subscribers.lock().unwrap();
            subscribers.get(name).cloned().unwrap_or_default()
        };
        for handler in handlers {
            handler.handle(event.as_ref()).await;
        }
    }

    async fn subscribe(&self, event_name: &'static str, handler: Arc<dyn EventHandler>) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.entry(event_name).or_default().push(handler);
    }
}
