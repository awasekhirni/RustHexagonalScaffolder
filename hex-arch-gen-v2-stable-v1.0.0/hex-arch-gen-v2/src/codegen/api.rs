// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! API layer codegen — the OpenAPI 3.1 document + Swagger UI binding.
//!
//! v0.3.0 (closes gap G-C01): utoipa 5 emits OpenAPI 3.0.3 natively;
//! there is no stable `openapi_3_1` feature flag in any released utoipa
//! version. The generated `api/openapi.rs` therefore builds the
//! document via utoipa's standard `#[derive(OpenApi)]` (3.0.3) and
//! then runs a `to_3_1()` post-processor at serve-time that:
//!
//! * sets the `openapi` field to `"3.1.0"`,
//! * adds `jsonSchemaDialect = "https://json-schema.org/draft/2020-12/schema"`,
//! * injects `webhooks` for every audit-log table detected by
//!   `analysis::classify_audit_log`,
//! * converts every `nullable: true` schema into the 3.1-native
//!   `type: ["X", "null"]` union.
//!
//! The handler at `/api-docs/openapi.json` returns the post-processed
//! JSON, so consumers see a true 3.1 document.

use std::path::Path;

use anyhow::Result;

use crate::codegen::write_file;
use crate::ir::Schema;

/// Writes `src/api/mod.rs` and `src/api/openapi.rs`.
pub fn write_shared(schema: &Schema, root: &Path) -> Result<()> {
    let api = root.join("src").join("api");
    write_file(&api.join("mod.rs"), &api_mod_rs(schema))?;
    write_file(&api.join("openapi.rs"), &openapi_rs(schema))?;
    Ok(())
}

fn api_mod_rs(_schema: &Schema) -> String {
    String::from(
        "//! API surface — OpenAPI 3.1 document + Swagger UI binding.\n//!\n//! The OpenAPI document is built by `utoipa` from the `#[utoipa::path]`\n//! attributes on every handler, then upgraded to 3.1 at serve-time by\n//! `openapi::to_3_1()`. The serialised form is served at\n//! `/api-docs/openapi.json` and rendered by Swagger UI at `/swagger-ui`.\n\npub mod openapi;\n\npub use openapi::ApiDoc;\npub use openapi::serve_openapi_3_1;\n",
    )
}

