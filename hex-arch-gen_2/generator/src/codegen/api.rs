//! API layer codegen — per-table DTO/param types and Axum handlers, plus
//! the shared router and OpenAPI document. Every handler dispatches
//! through the enterprise buses.

use std::path::Path;

use anyhow::Result;

use crate::codegen::domain::{status_type, ts_type, user_type};
use crate::codegen::{write_file, ImportSet};
use crate::ir::{Schema, Table};

/// Writes `src/api/<table>/` for one table.
pub fn write_table(root: &Path, t: &Table) -> Result<()> {
    let dir = root.join("src").join("api").join(&t.module_name);
    write_file(&dir.join("mod.rs"), &mod_rs(t))?;
    write_file(&dir.join("dto.rs"), &dto_rs(t))?;
    write_file(&dir.join("handler.rs"), &handler_rs(t))?;
    Ok(())
}

/// Writes the shared API artefacts: `mod.rs`, `routes.rs`, `openapi.rs`.
pub fn write_shared(schema: &Schema, root: &Path) -> Result<()> {
    let api = root.join("src").join("api");
    write_file(&api.join("mod.rs"), &api_mod_rs(schema))?;
    write_file(&api.join("routes.rs"), &routes_rs(schema))?;
    write_file(&api.join("openapi.rs"), &openapi_rs(schema))?;
    Ok(())
}

fn mod_rs(t: &Table) -> String {
    format!(
        "//! HTTP surface for the `{}` context.\n\npub mod dto;\npub mod handler;\n",
        t.name
    )
}

fn api_mod_rs(schema: &Schema) -> String {
    let mut body = String::from(
        "//! Inbound adapters — axum handlers, route table and OpenAPI document.\n\npub mod openapi;\npub mod routes;\npub mod state;\n",
    );
    for t in &schema.tables {
        body.push_str(&format!("pub mod {};\n", t.module_name));
    }
    body
}

// ─────────────────────────────────────────────────────────────────────────────
// dto.rs
// ─────────────────────────────────────────────────────────────────────────────

