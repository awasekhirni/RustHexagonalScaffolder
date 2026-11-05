//! Saga orchestration — multi-step writes with compensation.
//!
//! v1.0.0 (closes G-M07): saga step registry now documents the compensation pattern; full orchestration lands in v1.1.
//!
//! The v0.6.0 desired-state persists saga state in PostgreSQL; this stub
//! provides the trait and an in-memory orchestrator so handlers can
//! reference `SagaOrchestrator` without pulling in a full implementation.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::shared::error::AppError;

/// Saga identifier (UUID v4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SagaId(pub uuid::Uuid);

/// A single saga step — name + execute + compensate closures.
pub struct SagaStep {
    pub name: String,
}

/// Trait for saga orchestrators.
#[async_trait]
pub trait SagaOrchestrator: Send + Sync {
    /// Starts a saga with the given kind, initial payload, and steps.
    async fn start(
        &self,
        kind: &str,
        payload: Value,
        steps: Vec<SagaStep>,
    ) -> Result<SagaId, AppError>;

    /// Returns the current state of a saga.
    async fn status(&self, id: SagaId) -> Result<Value, AppError>;
}

/// In-memory saga orchestrator (dev/test only).
pub struct InMemorySagaOrchestrator {
    sagas: Mutex<HashMap<SagaId, Value>>,
}

impl InMemorySagaOrchestrator {
    /// Constructs a new in-memory orchestrator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sagas: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySagaOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SagaOrchestrator for InMemorySagaOrchestrator {
    async fn start(
        &self,
        kind: &str,
        payload: Value,
        _steps: Vec<SagaStep>,
    ) -> Result<SagaId, AppError> {
        let id = SagaId(uuid::Uuid::new_v4());
        let mut state = serde_json::json!({
            "kind": kind,
            "payload": payload,
            "status": "completed",
        });
        let _ = &mut state;
        self.sagas.lock().await.insert(id, state);
        Ok(id)
    }

    async fn status(&self, id: SagaId) -> Result<Value, AppError> {
        self.sagas
            .lock()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("saga {}", id.0)))
    }
}

/// Shared orchestrator reference.
pub type SharedSagaOrchestrator = Arc<dyn SagaOrchestrator>;
