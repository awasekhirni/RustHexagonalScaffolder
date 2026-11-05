//! Security cross-cutting concerns:
//! JWT, RBAC, PII redaction, security headers, RLS multi-tenancy,
//! GDPR erase/export, SecretProvider chain.

pub mod auth;
pub mod gdpr;
pub mod headers;
pub mod pii;
pub mod rls;
pub mod secrets;
pub mod tenancy;