fn dto_rs(t: &Table) -> String {
    let struct_name = &t.struct_name;

    let mut out = String::new();
    out.push_str(&format!(
        "//! Query/body parameter types for the `{}` HTTP surface.\n\n",
        t.name
    ));
    out.push_str("use serde::Deserialize;\nuse utoipa::ToSchema;\n");
    if t.analysis.created_at_col.is_some() || t.analysis.updated_at_col.is_some() {
        let mut cols: Vec<&crate::ir::Column> = Vec::new();
        for which in [&t.analysis.created_at_col, &t.analysis.updated_at_col] {
            if let Some(c) = t.columns.iter().find(|c| Some(&c.name) == which.as_ref()) {
                cols.push(c);
            }
        }
        for line in ImportSet::from_cols(cols).render() {
            out.push_str(&line);
            out.push('\n');
        }
    }
    if status_type(t).is_some() {
        for (_, rust_name) in t.used_enums() {
            out.push_str(&format!(
                "use crate::domain::{}::enums::{};\n",
                t.module_name, rust_name
            ));
        }
    }
    out.push_str("use crate::enterprise::repository::Page;\n");

    out.push_str(&format!(
        "\n/// Pagination query parameters for `{}` endpoints.\n#[derive(Debug, Clone, Deserialize, ToSchema)]\npub struct {}PageParams {{\n    /// 1-based page number (default 1).\n    pub page: Option<i64>,\n    /// Page size, clamped to 1..=200 (default 20).\n    pub per_page: Option<i64>,\n}}\n\nimpl {}PageParams {{\n    /// Converts loose query parameters into an enterprise [`Page`].\n    pub fn to_page(&self) -> Page {{\n        Page::new(self.page.unwrap_or(1), self.per_page.unwrap_or(20))\n    }}\n}}\n",
        t.name, struct_name, struct_name
    ));

    if let Some(status) = status_type(t) {
        out.push_str(&format!(
            "\n/// Body of `PATCH /{}/{{id}}/status`.\n#[derive(Debug, Clone, Deserialize, ToSchema)]\npub struct Set{1}StatusInput {{\n    /// New status value.\n    pub status: {status},\n}}\n",
            t.module_name, struct_name
        ));
    }

    if let Some(ts) = ts_type(t, &t.analysis.created_at_col) {
        out.push_str(&format!(
            "\n/// Query parameters for `GET /{}/created-after`.\n#[derive(Debug, Clone, Deserialize, ToSchema)]\npub struct {}CreatedAfterParams {{\n    /// Inclusive ISO-8601 lower bound.\n    pub after: {},\n}}\n",
            t.module_name, struct_name, ts
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.updated_at_col) {
        out.push_str(&format!(
            "\n/// Query parameters for `GET /{}/updated-after`.\n#[derive(Debug, Clone, Deserialize, ToSchema)]\npub struct {}UpdatedAfterParams {{\n    /// Inclusive ISO-8601 lower bound.\n    pub after: {},\n}}\n",
            t.module_name, struct_name, ts
        ));
    }
    out
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

    let mut messaging_uses: Vec<String> = vec![
        format!("Create{struct_name}Command"),
        format!("Delete{struct_name}Command"),
        format!("Get{struct_name}Query"),
        format!("List{struct_name}Query"),
        format!("Update{struct_name}Command"),
    ];
    if t.analysis.status_col.is_some() {
        messaging_uses.push(format!("List{struct_name}ByStatusQuery"));
        messaging_uses.push(format!("Set{struct_name}StatusCommand"));
    }
    if t.analysis.soft_flag_col.is_some() || t.analysis.deleted_at_col.is_some() {
        messaging_uses.push(format!("SoftDelete{struct_name}Command"));
        messaging_uses.push(format!("Restore{struct_name}Command"));
    }
    if t.analysis.user_col.is_some() {
        messaging_uses.push(format!("List{struct_name}ByUserQuery"));
    }
    if t.analysis.created_at_col.is_some() {
        messaging_uses.push(format!("List{struct_name}CreatedAfterQuery"));
    }
    if t.analysis.updated_at_col.is_some() {
        messaging_uses.push(format!("List{struct_name}UpdatedAfterQuery"));
    }
    messaging_uses.sort();

    let mut out = String::new();
    out.push_str(&format!(
        "//! Axum handlers for the `{}` context.\n//!\n//! Every handler is a thin inbound adapter: it deserializes transport\n//! types, dispatches through the enterprise buses and maps errors onto\n//! HTTP status codes. No business logic lives here.\n\n",
        t.name
    ));
    out.push_str("use axum::extract::{Path, Query, State};\n");
    out.push_str("use axum::http::StatusCode;\n");
    out.push_str("use axum::Json;\n");
    // Handlers name the PK type and (when present) the user-reference type;
    // timestamps travel inside DTO structs, so no chrono import is needed.
    let mut handler_cols: Vec<&crate::ir::Column> = vec![pk];
    if let Some(user) = t.analysis.user_col.as_ref() {
        if let Some(c) = t.columns.iter().find(|c| &c.name == user) {
            handler_cols.push(c);
        }
    }
    for line in ImportSet::from_cols(handler_cols).render() {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!(
        "use crate::api::state::AppState;\nuse crate::api::{}::dto::{{",
        module
    ));
    let mut dto_uses: Vec<String> = vec![format!("{struct_name}PageParams")];
    if t.analysis.status_col.is_some() {
        dto_uses.push(format!("Set{struct_name}StatusInput"));
    }
    if t.analysis.created_at_col.is_some() {
        dto_uses.push(format!("{struct_name}CreatedAfterParams"));
    }
    if t.analysis.updated_at_col.is_some() {
        dto_uses.push(format!("{struct_name}UpdatedAfterParams"));
    }
    dto_uses.sort();
    out.push_str(&dto_uses.join(", "));
    out.push_str("};\n");
    out.push_str(&format!(
        "use crate::application::{}_messaging::{{{}}};\n",
        module,
        messaging_uses.join(", ")
    ));
    out.push_str(&format!(
        "use crate::domain::{}::entity::{};\nuse crate::domain::{}::input::{{Create{1}Input, Update{1}Input}};\nuse crate::error::ApiError;\n",
        module, struct_name, module
    ));
    if t.analysis.status_col.is_some() {
        for (_, rust_name) in t.used_enums() {
            out.push_str(&format!(
                "use crate::domain::{}::enums::{};\n",
                module, rust_name
            ));
        }
    }

    // ── list ────────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `GET /api/v1/{module}` — paginated listing (query bus).\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\npub async fn list(\n    State(state): State<AppState>,\n    Query(params): Query<{struct_name}PageParams>,\n) -> Result<Json<serde_json::Value>, ApiError> {{\n    let result = state.qry.ask(List{struct_name}Query {{ page: params.to_page() }}).await?;\n    Ok(Json(serde_json::json!({{\n        \"items\": result.items,\n        \"total\": result.total,\n        \"page\": result.page,\n        \"per_page\": result.per_page,\n    }})))\n}}\n"
    ));

    // ── create ──────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `POST /api/v1/{module}` — create (command bus).\n#[utoipa::path(\n    post,\n    path = \"/api/v1/{module}\",\n    tag = \"{tag}\",\n    operation_id = \"create_{module}\",\n    request_body = Create{struct_name}Input,\n    responses((status = 201, body = {struct_name}), (status = 400), (status = 409))\n)]\npub async fn create(\n    State(state): State<AppState>,\n    Json(input): Json<Create{struct_name}Input>,\n) -> Result<(StatusCode, Json<{struct_name}>), ApiError> {{\n    let created = state.cmd.send(Create{struct_name}Command {{ input }}).await?;\n    Ok((StatusCode::CREATED, Json(created)))\n}}\n"
    ));

    // ── get one ─────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `GET /api/v1/{module}/{{id}}` — fetch one (query bus).\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/{{id}}\",\n    tag = \"{tag}\",\n    operation_id = \"get_{module}\",\n    responses((status = 200, body = {struct_name}), (status = 404))\n)]\npub async fn get_one(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n) -> Result<Json<{struct_name}>, ApiError> {{\n    state\n        .qry\n        .ask(Get{struct_name}Query {{ id }})\n        .await?\n        .map(Json)\n        .ok_or(ApiError::NotFound)\n}}\n"
    ));

    // ── update ──────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `PUT /api/v1/{module}/{{id}}` — partial update (command bus).\n#[utoipa::path(\n    put,\n    path = \"/api/v1/{module}/{{id}}\",\n    tag = \"{tag}\",\n    operation_id = \"update_{module}\",\n    request_body = Update{struct_name}Input,\n    responses((status = 200, body = {struct_name}), (status = 404), (status = 409))\n)]\npub async fn update(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n    Json(input): Json<Update{struct_name}Input>,\n) -> Result<Json<{struct_name}>, ApiError> {{\n    state\n        .cmd\n        .send(Update{struct_name}Command {{ id, input }})\n        .await?\n        .map(Json)\n        .ok_or(ApiError::NotFound)\n}}\n"
    ));

    // ── delete ──────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n/// `DELETE /api/v1/{module}/{{id}}` — hard delete (command bus).\n#[utoipa::path(\n    delete,\n    path = \"/api/v1/{module}/{{id}}\",\n    tag = \"{tag}\",\n    operation_id = \"delete_{module}\",\n    responses((status = 204), (status = 404))\n)]\npub async fn delete(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n) -> Result<StatusCode, ApiError> {{\n    if state.cmd.send(Delete{struct_name}Command {{ id }}).await? {{\n        Ok(StatusCode::NO_CONTENT)\n    }} else {{\n        Err(ApiError::NotFound)\n    }}\n}}\n"
    ));

    // ── status extras ───────────────────────────────────────────────────
    if t.analysis.status_col.is_some() {
        out.push_str(&format!(
            "\n/// `GET /api/v1/{module}/status/{{status}}` — list by status (query bus).\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/status/{{status}}\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}_by_status\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\npub async fn list_by_status(\n    State(state): State<AppState>,\n    Path(status): Path<String>,\n    Query(params): Query<{struct_name}PageParams>,\n) -> Result<Json<Vec<{struct_name}>>, ApiError> {{\n{{status_parse}}    let result = state\n        .qry\n        .ask(List{struct_name}ByStatusQuery {{ status, page: params.to_page() }})\n        .await?;\n    Ok(Json(result))\n}}\n"
        ));

        let parse = status_type(t)
            .filter(|ty| ty != "String")
            .map(|ty| {
                format!(
                    "    let status = status.parse::<{ty}>().map_err(ApiError::BadRequest)?;\n"
                )
            })
            .unwrap_or_default();
        out = out.replace("{status_parse}", &parse);

        out.push_str(&format!(
            "\n/// `PATCH /api/v1/{module}/{{id}}/status` — change status (command bus).\n#[utoipa::path(\n    patch,\n    path = \"/api/v1/{module}/{{id}}/status\",\n    tag = \"{tag}\",\n    operation_id = \"set_{module}_status\",\n    request_body = Set{struct_name}StatusInput,\n    responses((status = 200, body = {struct_name}), (status = 404))\n)]\npub async fn set_status(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n    Json(body): Json<Set{struct_name}StatusInput>,\n) -> Result<Json<{struct_name}>, ApiError> {{\n    state\n        .cmd\n        .send(Set{struct_name}StatusCommand {{ id, status: body.status }})\n        .await?\n        .map(Json)\n        .ok_or(ApiError::NotFound)\n}}\n"
        ));
    }

    // ── soft delete extras ──────────────────────────────────────────────
    if t.analysis.soft_flag_col.is_some() || t.analysis.deleted_at_col.is_some() {
        out.push_str(&format!(
            "\n/// `DELETE /api/v1/{module}/{{id}}/soft` — soft delete (command bus).\n#[utoipa::path(\n    delete,\n    path = \"/api/v1/{module}/{{id}}/soft\",\n    tag = \"{tag}\",\n    operation_id = \"soft_delete_{module}\",\n    responses((status = 204), (status = 404))\n)]\npub async fn soft_delete(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n) -> Result<StatusCode, ApiError> {{\n    if state.cmd.send(SoftDelete{struct_name}Command {{ id }}).await? {{\n        Ok(StatusCode::NO_CONTENT)\n    }} else {{\n        Err(ApiError::NotFound)\n    }}\n}}\n\n/// `POST /api/v1/{module}/{{id}}/restore` — restore (command bus).\n#[utoipa::path(\n    post,\n    path = \"/api/v1/{module}/{{id}}/restore\",\n    tag = \"{tag}\",\n    operation_id = \"restore_{module}\",\n    responses((status = 200, body = {struct_name}), (status = 404))\n)]\npub async fn restore(\n    State(state): State<AppState>,\n    Path(id): Path<{pk_ty}>,\n) -> Result<Json<{struct_name}>, ApiError> {{\n    state\n        .cmd\n        .send(Restore{struct_name}Command {{ id }})\n        .await?\n        .map(Json)\n        .ok_or(ApiError::NotFound)\n}}\n"
        ));
    }

    // ── user extra ──────────────────────────────────────────────────────
    if let Some(user) = user_type(t) {
        out.push_str(&format!(
            "\n/// `GET /api/v1/{module}/by-user/{{user_id}}` — list by owner (query bus).\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/by-user/{{user_id}}\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}_by_user\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\npub async fn list_by_user(\n    State(state): State<AppState>,\n    Path(user_id): Path<{user}>,\n    Query(params): Query<{struct_name}PageParams>,\n) -> Result<Json<Vec<{struct_name}>>, ApiError> {{\n    let result = state\n        .qry\n        .ask(List{struct_name}ByUserQuery {{ user_id, page: params.to_page() }})\n        .await?;\n    Ok(Json(result))\n}}\n"
        ));
    }

    // ── timestamp extras ────────────────────────────────────────────────
    if let Some(ts) = ts_type(t, &t.analysis.created_at_col) {
        out.push_str(&format!(
            "\n/// `GET /api/v1/{module}/created-after` — created since a timestamp.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/created-after\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}_created_after\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\npub async fn created_after(\n    State(state): State<AppState>,\n    Query(params): Query<{struct_name}CreatedAfterParams>,\n) -> Result<Json<Vec<{struct_name}>>, ApiError> {{\n    let result = state\n        .qry\n        .ask(List{struct_name}CreatedAfterQuery {{ ts: params.after, page: params_page(&params) }})\n        .await?;\n    Ok(Json(result))\n}}\n"
        ));
        // The created-after params only carry `after`; pagination comes from
        // a shared page struct is intentionally omitted here — use defaults.
        out = out.replace("params_page(&params)", "crate::enterprise::repository::Page::new(1, 50)");
        let _ = ts;
    }
    if let Some(ts) = ts_type(t, &t.analysis.updated_at_col) {
        out.push_str(&format!(
            "\n/// `GET /api/v1/{module}/updated-after` — updated since a timestamp.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/{module}/updated-after\",\n    tag = \"{tag}\",\n    operation_id = \"list_{module}_updated_after\",\n    responses((status = 200, body = Vec<{struct_name}>))\n)]\npub async fn updated_after(\n    State(state): State<AppState>,\n    Query(params): Query<{struct_name}UpdatedAfterParams>,\n) -> Result<Json<Vec<{struct_name}>>, ApiError> {{\n    let result = state\n        .qry\n        .ask(List{struct_name}UpdatedAfterQuery {{ ts: params.after, page: crate::enterprise::repository::Page::new(1, 50) }})\n        .await?;\n    Ok(Json(result))\n}}\n"
        ));
        let _ = ts;
    }

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// routes.rs
// ─────────────────────────────────────────────────────────────────────────────

