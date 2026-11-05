//! Saga step registry (stub).
//!
//! The v0.6.0 desired-state persists sagas in PostgreSQL with a
//! compensation registry; here we expose the canonical kind constant
//! and a step list helper so handlers compile.

use sqlx::PgPool;

use crate::core::saga::SagaStep;

/// Canonical saga kind for user onboarding.
pub const SAGA_KIND: &str = "user_onboarding";

/// Returns the ordered saga steps for user onboarding.
#[must_use]
pub fn user_onboarding_steps(_pool: PgPool) -> Vec<SagaStep> {
    vec![
        SagaStep { name: "create_user_record".to_string() },
        SagaStep { name: "provision_mailbox".to_string() },
        SagaStep { name: "send_welcome_email".to_string() },
    ]
}
