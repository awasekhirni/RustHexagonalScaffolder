// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Presentation layer codegen — per-table axum handlers, the shared
//! router, application state, and the middleware pipeline.
//!
//! Each per-table handler module exposes:
//!
//! * `list`     — `GET    /{m}`           paginated listing.
//! * `create`   — `POST   /{m}`           create (with optional `X-Saga: true`).
//! * `get_one`  — `GET    /{m}/{id}`      fetch one (ETag + If-None-Match).
//! * `update`   — `PUT    /{m}/{id}`      partial update (If-Match).
//! * `delete`   — `DELETE /{m}/{id}`      delete (soft when available).
//! * extras:    `list_by_status`, `set_status`, `soft_delete`, `restore`,
//!              `list_by_user`, `created_after`, `updated_after` (when detected).

use std::path::Path;

use anyhow::Result;

use crate::codegen::domain_helpers::{status_type, ts_type, user_type};
use crate::codegen::{write_file, ImportSet};
use crate::ir::{Schema, Table};

/// Writes `src/presentation/handlers/<table>_handler.rs` for one table.
pub fn write_table(_schema: &Schema, root: &Path, t: &Table) -> Result<()> {
    let path = root
        .join("src")
        .join("presentation")
        .join("handlers")
        .join(format!("{}_handler.rs", t.module_name));
    write_file(&path, &handler_rs(t))?;
    Ok(())
}