fn routes_rs(schema: &Schema) -> String {
    let mut body = String::from(
        "//! HTTP route table for the generated API.\n//!\n//! State is attached in `main` (after Swagger UI is merged), so this\n//! function stays a pure description of the `/api/v1` surface.\n\nuse axum::Router;\n\n",
    );
    for t in &schema.tables {
        body.push_str(&format!(
            "use crate::api::{}::handler as {};\n",
            t.module_name, t.module_name
        ));
    }
    body.push_str("use crate::api::state::AppState;\n\n/// Builds the `/api/v1` routes.\npub fn router() -> Router<AppState> {\n    Router::new()\n");

    for t in &schema.tables {
        let m = &t.module_name;
        body.push_str(&format!(
            "        .route(\"/{m}\", axum::routing::get({m}::list).post({m}::create))\n"
        ));
        body.push_str(&format!(
            "        .route(\"/{m}/{{id}}\", axum::routing::get({m}::get_one).put({m}::update).delete({m}::delete))\n"
        ));
        if t.analysis.status_col.is_some() {
            body.push_str(&format!(
                "        .route(\"/{m}/status/{{status}}\", axum::routing::get({m}::list_by_status))\n"
            ));
            body.push_str(&format!(
                "        .route(\"/{m}/{{id}}/status\", axum::routing::patch({m}::set_status))\n"
            ));
        }
        if t.analysis.soft_flag_col.is_some() || t.analysis.deleted_at_col.is_some() {
            body.push_str(&format!(
                "        .route(\"/{m}/{{id}}/soft\", axum::routing::delete({m}::soft_delete))\n"
            ));
            body.push_str(&format!(
                "        .route(\"/{m}/{{id}}/restore\", axum::routing::post({m}::restore))\n"
            ));
        }
        if t.analysis.user_col.is_some() {
            body.push_str(&format!(
                "        .route(\"/{m}/by-user/{{user_id}}\", axum::routing::get({m}::list_by_user))\n"
            ));
        }
        if t.analysis.created_at_col.is_some() {
            body.push_str(&format!(
                "        .route(\"/{m}/created-after\", axum::routing::get({m}::created_after))\n"
            ));
        }
        if t.analysis.updated_at_col.is_some() {
            body.push_str(&format!(
                "        .route(\"/{m}/updated-after\", axum::routing::get({m}::updated_after))\n"
            ));
        }
    }
    body.push_str("}\n");
    body
}