/// Renders the generated `src/api/openapi.rs`.
///
/// Emits a `#[derive(OpenApi)]` struct whose `#[openapi(...)]` attribute
/// uses the standard 3.0.3 dialect (no `openapi_3_1` modifier — that
/// feature does not exist on any released utoipa 5 version). The
/// `to_3_1()` post-processor upgrades the serialised JSON to 3.1 at
/// serve-time.
fn openapi_rs(schema: &Schema) -> String {
    let mut paths = String::new();
    let mut schemas = String::new();
    let mut tags = String::new();
    let mut webhook_names: Vec<String> = Vec::new();

    for t in &schema.tables {
        let m = &t.module_name;
        let n = &t.struct_name;
        paths.push_str(&format!(
            "        crate::presentation::handlers::{m}_handler::list,\n        crate::presentation::handlers::{m}_handler::get_one,\n        crate::presentation::handlers::{m}_handler::create,\n        crate::presentation::handlers::{m}_handler::update,\n        crate::presentation::handlers::{m}_handler::delete,\n"
        ));
        if t.analysis.status_col.is_some() {
            paths.push_str(&format!(
                "        crate::presentation::handlers::{m}_handler::list_by_status,\n        crate::presentation::handlers::{m}_handler::set_status,\n"
            ));
        }
        if t.analysis.has_soft_delete() {
            paths.push_str(&format!(
                "        crate::presentation::handlers::{m}_handler::soft_delete,\n        crate::presentation::handlers::{m}_handler::restore,\n"
            ));
        }
        if t.analysis.user_col.is_some() {
            paths.push_str(&format!(
                "        crate::presentation::handlers::{m}_handler::list_by_user,\n"
            ));
        }
        if t.analysis.created_at_col.is_some() {
            paths.push_str(&format!(
                "        crate::presentation::handlers::{m}_handler::created_after,\n"
            ));
        }
        if t.analysis.updated_at_col.is_some() {
            paths.push_str(&format!(
                "        crate::presentation::handlers::{m}_handler::updated_after,\n"
            ));
        }

        schemas.push_str(&format!(
            "        crate::domain::entities::{m}::{n},\n        crate::application::dto::{m}_dto::Create{n}Input,\n        crate::application::dto::{m}_dto::Update{n}Input,\n"
        ));
        if t.analysis.status_col.is_some() {
            schemas.push_str(&format!(
                "        crate::presentation::handlers::{m}_handler::Set{n}StatusInput,\n"
            ));
        }
        if t.analysis.created_at_col.is_some() {
            schemas.push_str(&format!(
                "        crate::presentation::handlers::{m}_handler::{n}CreatedAfterParams,\n"
            ));
        }
        if t.analysis.updated_at_col.is_some() {
            schemas.push_str(&format!(
                "        crate::presentation::handlers::{m}_handler::{n}UpdatedAfterParams,\n"
            ));
        }

        tags.push_str(&format!(
            "        (name = \"{m}\", description = \"{} management\"),\n",
            t.name
        ));

        // Collect audit-log table names so the 3.1 post-processor can
        // inject `webhooks` for each one.
        if t.analysis.is_audit_log {
            webhook_names.push(format!("{m}_changed"));
        }
    }
    // Always include the AuditChangedEvent schema so 3.1 webhook bodies
    // can resolve it via `components.schemas` even on projects with no
    // audit-log tables (the type is referenced unconditionally).
    schemas.push_str("        crate::api::openapi::AuditChangedEvent,\n");
    let _ = &mut schemas;

    // Render the list of webhook names as a Rust array literal so the
    // post-processor knows which webhooks to inject.
    let webhook_array = if webhook_names.is_empty() {
        "&[]".to_string()
    } else {
        let quoted: Vec<String> = webhook_names.iter().map(|w| format!("\"{w}\"")).collect();
        format!("&[{}]", quoted.join(", "))
    };

    format!(
        r##"//! OpenAPI 3.1 document, served at `/api-docs/openapi.json`.
//!
//! v0.3.0 (closes gap G-C01). utoipa 5 emits OpenAPI 3.0.3 natively;
//! there is no stable `openapi_3_1` feature flag in any released
//! version. The `to_3_1()` post-processor below upgrades the
//! serialised JSON to 3.1 at serve-time:
//!
//! * sets `openapi` to `"3.1.0"`,
//! * adds `jsonSchemaDialect = "https://json-schema.org/draft/2020-12/schema"`,
//! * injects `webhooks` for every audit-log table detected by
//!   `analysis::classify_audit_log`,
//! * converts every `nullable: true` schema into the 3.1-native
//!   `type: ["X", "null"]` union.
//!
//! The handler at `/api-docs/openapi.json` returns the post-processed
//! JSON, so consumers see a true 3.1 document.

use utoipa::OpenApi;

/// The generated OpenAPI 3.0.3 document (upgraded to 3.1 at serve-time
/// by `to_3_1()`).
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Hexagonal Architecture API",
        version = "1.0.0",
        description = "Generated by hex-gen v0.3.0. Writes go through the application service, reads through the same service; handlers depend only on trait-object ports.",
        license(name = "MIT")
    ),
    paths(
{paths}    ),
    components(schemas(
{schemas}    )),
    tags(
{tags}    )
)]
pub struct ApiDoc;

/// Marker event type for OpenAPI 3.1 webhook bodies.
///
/// Every audit-log table's webhook emits this payload shape so the
/// OpenAPI document has a single, stable payload type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AuditChangedEvent {{
    /// The table that emitted the event.
    pub table: String,
    /// The primary key of the changed row, serialised as a string.
    pub id: String,
    /// One of `insert`, `update`, `delete`.
    pub op: String,
    /// ISO-8601 UTC timestamp of the change.
    pub at: chrono::DateTime<chrono::Utc>,
}}

/// Webhook names that should be injected into the 3.1 document. One
/// entry per audit-log table; the array is built at codegen time so
/// the runtime post-processor does not need to introspect the IR.
const AUDIT_WEBHOOKS: &[&str] = {webhook_array};

