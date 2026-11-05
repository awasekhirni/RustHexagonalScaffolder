// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Codegen for the generated project's integration test harness.
//!
//! Closes gap **G-H09**. The v0.3 sample shipped with only the
//! generator's own `tests/smoke.rs` (file existence check). This
//! module emits `tests/integration_smoke.rs` into every generated
//! project, exercising:
//!
//! * the GraphQL `build_schema()` helper (no live DB required),
//! * the OpenAPI 3.1 `to_3_1()` post-processor,
//! * the JWT issue→verify round-trip,
//! * the RBAC policy allow/deny paths.
//!
//! The tests are hermetic (no DB, no network) so they run in CI
//! without spinning a testcontainer. A follow-up module in v0.5
//! will emit testcontainer-backed CRUD round-trips per table.

use std::path::Path;

use tracing::instrument;

use crate::codegen::write_file;
use crate::errors::GenResult;
use crate::ir::Schema;

/// Emits `tests/integration_smoke.rs` into the generated project.
#[instrument(
    skip(_schema, root),
    fields(category = "codegen", function_id = "test_harness::write_all",),
    err
)]
pub fn write_all(_schema: &Schema, root: &Path) -> GenResult<()> {
    let path = root.join("tests").join("integration_smoke.rs");
    write_file(&path, INTEGRATION_SMOKE_RS)?;
    Ok(())
}

const INTEGRATION_SMOKE_RS: &str = r#"//! Integration smoke tests for the generated project.
//!
//! v0.4.0 (closes G-H09). Hermetic — no DB, no network. Exercises:
//! * the GraphQL `build_schema()` helper,
//! * the OpenAPI 3.1 `to_3_1()` post-processor,
//! * the JWT issue→verify round-trip + scope enforcement,
//! * the RBAC policy allow/deny paths.

use generated_api::api::openapi::{to_3_1, ApiDoc};
use generated_api::graphql::{build_schema, MutationRoot, QueryRoot};
use generated_api::security::auth::jwt::JwtService;
use generated_api::security::auth::rbac::{RbacEntry, RbacPolicy};
use generated_api::security::auth::extractor::AuthUser;

use async_graphql::{EmptySubscription, Schema as GqlSchema};
use utoipa::OpenApi;
use uuid::Uuid;

/// The GraphQL schema builds without panicking.
#[test]
fn graphql_schema_builds() {
    let _schema: GqlSchema<QueryRoot, MutationRoot, EmptySubscription> = build_schema();
}

/// The OpenAPI 3.1 post-processor returns a JSON object with the
/// `openapi` field set to `"3.1.0"`.
#[test]
fn openapi_3_1_post_processor_emits_3_1_version() {
    let doc = to_3_1().expect("to_3_1 must succeed");
    assert_eq!(doc["openapi"], "3.1.0");
    assert_eq!(
        doc["jsonSchemaDialect"],
        "https://json-schema.org/draft/2020-12/schema"
    );
}

/// The ApiDoc derive compiles and exposes an `openapi()` method.
#[test]
fn openapi_apidoc_derives() {
    let _ = ApiDoc::openapi();
}

/// A JWT issued by the service verifies cleanly.
#[test]
fn jwt_issue_verify_round_trip() {
    let svc = JwtService::new("test-secret-32-chars-minimum-please", "test-issuer", "test-aud", 3600);
    let user = Uuid::new_v4();
    let token = svc.issue(user, Some("tenant-a".into()), Some("user".into()))
        .expect("issue must succeed");
    let claims = svc.verify(&token).expect("verify must succeed");
    assert_eq!(claims.sub, user.to_string());
    assert_eq!(claims.tenant_id.as_deref(), Some("tenant-a"));
    assert_eq!(claims.role.as_deref(), Some("user"));
    assert!(claims.scopes.is_empty(), "access token must not carry refresh scope");
}

/// A refresh-scope token is rejected by `verify()` (which expects
/// an access token).
#[test]
fn jwt_rejects_refresh_scope_when_verifying_access() {
    let svc = JwtService::new("test-secret-32-chars-minimum-please", "test-issuer", "test-aud", 3600);
    let user = Uuid::new_v4();
    let (_access, refresh) = svc.issue_refresh_pair(user, None, Some("user".into()))
        .expect("issue_refresh_pair must succeed");
    let result = svc.verify(&refresh);
    assert!(result.is_err(), "verify must reject refresh-scope token");
}

/// A refresh token verifies via `verify_refresh()`.
#[test]
fn jwt_refresh_round_trip() {
    let svc = JwtService::new("test-secret-32-chars-minimum-please", "test-issuer", "test-aud", 3600);
    let user = Uuid::new_v4();
    let (_access, refresh) = svc.issue_refresh_pair(user, None, Some("user".into()))
        .expect("issue_refresh_pair must succeed");
    let claims = svc.verify_refresh(&refresh).expect("verify_refresh must succeed");
    assert_eq!(claims.sub, user.to_string());
    assert!(claims.scopes.iter().any(|s| s == "refresh"));
}

/// Key rotation: a token signed with the old key continues to
/// verify for one TTL window after rotation.
#[test]
fn jwt_key_rotation_preserves_outstanding_tokens() {
    let svc = JwtService::new("old-secret-32-chars-minimum-please", "i", "a", 3600);
    let user = Uuid::new_v4();
    let token = svc.issue(user, None, None).expect("issue");
    svc.rotate_key(b"new-secret-32-chars-minimum-please", "v2").expect("rotate");
    let claims = svc.verify(&token).expect("old token must still verify after rotation");
    assert_eq!(claims.sub, user.to_string());
}

/// RBAC: the `admin` role is a wildcard allow.
#[test]
fn rbac_admin_wildcard_allow() {
    let policy = RbacPolicy::new();
    let auth = AuthUser {
        user_id: Uuid::new_v4(),
        tenant_id: None,
        role: "admin".into(),
        scopes: vec![],
    };
    policy.check(&auth, "orders", "delete").expect("admin must be allowed");
}

/// RBAC: a role with no matching entry is denied.
#[test]
fn rbac_unknown_role_denied() {
    let policy = RbacPolicy::new();
    let auth = AuthUser {
        user_id: Uuid::new_v4(),
        tenant_id: None,
        role: "unknown".into(),
        scopes: vec![],
    };
    assert!(policy.check(&auth, "orders", "read").is_err());
}

/// RBAC: an exact (role, table, action) entry is allowed.
#[test]
fn rbac_exact_match_allowed() {
    let mut policy = RbacPolicy::new();
    policy.add(RbacEntry {
        role: "orders:read".into(),
        table: "orders".into(),
        action: "read".into(),
        row_filter: None,
    });
    let auth = AuthUser {
        user_id: Uuid::new_v4(),
        tenant_id: None,
        role: "orders:read".into(),
        scopes: vec![],
    };
    policy.check(&auth, "orders", "read").expect("exact match must be allowed");
}

/// RBAC: an action mismatch is denied even when the role+table match.
#[test]
fn rbac_action_mismatch_denied() {
    let mut policy = RbacPolicy::new();
    policy.add(RbacEntry {
        role: "orders:read".into(),
        table: "orders".into(),
        action: "read".into(),
        row_filter: None,
    });
    let auth = AuthUser {
        user_id: Uuid::new_v4(),
        tenant_id: None,
        role: "orders:read".into(),
        scopes: vec![],
    };
    assert!(policy.check(&auth, "orders", "delete").is_err(),
        "read-scoped role must not be allowed to delete");
}
"#;