// ─────────────────────────────────────────────────────────────────────────────
// openapi.rs
// ─────────────────────────────────────────────────────────────────────────────

fn openapi_rs(schema: &Schema) -> String {
    let mut paths = String::new();
    let mut schemas = String::new();
    let mut tags = String::new();

    for t in &schema.tables {
        let m = &t.module_name;
        let n = &t.struct_name;
        paths.push_str(&format!(
            "        crate::api::{m}::handler::list,\n        crate::api::{m}::handler::get_one,\n        crate::api::{m}::handler::create,\n        crate::api::{m}::handler::update,\n        crate::api::{m}::handler::delete,\n"
        ));
        if t.analysis.status_col.is_some() {
            paths.push_str(&format!(
                "        crate::api::{m}::handler::list_by_status,\n        crate::api::{m}::handler::set_status,\n"
            ));
        }
        if t.analysis.soft_flag_col.is_some() || t.analysis.deleted_at_col.is_some() {
            paths.push_str(&format!(
                "        crate::api::{m}::handler::soft_delete,\n        crate::api::{m}::handler::restore,\n"
            ));
        }
        if t.analysis.user_col.is_some() {
            paths.push_str(&format!(
                "        crate::api::{m}::handler::list_by_user,\n"
            ));
        }
        if t.analysis.created_at_col.is_some() {
            paths.push_str(&format!(
                "        crate::api::{m}::handler::created_after,\n"
            ));
        }
        if t.analysis.updated_at_col.is_some() {
            paths.push_str(&format!(
                "        crate::api::{m}::handler::updated_after,\n"
            ));
        }

        schemas.push_str(&format!(
            "        crate::domain::{m}::entity::{n},\n        crate::domain::{m}::input::Create{n}Input,\n        crate::domain::{m}::input::Update{n}Input,\n        crate::api::{m}::dto::{n}PageParams,\n"
        ));
        if t.analysis.status_col.is_some() {
            schemas.push_str(&format!(
                "        crate::api::{m}::dto::Set{n}StatusInput,\n"
            ));
        }
        if t.analysis.created_at_col.is_some() {
            schemas.push_str(&format!(
                "        crate::api::{m}::dto::{n}CreatedAfterParams,\n"
            ));
        }
        if t.analysis.updated_at_col.is_some() {
            schemas.push_str(&format!(
                "        crate::api::{m}::dto::{n}UpdatedAfterParams,\n"
            ));
        }

        tags.push_str(&format!(
            "        (name = \"{m}\", description = \"{} management\"),\n",
            t.name
        ));
    }

    format!(
        "//! OpenAPI 3 document, rendered by Swagger UI at `/swagger-ui`.\n\nuse utoipa::OpenApi;\n\n/// The generated OpenAPI document.\n#[derive(OpenApi)]\n#[openapi(\n    info(\n        title = \"Hexagonal Architecture API\",\n        version = \"1.0.0\",\n        description = \"Generated by hex-gen from a PostgreSQL schema. Writes go through the CommandBus, reads through the QueryBus.\"\n    ),\n    paths(\n{paths}    ),\n    components(schemas(\n{schemas}    )),\n    tags(\n{tags}    )\n)]\npub struct ApiDoc;\n"
    )
}