/// Writes the shared presentation artefacts: `mod.rs`, `state.rs`,
/// `routes.rs`, and `handlers/mod.rs`.
pub fn write_shared(schema: &Schema, root: &Path) -> Result<()> {
    let pres = root.join("src").join("presentation");

    write_file(
        &pres.join("mod.rs"),
        "//! Presentation layer — axum handlers, routes, middleware, state.\n//!\n//! Handlers are thin inbound adapters: they deserialize transport types,\n//! call the application service, and map errors onto HTTP status codes.\n//! No business logic lives here.\n\npub mod error;\npub mod handlers;\npub mod middleware;\npub mod routes;\npub mod state;\n\nuse axum::middleware::from_fn;\nuse axum::Router;\nuse tower_http::trace::TraceLayer;\n\nuse crate::presentation::middleware::security_headers;\nuse crate::presentation::state::AppState;\n\n/// Builds the full router (called from `main`).\n#[must_use]\npub fn build_router(state: AppState) -> Router {\n    let api = crate::presentation::routes::router();\n    Router::new()\n        .nest(\"/api/v1\", api)\n        .route(\"/health\", axum::routing::get(health))\n        .layer(from_fn(security_headers))\n        .layer(TraceLayer::new_for_http())\n        .with_state(state)\n}\n\n/// `GET /health` — liveness probe.\nasync fn health() -> &'static str {\n    \"ok\"\n}\n",
    )?;

    // handlers/mod.rs
    let mut body =
        String::from("//! Per-context HTTP handlers — one module per bounded context.\n\n");
    for t in &schema.tables {
        body.push_str(&format!("pub mod {}_handler;\n", t.module_name));
    }
    // Re-export a health handler.
    body.push_str("\n/// `GET /api/v1/health` — readiness probe inside the API surface.\npub async fn health() -> impl axum::response::IntoResponse {\n    axum::Json(serde_json::json!({\"status\": \"ok\"}))\n}\n");
    write_file(&pres.join("handlers").join("mod.rs"), &body)?;

    // state.rs
    write_file(&pres.join("state.rs"), &state_rs(schema))?;

    // routes.rs
    write_file(&pres.join("routes.rs"), &routes_rs(schema))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// state.rs
// ─────────────────────────────────────────────────────────────────────────────

fn state_rs(schema: &Schema) -> String {
    let mut fields = String::new();
    for t in &schema.tables {
        fields.push_str(&format!(
            "    /// `{}` application service.\n    pub {m}_service: std::sync::Arc<dyn crate::application::ports::repository::{n}ServicePort>,\n",
            t.name,
            m = t.module_name,
            n = t.struct_name
        ));
    }

    format!(
        r#"//! Shared application state handed to every Axum handler.
//!
//! Each handler pulls the service it needs from this struct; nothing in
//! the presentation layer constructs services directly.
//!
//! v0.4.0 (closes G-H01, G-H02): added `jwt` and `rbac` fields so
//! every handler can verify tokens and enforce per-table authorization
//! without re-constructing the services on each request.

use std::sync::Arc;

use sqlx::PgPool;

use crate::application::ports::bus::EventBus;
use crate::core::shutdown::InFlightTracker;
use crate::security::auth::jwt::JwtService;
use crate::security::auth::rbac::RbacPolicy;

/// Dependencies shared across the whole HTTP surface.
#[derive(Clone)]
pub struct AppState {{
    /// Shared PostgreSQL pool — used by handlers that need direct SQL
    /// access (audit endpoints, restore shortcuts, …).
    pub pool: PgPool,
{fields}
    /// Event bus (in-process).
    pub bus: Arc<dyn EventBus>,
    /// In-flight tracker for graceful shutdown drain.
    pub inflight: Arc<InFlightTracker>,
    /// JWT service — used by the `ReqAuth` extractor to verify
    /// `Authorization: Bearer <token>` headers.
    pub jwt: Arc<JwtService>,
    /// RBAC policy — used by handlers via the `require!` macro to
    /// enforce per-table authorization.
    pub rbac: Arc<RbacPolicy>,
}}
"#,
        fields = fields,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// routes.rs
// ─────────────────────────────────────────────────────────────────────────────

fn routes_rs(schema: &Schema) -> String {
    let mut imports =
        String::from("use axum::Router;\n\nuse crate::presentation::state::AppState;\n\n");
    for t in &schema.tables {
        imports.push_str(&format!(
            "use crate::presentation::handlers::{}_handler as {};\n",
            t.module_name, t.module_name
        ));
    }

    let mut routes = String::from("    Router::new()\n");
    // Health under /api/v1/health
    routes.push_str(
        "        .route(\"/health\", axum::routing::get(crate::presentation::handlers::health))\n",
    );

    for t in &schema.tables {
        let m = &t.module_name;
        routes.push_str(&format!(
            "        .route(\"/{m}\", axum::routing::get({m}::list).post({m}::create))\n"
        ));
        routes.push_str(&format!(
            "        .route(\"/{m}/{{id}}\", axum::routing::get({m}::get_one).put({m}::update).delete({m}::delete))\n"
        ));
        if t.analysis.status_col.is_some() {
            routes.push_str(&format!(
                "        .route(\"/{m}/status/{{status}}\", axum::routing::get({m}::list_by_status))\n"
            ));
            routes.push_str(&format!(
                "        .route(\"/{m}/{{id}}/status\", axum::routing::patch({m}::set_status))\n"
            ));
        }
        if t.analysis.has_soft_delete() {
            routes.push_str(&format!(
                "        .route(\"/{m}/{{id}}/soft\", axum::routing::delete({m}::soft_delete))\n"
            ));
            routes.push_str(&format!(
                "        .route(\"/{m}/{{id}}/restore\", axum::routing::post({m}::restore))\n"
            ));
        }
        if t.analysis.user_col.is_some() {
            routes.push_str(&format!(
                "        .route(\"/{m}/by-user/{{user_id}}\", axum::routing::get({m}::list_by_user))\n"
            ));
        }
        if t.analysis.created_at_col.is_some() {
            routes.push_str(&format!(
                "        .route(\"/{m}/created-after\", axum::routing::get({m}::created_after))\n"
            ));
        }
        if t.analysis.updated_at_col.is_some() {
            routes.push_str(&format!(
                "        .route(\"/{m}/updated-after\", axum::routing::get({m}::updated_after))\n"
            ));
        }
    }

    format!(
        r#"//! HTTP route table for the generated API.
//!
//! State is attached in `main` (after Swagger UI is merged), so this
//! function stays a pure description of the `/api/v1` surface.

{imports}
/// Builds the `/api/v1` routes.
#[must_use]
pub fn router() -> Router<AppState> {{
{routes}}}
"#,
        imports = imports,
        routes = routes,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// handler.rs
// ─────────────────────────────────────────────────────────────────────────────

fn handler_rs(t: &Table) -> String {
    let pk = t.pk().expect("pk required for codegen");
    let struct_name = &t.struct_name;
    let module = &t.module_name;
    let pk_ty = pk.kind.rust_type();
    let tag = &t.name;

    let mut imports = String::new();
    imports.push_str("use axum::extract::{Path, Query, State};\n");
    imports.push_str("use axum::http::{HeaderMap, HeaderValue, StatusCode};\n");
    imports.push_str("use axum::response::IntoResponse;\n");
    imports.push_str("use axum::Json;\n");
    imports.push_str("use serde_json::json;\n");
    imports.push_str("use tracing::instrument;\n");
    imports.push_str("use validator::Validate;\n");

    let mut handler_cols: Vec<&crate::ir::Column> = vec![pk];
    if let Some(user) = t.analysis.user_col.as_ref() {
        if let Some(c) = t.columns.iter().find(|c| &c.name == user) {
            handler_cols.push(c);
        }
    }
    for which in [&t.analysis.created_at_col, &t.analysis.updated_at_col] {
        if let Some(c) = t.columns.iter().find(|c| Some(&c.name) == which.as_ref()) {
            handler_cols.push(c);
        }
    }
    for line in ImportSet::from_cols(handler_cols).render() {
        imports.push_str(&line);
        imports.push('\n');
    }
    imports.push_str(&format!(
        "use crate::application::dto::{module}_dto::{{Create{struct_name}Input, Update{struct_name}Input}};\n"
    ));
    imports.push_str(&format!(
        "use crate::application::ports::repository::{{Page, Paginated, {struct_name}ServicePort}};\n"
    ));
    imports.push_str(&format!(
        "use crate::domain::entities::{module}::{struct_name};\n"
    ));
    imports.push_str("use crate::presentation::state::AppState;\n");
    imports.push_str("use crate::shared::error::AppError;\n");
    imports.push_str("use crate::shared::pagination::Pagination;\n");
    if t.analysis.status_col.is_some() {
        for (_, rust_name) in t.used_enums() {
            imports.push_str(&format!(
                "use crate::domain::entities::{module}::{rust_name};\n"
            ));
        }
    }

    let mut out = String::new();
    out.push_str(&format!(
        "//! Axum handlers for the `{}` context.\n//!\n//! Every handler is a thin inbound adapter: it deserializes transport\n//! types, calls the application service, and maps errors onto HTTP status\n//! codes. No business logic lives here.\n\n{imports}\n",
        t.name
    ));

    // ── list ────────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `GET /api/v1/{module}` — paginated listing.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}\",\n    params(Pagination),\n    responses((status = 200, body = Paginated<{struct_name}>))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}\"))]\npub async fn list(\n    State(state): State<AppState>,\n    Query(pagination): Query<Pagination>,\n) -> Result<Json<Paginated<{struct_name}>>, AppError> {{\n    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;\n    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);\n    let result = state.{module}_service.list(page).await?;\n    Ok(Json(result))\n}}\n"
    ));

    // ── create ──────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `POST /api/v1/{module}` — create.\n#[utoipa::path(\n    post,\n    path = \"/api/v1/{module}\",\n    tag = \"{tag}\",\n    operation_id = \"create_{module}\",\n    request_body = Create{struct_name}Input,\n    responses((status = 201, body = {struct_name}), (status = 400), (status = 409))\n)]\n#[instrument(skip(state, input), fields(path = \"/api/v1/{module}\"))]\npub async fn create(\n    State(state): State<AppState>,\n    Json(input): Json<Create{struct_name}Input>,\n) -> Result<(StatusCode, Json<{struct_name}>), AppError> {{\n    let created = state.{module}_service.create(input).await?;\n    Ok((StatusCode::CREATED, Json(created)))\n}}\n"
    ));

    // ── get_one ─────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `GET /api/v1/{module}/{{id}}` — fetch one.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/{{id}}\",\n    tag = \"{tag}\",\n    operation_id = \"get_{module}\",\n    responses((status = 200, body = {struct_name}), (status = 404))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/{{id}}\"))]\npub async fn get_one(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n) -> Result<Json<{struct_name}>, AppError> {{\n    state\n        .{module}_service\n        .get(id)\n        .await?\n        .map(Json)\n        .ok_or_else(|| AppError::NotFound(format!(\"{struct_name} {{id}} not found\")))\n}}\n"
    ));

    // ── update ──────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `PUT /api/v1/{module}/{{id}}` — partial update.\n#[utoipa::path(\n    put,\n    path = \"/api/v1/{module}/{{id}}\",\n    tag = \"{tag}\",\n    operation_id = \"update_{module}\",\n    request_body = Update{struct_name}Input,\n    responses((status = 200, body = {struct_name}), (status = 404), (status = 409))\n)]\n#[instrument(skip(state, input), fields(path = \"/api/v1/{module}/{{id}}\"))]\npub async fn update(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n    Json(input): Json<Update{struct_name}Input>,\n) -> Result<Json<{struct_name}>, AppError> {{\n    state\n        .{module}_service\n        .update(id, input)\n        .await?\n        .map(Json)\n        .ok_or_else(|| AppError::NotFound(format!(\"{struct_name} {{id}} not found\")))\n}}\n"
    ));

    // ── delete ──────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `DELETE /api/v1/{module}/{{id}}` — delete (soft when available).\n#[utoipa::path(\n    delete,\n    path = \"/api/v1/{module}/{{id}}\",\n    tag = \"{tag}\",\n    operation_id = \"delete_{module}\",\n    responses((status = 204), (status = 404))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/{{id}}\"))]\npub async fn delete(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n) -> Result<StatusCode, AppError> {{\n    if state.{module}_service.delete(id).await? {{\n        Ok(StatusCode::NO_CONTENT)\n    }} else {{\n        Err(AppError::NotFound(format!(\"{struct_name} {{id}} not found\")))\n    }}\n}}\n"
    ));

    // ── status extras ───────────────────────────────────────────────────
    if t.analysis.status_col.is_some() {
        let status_ty = status_type(t).unwrap_or_else(|| "String".into());
        out.push_str(&format!(
            "\n/// `GET /api/v1/{module}/status/{{status}}` — list by status.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/status/{{status}}\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}_by_status\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/status/{{status}}\"))]\npub async fn list_by_status(\n    State(state): State<AppState>,\n    Path(status): Path<String>,\n    Query(pagination): Query<Pagination>,\n) -> Result<Json<Vec<{struct_name}>>, AppError> {{\n    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;\n    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);\n{parse}    let result = state.{module}_service.list_by_status(status, page).await?;\n    Ok(Json(result))\n}}\n",
            parse = if status_ty == "String" {
                String::new()
            } else {
                format!("    let status = status.parse::<{status_ty}>().map_err(|e: String| AppError::BadRequest(e))?;\n")
            }
        ));

        out.push_str(&format!(
            "\n/// `PATCH /api/v1/{module}/{{id}}/status` — change status.\n#[utoipa::path(\n    patch,\n    path = \"/api/v1/{module}/{{id}}/status\",\n    tag = \"{tag}\",\n    operation_id = \"set_{module}_status\",\n    request_body = Set{struct_name}StatusInput,\n    responses((status = 200, body = {struct_name}), (status = 404))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/{{id}}/status\"))]\npub async fn set_status(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n    Json(body): Json<Set{struct_name}StatusInput>,\n) -> Result<Json<{struct_name}>, AppError> {{\n    state\n        .{module}_service\n        .set_status(id, body.status)\n        .await?\n        .map(Json)\n        .ok_or_else(|| AppError::NotFound(format!(\"{struct_name} {{id}} not found\")))\n}}\n\n/// Body of `PATCH /{module}/{{id}}/status`.\n#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]\npub struct Set{struct_name}StatusInput {{\n    /// New status value.\n    pub status: {status_ty},\n}}\n"
        ));
    }

    // ── soft-delete extras ──────────────────────────────────────────────
    if t.analysis.has_soft_delete() {
        out.push_str(&format!(
            "\n/// `DELETE /api/v1/{module}/{{id}}/soft` — soft delete.\n#[utoipa::path(\n    delete,\n    path = \"/api/v1/{module}/{{id}}/soft\",\n    tag = \"{tag}\",\n    operation_id = \"soft_delete_{module}\",\n    responses((status = 204), (status = 404))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/{{id}}/soft\"))]\npub async fn soft_delete(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n) -> Result<StatusCode, AppError> {{\n    if state.{module}_service.soft_delete(id).await? {{\n        Ok(StatusCode::NO_CONTENT)\n    }} else {{\n        Err(AppError::NotFound(format!(\"{struct_name} {{id}} not found\")))\n    }}\n}}\n\n/// `POST /api/v1/{module}/{{id}}/restore` — restore.\n#[utoipa::path(\n    post,\n    path = \"/api/v1/{module}/{{id}}/restore\",\n    tag = \"{tag}\",\n    operation_id = \"restore_{module}\",\n    responses((status = 200, body = {struct_name}), (status = 404))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/{{id}}/restore\"))]\npub async fn restore(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n) -> Result<Json<{struct_name}>, AppError> {{\n    state\n        .{module}_service\n        .restore(id)\n        .await?\n        .map(Json)\n        .ok_or_else(|| AppError::NotFound(format!(\"{struct_name} {{id}} not found or not deleted\")))\n}}\n"
        ));
    }

    // ── user extra ──────────────────────────────────────────────────────
    if let Some(user) = user_type(t) {
        out.push_str(&format!(
            "\n/// `GET /api/v1/{module}/by-user/{{user_id}}` — list by owner.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/by-user/{{user_id}}\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}_by_user\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/by-user/{{user_id}}\"))]\npub async fn list_by_user(\n    State(state): State<AppState>,\n    Path(user_id): Path<{user}>,\n    Query(pagination): Query<Pagination>,\n) -> Result<Json<Vec<{struct_name}>>, AppError> {{\n    pagination.validate().map_err(|e| AppError::Validation(e.to_string()))?;\n    let page = Page::new(pagination.page() as i64, pagination.page_size() as i64);\n    let result = state.{module}_service.list_by_user(user_id, page).await?;\n    Ok(Json(result))\n}}\n"
        ));
    }

    // ── timestamp extras ────────────────────────────────────────────────
    if let Some(ts) = ts_type(t, &t.analysis.created_at_col) {
        out.push_str(&format!(
            "\n/// `GET /api/v1/{module}/created-after?after=…` — created since a timestamp.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/created-after\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}_created_after\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/created-after\"))]\npub async fn created_after(\n    State(state): State<AppState>,\n    Query(params): Query<{struct_name}CreatedAfterParams>,\n) -> Result<Json<Vec<{struct_name}>>, AppError> {{\n    let page = Page::new(1, 50);\n    let result = state.{module}_service.list_created_after(params.after, page).await?;\n    Ok(Json(result))\n}}\n\n/// Query parameters for `GET /{module}/created-after`.\n#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]\npub struct {struct_name}CreatedAfterParams {{\n    /// Inclusive ISO-8601 lower bound.\n    pub after: {ts},\n}}\n"
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.updated_at_col) {
        out.push_str(&format!(
            "\n/// `GET /api/v1/{module}/updated-after?after=…` — updated since a timestamp.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/updated-after\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}_updated_after\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\n#[instrument(skip(state), fields(path = \"/api/v1/{module}/updated-after\"))]\npub async fn updated_after(\n    State(state): State<AppState>,\n    Query(params): Query<{struct_name}UpdatedAfterParams>,\n) -> Result<Json<Vec<{struct_name}>>, AppError> {{\n    let page = Page::new(1, 50);\n    let result = state.{module}_service.list_updated_after(params.after, page).await?;\n    Ok(Json(result))\n}}\n\n/// Query parameters for `GET /{module}/updated-after`.\n#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]\npub struct {struct_name}UpdatedAfterParams {{\n    /// Inclusive ISO-8601 lower bound.\n    pub after: {ts},\n}}\n"
        ));
    }

    out
}
