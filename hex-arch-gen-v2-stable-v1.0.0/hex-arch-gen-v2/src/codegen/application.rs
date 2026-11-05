// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Application layer codegen — use-case services, DTOs, ports.
//!
//! Emits the following per-table artefacts:
//!
//! * `src/application/dto/<table>_dto.rs`   — `Create…Input`, `Update…Input`
//! * `src/application/services/<table>_service.rs` — service implementing the
//!   inbound port on top of the repository port.
//!
//! And the following shared artefacts (written once, in `write_mod`):
//!
//! * `src/application/ports/repository.rs` — per-table repository port traits
//! * `src/application/ports/bus.rs`        — `EventBus` trait (in-process)
//! * `src/application/sagas/mod.rs`        — saga step registry (stub)
//! * `src/application/mod.rs`              — module manifest
//!
//! Templates use `@TOKEN@` substitution (rather than `format!`) so the
//! generated braces stay literal and arg-order bugs cannot happen.

use std::path::Path;

use anyhow::Result;

use crate::codegen::domain_helpers::{status_type, ts_type, user_type};
use crate::codegen::{write_file, ImportSet};
use crate::ir::{Schema, Table};

/// Writes the per-table artefacts under `src/application/`.
pub fn write_table(_schema: &Schema, root: &Path, t: &Table) -> Result<()> {
    let dto_dir = root.join("src").join("application").join("dto");
    let svc_dir = root.join("src").join("application").join("services");
    write_file(
        &dto_dir.join(format!("{}_dto.rs", t.module_name)),
        &dto_rs(t),
    )?;
    write_file(
        &svc_dir.join(format!("{}_service.rs", t.module_name)),
        &service_rs(t),
    )?;
    Ok(())
}

/// Writes the aggregate `mod.rs` files + the shared ports.
pub fn write_mod(schema: &Schema, root: &Path) -> Result<()> {
    let app = root.join("src").join("application");

    // ── application/mod.rs ──────────────────────────────────────────────
    let mut body = String::from(
        "//! Application layer — use-case services, DTOs, ports (the hexagon core).\n//!\n//! Services depend only on port traits; the presentation layer calls them,\n//! and infrastructure provides concrete adapters.\n\npub mod dto;\npub mod ports;\npub mod sagas;\npub mod services;\n",
    );
    write_file(&app.join("mod.rs"), &body)?;

    // ── application/dto/mod.rs ──────────────────────────────────────────
    body = String::from(
        "//! Data Transfer Objects — request/response shapes for the HTTP surface.\n\n",
    );
    for t in &schema.tables {
        body.push_str(&format!("pub mod {}_dto;\n", t.module_name));
    }
    write_file(&app.join("dto").join("mod.rs"), &body)?;

    // ── application/services/mod.rs ─────────────────────────────────────
    body = String::from(
        "//! Use-case services — implement the inbound port on top of repository + bus ports.\n\n",
    );
    for t in &schema.tables {
        body.push_str(&format!("pub mod {}_service;\n", t.module_name));
    }
    // Re-export the logging event handler + a couple of canned events.
    body.push_str(
        "\nuse async_trait::async_trait;\nuse std::sync::Arc;\n\nuse crate::application::ports::bus::{DomainEvent, EventBus};\n\n/// A logging event handler that prints every event to the tracing log.\n#[derive(Default)]\npub struct LoggingEventHandler;\n\n#[async_trait]\nimpl crate::application::ports::bus::EventHandler for LoggingEventHandler {\n    async fn handle(&self, event: &dyn DomainEvent) {\n        tracing::info!(event_name = event.name(), \"event handled (log)\");\n    }\n}\n\n/// A canned `UserCreated` event — emitted by the user service after a\n/// successful create.\n#[derive(Debug, Clone)]\npub struct UserCreated {\n    pub id: uuid::Uuid,\n    pub occurred_at: chrono::DateTime<chrono::Utc>,\n}\n\nimpl DomainEvent for UserCreated {\n    fn name(&self) -> &'static str {\n        \"user_created\"\n    }\n}\n\n/// A canned `ProductCreated` event for parity with the desired state.\n#[derive(Debug, Clone)]\npub struct ProductCreated {\n    pub id: uuid::Uuid,\n    pub occurred_at: chrono::DateTime<chrono::Utc>,\n}\n\nimpl DomainEvent for ProductCreated {\n    fn name(&self) -> &'static str {\n        \"product_created\"\n    }\n}\n",
    );
    write_file(&app.join("services").join("mod.rs"), &body)?;

    // ── application/ports/{mod,bus,repository}.rs ───────────────────────
    write_file(
        &app.join("ports").join("mod.rs"),
        "//! Ports — outbound contracts owned by the application layer.\n//!\n//! * `bus`        — `EventBus` for in-process domain event publishing.\n//! * `repository`  — per-entity repository port traits.\n\npub mod bus;\npub mod repository;\n",
    )?;

    // bus.rs — EventBus trait.
    write_file(&app.join("ports").join("bus.rs"), BUS_RS)?;

    // repository.rs — base Repository<T, ID> + per-table trait.
    write_file(
        &app.join("ports").join("repository.rs"),
        &repository_port_rs(schema),
    )?;

    // ── application/sagas/mod.rs (stub) ─────────────────────────────────
    write_file(
        &app.join("sagas").join("mod.rs"),
        "//! Saga step registry (stub).\n//!\n//! The v0.6.0 desired-state persists sagas in PostgreSQL with a\n//! compensation registry; here we expose the canonical kind constant\n//! and a step list helper so handlers compile.\n\nuse sqlx::PgPool;\n\nuse crate::core::saga::SagaStep;\n\n/// Canonical saga kind for user onboarding.\npub const SAGA_KIND: &str = \"user_onboarding\";\n\n/// Returns the ordered saga steps for user onboarding.\n#[must_use]\npub fn user_onboarding_steps(_pool: PgPool) -> Vec<SagaStep> {\n    vec![\n        SagaStep { name: \"create_user_record\".to_string() },\n        SagaStep { name: \"provision_mailbox\".to_string() },\n        SagaStep { name: \"send_welcome_email\".to_string() },\n    ]\n}\n",
    )?;

    Ok(())
}

