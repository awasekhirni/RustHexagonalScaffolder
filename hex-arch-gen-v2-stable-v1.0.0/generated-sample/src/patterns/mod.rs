//! Resilience and integration patterns:
//! idempotency, outbox + DLQ, circuit breaker, retry, cursor pagination,
//! filtering, webhooks, cache, feature flags, load shed.

pub mod cache;
pub mod circuit_breaker;
pub mod feature_flags;
pub mod filtering;
pub mod idempotency;
pub mod load_shed;
pub mod outbox_dlq;
pub mod outbox_worker;
pub mod pagination;
pub mod retry;
pub mod webhooks;