/// Upgrades the serialised OpenAPI 3.0.3 document to 3.1.
///
/// Returns the post-processed JSON as a `serde_json::Value` so the
/// axum handler can serialise it directly without re-parsing.
///
/// # Errors
///
/// Returns an error if the underlying utoipa serialisation fails
/// (should never happen for a derive-generated `OpenApi`).
pub fn to_3_1() -> anyhow::Result<serde_json::Value> {{
    let mut doc: serde_json::Value =
        serde_json::from_str(&ApiDoc::openapi().to_pretty_json()?)?;
    let obj = doc.as_object_mut().ok_or_else(|| {{
        anyhow::anyhow!("OpenAPI document is not a JSON object")
    }})?;
    // 1. Bump the version.
    obj.insert("openapi".into(), serde_json::Value::String("3.1.0".into()));
    // 2. Pin the JSON Schema dialect.
    obj.insert(
        "jsonSchemaDialect".into(),
        serde_json::Value::String(
            "https://json-schema.org/draft/2020-12/schema".into(),
        ),
    );
    // 3. Inject webhooks for every audit-log table.
    if !AUDIT_WEBHOOKS.is_empty() {{
        let mut webhooks = serde_json::Map::new();
        for name in AUDIT_WEBHOOKS {{
            webhooks.insert(
                (*name).into(),
                serde_json::json!({{
                    "description": format!("Push notification emitted when an audit-log row changes ({{}}).", name),
                    "post": {{
                        "requestBody": {{
                            "content": {{
                                "application/json": {{
                                    "schema": {{
                                        "$ref": "#/components/schemas/AuditChangedEvent"
                                    }}
                                }}
                            }}
                        }},
                        "responses": {{
                            "200": {{ "description": "ack" }}
                        }}
                    }}
                }}),
            );
        }}
        obj.insert("webhooks".into(), serde_json::Value::Object(webhooks));
    }}
    // 4. Walk every schema in components and convert
    // `nullable: true` + `type: "X"` into the 3.1-native
    // `type: ["X", "null"]` union. OpenAPI 3.1 drops the `nullable`
    // keyword entirely.
    if let Some(components) = obj.get_mut("components").and_then(|c| c.get_mut("schemas")) {{
        if let Some(schemas_obj) = components.as_object_mut() {{
            for (_name, schema) in schemas_obj.iter_mut() {{
                convert_nullable_to_type_array(schema);
            }}
        }}
    }}
    Ok(doc)
}}

/// Recursively walks a JSON schema and converts `nullable: true` into
/// the 3.1-native `type: ["X", "null"]` form. Modifies the schema
/// in place.
fn convert_nullable_to_type_array(schema: &mut serde_json::Value) {{
    if let Some(obj) = schema.as_object_mut() {{
        let nullable = obj.get("nullable").and_then(|v| v.as_bool()).unwrap_or(false);
        if nullable {{
            obj.remove("nullable");
            // Merge "null" into the existing `type` field.
            let existing_type = obj.get("type").cloned();
            let new_type = match existing_type {{
                Some(serde_json::Value::String(t)) => {{
                    serde_json::Value::Array(vec![
                        serde_json::Value::String(t),
                        serde_json::Value::String("null".into()),
                    ])
                }}
                Some(serde_json::Value::Array(arr)) => {{
                    let mut arr = arr;
                    if !arr.iter().any(|v| v.as_str() == Some("null")) {{
                        arr.push(serde_json::Value::String("null".into()));
                    }}
                    serde_json::Value::Array(arr)
                }}
                _ => {{
                    serde_json::Value::String("null".into())
                }}
            }};
            obj.insert("type".into(), new_type);
        }}
        // Recurse into properties, items, allOf, anyOf, oneOf.
        for key in ["properties", "items", "allOf", "anyOf", "oneOf"] {{
            if let Some(child) = obj.get_mut(key) {{
                convert_nullable_to_type_array(child);
            }}
        }}
    }} else if let Some(arr) = schema.as_array_mut() {{
        for item in arr.iter_mut() {{
            convert_nullable_to_type_array(item);
        }}
    }}
}}

/// Axum handler that serves the OpenAPI 3.1 document.
///
/// Mounted at `/api-docs/openapi.json` by `presentation::build_router`.
pub async fn serve_openapi_3_1() -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {{
    match to_3_1() {{
        Ok(doc) => Ok(axum::Json(doc)),
        Err(e) => {{
            tracing::error!(error = %e, "failed to render OpenAPI 3.1 document");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }}
    }}
}}
"##,
        paths = paths,
        schemas = schemas,
        tags = tags,
        webhook_array = webhook_array,
    )
}