/// Replaces every `@KEY@` occurrence with its value.
fn sub(template: &str, pairs: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (key, value) in pairs {
        out = out.replace(key, value);
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// dto.rs
// ─────────────────────────────────────────────────────────────────────────────

fn dto_rs(t: &Table) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "//! Request/response DTOs for the `{}` context.\n//!\n//! `Create{0}Input` covers every client-writable column of an INSERT\n//! (server-generated columns are filled by the database); `Update{0}Input`\n//! is fully optional so `PUT` behaves as a partial update.\n\n",
        t.struct_name
    ));

    let imports = ImportSet::from_cols(t.insert_cols().into_iter().chain(t.update_cols()));
    for line in imports.render() {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("use serde::Deserialize;\n");
    out.push_str("use utoipa::ToSchema;\n");
    for (_, rust_name) in t.used_enums() {
        out.push_str(&format!(
            "use crate::domain::entities::{}::{};\n",
            t.module_name, rust_name
        ));
    }

    // Create input
    let insert_cols = t.insert_cols();
    out.push_str(&format!(
        "\n/// Payload accepted by `POST /{}` — validated before persistence.\n#[derive(Debug, Clone, Deserialize, ToSchema)]\npub struct Create{}Input {{\n",
        t.module_name, t.struct_name
    ));
    for c in &insert_cols {
        out.push_str(&format!("    /// {}\n", c.doc_line()));
        if let Some(attr) = c.schema_attr() {
            out.push_str(&format!("    {attr}\n"));
        }
        if let Some(attr) = c.serde_rename() {
            out.push_str(&format!("    {attr}\n"));
        }
        out.push_str(&format!(
            "    pub {}: {},\n",
            c.field_ident,
            c.create_type()
        ));
    }
    out.push_str("}\n\n");

    // validate()
    out.push_str(&format!(
        "impl Create{}Input {{\n    /// Bean-style validation invoked by the application service.\n    pub fn validate(&self) -> Result<(), String> {{\n",
        t.struct_name
    ));
    for c in insert_cols
        .iter()
        .filter(|c| !c.nullable && matches!(c.kind, crate::ir::ColKind::Str))
    {
        out.push_str(&format!(
            "        if self.{}.trim().is_empty() {{\n            return Err(\"{} must not be empty\".to_string());\n        }}\n",
            c.field_ident, c.name
        ));
    }
    out.push_str("        Ok(())\n    }\n}\n\n");

    // Update input
    out.push_str(&format!(
        "/// Payload accepted by `PUT /{}/{{id}}` — every field optional.\n#[derive(Debug, Clone, Default, Deserialize, ToSchema)]\npub struct Update{}Input {{\n",
        t.module_name, t.struct_name
    ));
    for c in t.update_cols() {
        out.push_str(&format!("    /// Optional new value for `{}`.\n", c.name));
        if let Some(attr) = c.schema_attr() {
            out.push_str(&format!("    {attr}\n"));
        }
        if let Some(attr) = c.serde_rename() {
            out.push_str(&format!("    {attr}\n"));
        }
        out.push_str(&format!(
            "    pub {}: {},\n",
            c.field_ident,
            c.update_type()
        ));
    }
    out.push_str("}\n");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// service.rs
// ─────────────────────────────────────────────────────────────────────────────

fn service_rs(t: &Table) -> String {
    let pk = t.pk().expect("pk required for codegen");

    // Extra service methods (delegating to repository port).
    // The trait declares `Result<_, AppError>` so each method maps
    // `RepositoryError` into `AppError` via the `Into` impl on the
    // shared error type.
    let mut extra_impl = String::new();
    if let Some(status) = status_type(t) {
        extra_impl.push_str(&sub(
            r#"
    async fn list_by_status(&self, status: @STATUS@, page: Page) -> Result<Vec<@STRUCT@>, AppError> {
        Ok(self.repo.find_by_status(status, page).await?)
    }

    async fn set_status(&self, id: @PK@, status: @STATUS@) -> Result<Option<@STRUCT@>, AppError> {
        Ok(self.repo.set_status(id, status).await?)
    }
"#,
            &[
                ("@STATUS@", status),
                ("@STRUCT@", t.struct_name.clone()),
                ("@PK@", pk.kind.rust_type()),
            ],
        ));
    }
    if t.analysis.has_soft_delete() {
        extra_impl.push_str(&sub(
            r#"
    async fn soft_delete(&self, id: @PK@) -> Result<bool, AppError> {
        Ok(self.repo.soft_delete(id).await?)
    }

    async fn restore(&self, id: @PK@) -> Result<Option<@STRUCT@>, AppError> {
        Ok(self.repo.restore(id).await?)
    }
"#,
            &[
                ("@PK@", pk.kind.rust_type()),
                ("@STRUCT@", t.struct_name.clone()),
            ],
        ));
    }
    if let Some(user) = user_type(t) {
        extra_impl.push_str(&sub(
            r#"
    async fn list_by_user(&self, user_id: @USER@, page: Page) -> Result<Vec<@STRUCT@>, AppError> {
        Ok(self.repo.find_by_user(user_id, page).await?)
    }
"#,
            &[("@USER@", user), ("@STRUCT@", t.struct_name.clone())],
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.created_at_col) {
        extra_impl.push_str(&sub(
            r#"
    async fn list_created_after(&self, ts: @TS@, page: Page) -> Result<Vec<@STRUCT@>, AppError> {
        Ok(self.repo.find_created_after(ts, page).await?)
    }
"#,
            &[("@TS@", ts), ("@STRUCT@", t.struct_name.clone())],
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.updated_at_col) {
        extra_impl.push_str(&sub(
            r#"
    async fn list_updated_after(&self, ts: @TS@, page: Page) -> Result<Vec<@STRUCT@>, AppError> {
        Ok(self.repo.find_updated_after(ts, page).await?)
    }
"#,
            &[("@TS@", ts), ("@STRUCT@", t.struct_name.clone())],
        ));
    }

    let mut imports = String::new();
    let mut scan_cols: Vec<&crate::ir::Column> = vec![pk];
    scan_cols.extend(crate::codegen::domain_helpers::port_feature_cols(t));
    for line in ImportSet::from_cols(scan_cols).render() {
        imports.push_str(&line);
        imports.push('\n');
    }
    for (_, rust_name) in t.used_enums() {
        imports.push_str(&format!(
            "use crate::domain::entities::{}::{};\n",
            t.module_name, rust_name
        ));
    }

    sub(
        r#"//! Application service adapter for `@TABLE@` — the concrete inbound adapter
//! implementing the inbound use-case contract on top of the repository port.
//!
//! The service is where business rules live: payload validation, invariant
//! checks, and orchestration of the repository port. Swap-in adapters for
//! other stores only need to implement the repository port.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::instrument;

@TYPE_IMPORTS@use crate::application::dto::@MODULE@_dto::{Create@STRUCT@Input, Update@STRUCT@Input};
use crate::application::ports::repository::{@STRUCT@Repository, @STRUCT@ServicePort, Page, Paginated};
use crate::domain::entities::@MODULE@::@STRUCT@;
use crate::shared::error::AppError;

/// Concrete service for `@TABLE@`; injected into handlers by `main`.
#[derive(Clone)]
pub struct @STRUCT@Service {
    repo: Arc<dyn @STRUCT@Repository>,
}

impl @STRUCT@Service {
    /// Wires the service to its repository port.
    #[must_use]
    pub fn new(repo: Arc<dyn @STRUCT@Repository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl @STRUCT@ServicePort for @STRUCT@Service {
    #[instrument(skip(self, _input), fields(table = "@TABLE@", op = "create"))]
    async fn create(&self, _input: Create@STRUCT@Input) -> Result<@STRUCT@, AppError> {
        _input.validate().map_err(AppError::Validation)?;
        // Build entity from input via the trivial mapper.
        let mut entity = @STRUCT@::default();
@SET_FIELDS@        entity.validate().map_err(AppError::Validation)?;
        let created = self.repo.create(&entity).await?;
        Ok(created)
    }

    #[instrument(skip(self), fields(table = "@TABLE@", op = "get", id = ?id))]
    async fn get(&self, id: @PK@) -> Result<Option<@STRUCT@>, AppError> {
        Ok(self.repo.find_by_id(id).await?)
    }

    #[instrument(skip(self), fields(table = "@TABLE@", op = "list"))]
    async fn list(&self, page: Page) -> Result<Paginated<@STRUCT@>, AppError> {
        Ok(self.repo.find_all(page).await?)
    }

    #[instrument(skip(self, _input), fields(table = "@TABLE@", op = "update"))]
    async fn update(&self, id: @PK@, _input: Update@STRUCT@Input) -> Result<Option<@STRUCT@>, AppError> {
        let mut current = match self.repo.find_by_id(id).await? {
            Some(c) => c,
            None => return Ok(None),
        };
@APPLY_UPDATE@        let updated = self.repo.update(id, &current).await?;
        Ok(updated)
    }

    #[instrument(skip(self), fields(table = "@TABLE@", op = "delete"))]
    async fn delete(&self, id: @PK@) -> Result<bool, AppError> {
        Ok(self.repo.delete(id).await?)
    }
@EXTRA_IMPL@}
"#,
        &[
            ("@TABLE@", t.name.clone()),
            ("@MODULE@", t.module_name.clone()),
            ("@STRUCT@", t.struct_name.clone()),
            ("@PK@", pk.kind.rust_type()),
            ("@TYPE_IMPORTS@", imports),
            ("@EXTRA_IMPL@", extra_impl),
            ("@SET_FIELDS@", set_fields_block(t)),
            ("@APPLY_UPDATE@", apply_update_block(t)),
        ],
    )
}

/// Renders the `entity.<field> = input.<field>;` block for create.
fn set_fields_block(t: &Table) -> String {
    let mut out = String::new();
    for c in t.insert_cols() {
        out.push_str(&format!(
            "        entity.{} = _input.{};\n",
            c.field_ident, c.field_ident
        ));
    }
    out
}

/// Renders the `if let Some(v) = …` block for partial update.
fn apply_update_block(t: &Table) -> String {
    let mut out = String::new();
    for c in t.update_cols() {
        if c.nullable {
            out.push_str(&format!(
                "        if let Some(v) = &_input.{f} {{\n            current.{f} = Some(v.clone());\n        }}\n",
                f = c.field_ident
            ));
        } else {
            out.push_str(&format!(
                "        if let Some(v) = &_input.{f} {{\n            current.{f} = v.clone();\n        }}\n",
                f = c.field_ident
            ));
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// repository port (shared, per-table traits)
// ─────────────────────────────────────────────────────────────────────────────

const BUS_RS: &str = r#"//! Event bus port — publishes domain events to in-process subscribers.
//!
//! The trait is dyn-compatible: every method takes concrete (non-generic)
//! arguments so `Arc<dyn EventBus>` works.

use async_trait::async_trait;
use std::sync::Arc;

/// Type-erased domain event — every concrete event box-into this.
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
    /// Stable event name (used for routing and logging).
    fn name(&self) -> &'static str;
}

/// Event handler trait — called when a matching event is published.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handles the event.
    async fn handle(&self, event: &(dyn DomainEvent));
}

/// Event bus port — publish/subscribe for domain events.
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publishes an event to all subscribers.
    async fn publish(&self, event: Box<dyn DomainEvent>);

    /// Subscribes a handler to all events of a given name.
    async fn subscribe(&self, event_name: &'static str, handler: Arc<dyn EventHandler>);
}

/// Type alias for shared bus references.
pub type SharedBus = Arc<dyn EventBus>;
"#;

/// Renders `application/ports/repository.rs` — base `Repository<T, ID>` trait
/// plus per-table trait declarations.
fn repository_port_rs(schema: &Schema) -> String {
    let mut out = String::from(
        "//! Repository ports — outbound contracts owned by the application layer.\n//!\n//! The base `Repository<T, ID>` trait declares the six canonical CRUD\n//! operations; each per-table trait extends it with context-specific\n//! look-ups detected in the schema.\n\nuse async_trait::async_trait;\nuse serde::Serialize;\nuse thiserror::Error;\n\n/// Error taxonomy shared by every repository adapter.\n#[derive(Debug, Error)]\npub enum RepositoryError {\n    /// The requested entity does not exist.\n    #[error(\"entity not found\")]\n    NotFound,\n    /// Input failed application-level validation.\n    #[error(\"validation failed: {0}\")]\n    Validation(String),\n    /// Database constraint was violated (unique / FK / not-null).\n    #[error(\"constraint violation: {0}\")]\n    Constraint(String),\n    /// Anything else raised by the driver.\n    #[error(\"database error: {0}\")]\n    Database(String),\n}\n\n/// Maps a raw [`sqlx::Error`] into the repository taxonomy.\npub fn map_db_error(error: sqlx::Error) -> RepositoryError {\n    use sqlx::Error;\n    match error {\n        Error::RowNotFound => RepositoryError::NotFound,\n        Error::Database(ref db) => match db.code().as_deref() {\n            Some(\"23505\") | Some(\"23503\") | Some(\"23502\") => {\n                RepositoryError::Constraint(db.message().to_string())\n            }\n            _ => RepositoryError::Database(error.to_string()),\n        },\n        other => RepositoryError::Database(other.to_string()),\n    }\n}\n\n/// Pagination request. `page` is 1-based; `per_page` is clamped to 1..=200.\n#[derive(Debug, Clone, Copy)]\npub struct Page {\n    /// 1-based page number.\n    pub page: i64,\n    /// Number of items per page.\n    pub per_page: i64,\n}\n\nimpl Page {\n    /// Builds a clamped page from raw query parameters.\n    #[must_use]\n    pub fn new(page: i64, per_page: i64) -> Self {\n        Self {\n            page: page.max(1),\n            per_page: per_page.clamp(1, 200),\n        }\n    }\n    /// SQL OFFSET for this page.\n    #[must_use]\n    pub fn offset(&self) -> i64 {\n        (self.page - 1) * self.per_page\n    }\n    /// SQL LIMIT for this page.\n    #[must_use]\n    pub fn limit(&self) -> i64 {\n        self.per_page\n    }\n}\n\n/// Pagination envelope returned by [`Repository::find_all`].\n#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]\npub struct Paginated<T> {\n    pub items: Vec<T>,\n    pub total: i64,\n    pub page: i64,\n    pub per_page: i64,\n}\n\nimpl From<RepositoryError> for crate::shared::error::AppError {\n    fn from(e: RepositoryError) -> Self {\n        match e {\n            RepositoryError::NotFound => crate::shared::error::AppError::NotFound(\"entity not found\".into()),\n            RepositoryError::Validation(m) => crate::shared::error::AppError::Validation(m),\n            RepositoryError::Constraint(m) => crate::shared::error::AppError::Conflict(m),\n            RepositoryError::Database(m) => crate::shared::error::AppError::Internal(m),\n        }\n    }\n}\n\n/// The enterprise repository contract.\n#[async_trait]\npub trait Repository<T, ID>: Send + Sync\nwhere\n    T: Send + Sync,\n    ID: Send + Sync,\n{\n    async fn find_by_id(&self, id: ID) -> Result<Option<T>, RepositoryError>;\n    async fn find_all(&self, page: Page) -> Result<Paginated<T>, RepositoryError>;\n    async fn create(&self, entity: &T) -> Result<T, RepositoryError>;\n    async fn update(&self, id: ID, entity: &T) -> Result<Option<T>, RepositoryError>;\n    async fn delete(&self, id: ID) -> Result<bool, RepositoryError>;\n    async fn count(&self) -> Result<i64, RepositoryError>;\n}\n\n",
    );

    // Collect the union of all per-table feature-column types so we can
    // emit a single import block at the top of the file (avoids duplicate
    // `use` statements when multiple tables use the same chrono/uuid types).
    let mut all_scan_cols: Vec<&crate::ir::Column> = Vec::new();
    for t in &schema.tables {
        let pk = t.pk().expect("pk required for codegen");
        all_scan_cols.push(pk);
        all_scan_cols.extend(crate::codegen::domain_helpers::port_feature_cols(t));
    }
    for line in ImportSet::from_cols(all_scan_cols).render() {
        out.push_str(&line);
        out.push('\n');
    }

    // Per-table entity imports.
    for t in &schema.tables {
        out.push_str(&format!(
            "use crate::domain::entities::{}::{};\n",
            t.module_name, t.struct_name
        ));
        for (_, rust_name) in t.used_enums() {
            out.push_str(&format!(
                "use crate::domain::entities::{}::{};\n",
                t.module_name, rust_name
            ));
        }
    }
    out.push('\n');

    for t in &schema.tables {
        out.push_str(&per_table_repository_trait(t));
    }
    out
}

/// Renders one per-table `trait XxxRepository: Repository<Xxx, PK>` block.
///
/// Imports are emitted once by the caller ([`repository_port_rs`]).
fn per_table_repository_trait(t: &Table) -> String {
    let pk = t.pk().expect("pk required for codegen");
    let mut out = String::new();

    out.push_str(&format!(
        "\n/// Repository port for `{}` — extends the enterprise base repository\n/// with the context-specific look-ups detected in the schema.\n#[async_trait]\npub trait {}Repository: Repository<{}, {}> {{\n",
        t.name, t.struct_name, t.struct_name, pk.kind.rust_type()
    ));

    if let Some(status) = status_type(t) {
        out.push_str(&format!(
            "    /// Lists records sharing a status value.\n    async fn find_by_status(&self, status: {status}, page: Page) -> Result<Vec<{}>, RepositoryError>;\n    /// Updates the status column of one record.\n    async fn set_status(&self, id: {}, status: {status}) -> Result<Option<{}>, RepositoryError>;\n",
            t.struct_name,
            pk.kind.rust_type(),
            t.struct_name
        ));
    }
    if t.analysis.has_soft_delete() {
        out.push_str(&format!(
            "    /// Tombstones the record (soft delete) instead of removing it.\n    async fn soft_delete(&self, id: {}) -> Result<bool, RepositoryError>;\n    /// Restores a soft-deleted record and returns it.\n    async fn restore(&self, id: {}) -> Result<Option<{}>, RepositoryError>;\n",
            pk.kind.rust_type(),
            pk.kind.rust_type(),
            t.struct_name
        ));
    }
    if let Some(user) = user_type(t) {
        out.push_str(&format!(
            "    /// Lists records belonging to a user.\n    async fn find_by_user(&self, user_id: {user}, page: Page) -> Result<Vec<{}>, RepositoryError>;\n",
            t.struct_name
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.created_at_col) {
        out.push_str(&format!(
            "    /// Lists records created at or after `ts`.\n    async fn find_created_after(&self, ts: {ts}, page: Page) -> Result<Vec<{}>, RepositoryError>;\n",
            t.struct_name
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.updated_at_col) {
        out.push_str(&format!(
            "    /// Lists records updated at or after `ts`.\n    async fn find_updated_after(&self, ts: {ts}, page: Page) -> Result<Vec<{}>, RepositoryError>;\n",
            t.struct_name
        ));
    }
    out.push_str("}\n\n");

    // Service trait — declared alongside the repository port so the service
    // adapter (in `services/<table>_service.rs`) can `impl XxxServicePort for …`.
    // Named `XxxServicePort` (not `XxxService`) to avoid clashing with the
    // concrete struct `XxxService` defined in `services/<table>_service.rs`.
    out.push_str(&format!(
        "/// Application service contract for `{}`.\n#[async_trait]\npub trait {}ServicePort: Send + Sync {{\n    async fn create(&self, input: crate::application::dto::{}_dto::Create{}Input) -> Result<{}, crate::shared::error::AppError>;\n    async fn get(&self, id: {}) -> Result<Option<{}>, crate::shared::error::AppError>;\n    async fn list(&self, page: Page) -> Result<Paginated<{}>, crate::shared::error::AppError>;\n    async fn update(&self, id: {}, input: crate::application::dto::{}_dto::Update{}Input) -> Result<Option<{}>, crate::shared::error::AppError>;\n    async fn delete(&self, id: {}) -> Result<bool, crate::shared::error::AppError>;\n",
        t.name,
        t.struct_name,
        t.module_name,
        t.struct_name,
        t.struct_name,
        pk.kind.rust_type(),
        t.struct_name,
        t.struct_name,
        pk.kind.rust_type(),
        t.module_name,
        t.struct_name,
        t.struct_name,
        pk.kind.rust_type()
    ));
    if let Some(status) = status_type(t) {
        out.push_str(&format!(
            "    async fn list_by_status(&self, status: {status}, page: Page) -> Result<Vec<{}>, crate::shared::error::AppError>;\n    async fn set_status(&self, id: {}, status: {status}) -> Result<Option<{}>, crate::shared::error::AppError>;\n",
            t.struct_name,
            pk.kind.rust_type(),
            t.struct_name
        ));
    }
    if t.analysis.has_soft_delete() {
        out.push_str(&format!(
            "    async fn soft_delete(&self, id: {}) -> Result<bool, crate::shared::error::AppError>;\n    async fn restore(&self, id: {}) -> Result<Option<{}>, crate::shared::error::AppError>;\n",
            pk.kind.rust_type(),
            pk.kind.rust_type(),
            t.struct_name
        ));
    }
    if let Some(user) = user_type(t) {
        out.push_str(&format!(
            "    async fn list_by_user(&self, user_id: {user}, page: Page) -> Result<Vec<{}>, crate::shared::error::AppError>;\n",
            t.struct_name
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.created_at_col) {
        out.push_str(&format!(
            "    async fn list_created_after(&self, ts: {ts}, page: Page) -> Result<Vec<{}>, crate::shared::error::AppError>;\n",
            t.struct_name
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.updated_at_col) {
        out.push_str(&format!(
            "    async fn list_updated_after(&self, ts: {ts}, page: Page) -> Result<Vec<{}>, crate::shared::error::AppError>;\n",
            t.struct_name
        ));
    }
    out.push_str("}\n\n");

    out
}
