//! Application layer codegen — use-case adapters that implement the
//! inbound service port, plus the enterprise-bus messaging surface.
//!
//! Templates use `@TOKEN@` substitution (rather than `format!`) so the
//! generated braces stay literal and arg-order bugs cannot happen.

use std::path::Path;

use anyhow::Result;

use crate::codegen::domain::{port_feature_cols, status_type, ts_type, user_type};
use crate::codegen::{write_file, ImportSet};
use crate::ir::{Schema, Table};

/// Writes `src/application/<table>_{{mapper,service,messaging}}.rs`.
pub fn write_table(_schema: &Schema, root: &Path, t: &Table) -> Result<()> {
    let dir = root.join("src").join("application");
    write_file(&dir.join(format!("{}_mapper.rs", t.module_name)), &mapper_rs(t))?;
    write_file(&dir.join(format!("{}_service.rs", t.module_name)), &service_rs(t))?;
    write_file(
        &dir.join(format!("{}_messaging.rs", t.module_name)),
        &messaging_rs(t),
    )?;
    Ok(())
}

/// Writes `src/application/mod.rs`.
pub fn write_mod(schema: &Schema, root: &Path) -> Result<()> {
    let mut body = String::from(
        "//! Application layer — use-case adapters driving the domain through ports.\n//!\n//! Services depend only on port traits; the enterprise-bus handlers below\n//! delegate to them, and HTTP handlers dispatch through the buses.\n\n",
    );
    for t in &schema.tables {
        body.push_str(&format!("pub mod {}_mapper;\n", t.module_name));
        body.push_str(&format!("pub mod {}_messaging;\n", t.module_name));
        body.push_str(&format!("pub mod {}_service;\n", t.module_name));
    }
    write_file(&root.join("src").join("application").join("mod.rs"), &body)
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
// <table>_mapper.rs
// ─────────────────────────────────────────────────────────────────────────────

fn mapper_rs(t: &Table) -> String {
    let mut to_entity_fields = String::new();
    for c in t.insert_cols() {
        to_entity_fields.push_str(&format!(
            "            {}: input.{},\n",
            c.field_ident, c.field_ident
        ));
    }

    let mut apply_update = String::new();
    for c in t.update_cols() {
        if c.nullable {
            apply_update.push_str(&format!(
                "        if let Some(v) = &input.{field} {{\n            current.{field} = Some(v.clone());\n        }}\n",
                field = c.field_ident
            ));
        } else {
            apply_update.push_str(&format!(
                "        if let Some(v) = &input.{field} {{\n            current.{field} = v.clone();\n        }}\n",
                field = c.field_ident
            ));
        }
    }

    sub(
        r#"//! Mapper for `@TABLE@` — translates input models into domain entities.
//!
//! Columns the database generates are left at their default here; the
//! repository INSERT simply omits them and `RETURNING *` hands the real
//! values back.

use crate::domain::@MODULE@::entity::@STRUCT@;
use crate::domain::@MODULE@::input::{Create@STRUCT@Input, Update@STRUCT@Input};

/// Input ↔ entity translation for `@TABLE@`.
pub struct @STRUCT@Mapper;

impl @STRUCT@Mapper {
    /// Builds a fresh entity from a validated create payload.
    pub fn to_entity(input: Create@STRUCT@Input) -> @STRUCT@ {
        @STRUCT@ {
@TO_ENTITY_FIELDS@            ..Default::default()
        }
    }

    /// Applies a partial update onto the loaded entity — only fields
    /// present in the payload are overwritten.
    pub fn apply_update(mut current: @STRUCT@, input: &Update@STRUCT@Input) -> @STRUCT@ {
@APPLY_UPDATE@        current
    }
}
"#,
        &[
            ("@TABLE@", t.name.clone()),
            ("@MODULE@", t.module_name.clone()),
            ("@STRUCT@", t.struct_name.clone()),
            ("@TO_ENTITY_FIELDS@", to_entity_fields),
            ("@APPLY_UPDATE@", apply_update),
        ],
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// <table>_service.rs
// ─────────────────────────────────────────────────────────────────────────────

fn service_rs(t: &Table) -> String {
    let pk = t.pk().expect("pk required for codegen");

    let mut extra_impl = String::new();
    if let Some(status) = status_type(t) {
        extra_impl.push_str(&sub(
            r#"
    async fn list_by_status(&self, status: @STATUS@, page: Page) -> Result<Vec<@STRUCT@>, RepositoryError> {
        self.repo.find_by_status(status, page).await
    }

    async fn set_status(&self, id: @PK@, status: @STATUS@) -> Result<Option<@STRUCT@>, RepositoryError> {
        self.repo.set_status(id, status).await
    }
"#,
            &[
                ("@STATUS@", status),
                ("@STRUCT@", t.struct_name.clone()),
                ("@PK@", pk.kind.rust_type()),
            ],
        ));
    }
    if t.analysis.soft_flag_col.is_some() || t.analysis.deleted_at_col.is_some() {
        extra_impl.push_str(&sub(
            r#"
    async fn soft_delete(&self, id: @PK@) -> Result<bool, RepositoryError> {
        self.repo.soft_delete(id).await
    }

    async fn restore(&self, id: @PK@) -> Result<Option<@STRUCT@>, RepositoryError> {
        self.repo.restore(id).await
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
    async fn list_by_user(&self, user_id: @USER@, page: Page) -> Result<Vec<@STRUCT@>, RepositoryError> {
        self.repo.find_by_user(user_id, page).await
    }
"#,
            &[
                ("@USER@", user),
                ("@STRUCT@", t.struct_name.clone()),
            ],
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.created_at_col) {
        extra_impl.push_str(&sub(
            r#"
    async fn list_created_after(&self, ts: @TS@, page: Page) -> Result<Vec<@STRUCT@>, RepositoryError> {
        self.repo.find_created_after(ts, page).await
    }
"#,
            &[
                ("@TS@", ts),
                ("@STRUCT@", t.struct_name.clone()),
            ],
        ));
    }
    if let Some(ts) = ts_type(t, &t.analysis.updated_at_col) {
        extra_impl.push_str(&sub(
            r#"
    async fn list_updated_after(&self, ts: @TS@, page: Page) -> Result<Vec<@STRUCT@>, RepositoryError> {
        self.repo.find_updated_after(ts, page).await
    }
"#,
            &[
                ("@TS@", ts),
                ("@STRUCT@", t.struct_name.clone()),
            ],
        ));
    }

    let mut imports = String::new();
    let mut scan_cols: Vec<&crate::ir::Column> = vec![pk];
    scan_cols.extend(port_feature_cols(t));
    for line in ImportSet::from_cols(scan_cols).render() {
        imports.push_str(&line);
        imports.push('\n');
    }
    for (_, rust_name) in t.used_enums() {
        imports.push_str(&format!(
            "use crate::domain::{}::enums::{};\n",
            t.module_name, rust_name
        ));
    }

    sub(
        r#"//! Application service adapter for `@TABLE@` — the concrete inbound adapter
//! implementing the [@STRUCT@Service] port on top of the repository port.
//!
//! The service is where business rules live: payload validation, invariant
//! checks, and orchestration of the repository port. Swap-in adapters for
//! other stores only need to implement the repository port.

use std::sync::Arc;

use async_trait::async_trait;

@TYPE_IMPORTS@use crate::application::@MODULE@_mapper::@STRUCT@Mapper;
use crate::domain::@MODULE@::entity::@STRUCT@;
use crate::domain::@MODULE@::input::{Create@STRUCT@Input, Update@STRUCT@Input};
use crate::domain::@MODULE@::repository::@STRUCT@Repository;
use crate::domain::@MODULE@::service::@STRUCT@Service;
use crate::enterprise::repository::{Page, Paginated, RepositoryError};

/// Concrete service for `@TABLE@`; injected into the bus handlers by `main`.
pub struct @STRUCT@ServiceImpl {
    repo: Arc<dyn @STRUCT@Repository>,
}

impl @STRUCT@ServiceImpl {
    /// Wires the service to its repository port.
    pub fn new(repo: Arc<dyn @STRUCT@Repository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl @STRUCT@Service for @STRUCT@ServiceImpl {
    async fn get(&self, id: @PK@) -> Result<Option<@STRUCT@>, RepositoryError> {
        self.repo.find_by_id(id).await
    }

    async fn list(&self, page: Page) -> Result<Paginated<@STRUCT@>, RepositoryError> {
        self.repo.find_all(page).await
    }

    async fn create(&self, input: Create@STRUCT@Input) -> Result<@STRUCT@, RepositoryError> {
        input.validate().map_err(RepositoryError::Validation)?;
        let entity = @STRUCT@Mapper::to_entity(input);
        entity.validate().map_err(RepositoryError::Validation)?;
        self.repo.create(&entity).await
    }

    async fn update(&self, id: @PK@, input: Update@STRUCT@Input) -> Result<Option<@STRUCT@>, RepositoryError> {
        let current = match self.repo.find_by_id(id).await? {
            Some(current) => current,
            None => return Ok(None),
        };
        let merged = @STRUCT@Mapper::apply_update(current, &input);
        merged.validate().map_err(RepositoryError::Validation)?;
        self.repo.update(id, &merged).await
    }

    async fn delete(&self, id: @PK@) -> Result<bool, RepositoryError> {
        self.repo.delete(id).await
    }
@EXTRA_IMPL@}
"#,
        &[
            ("@TABLE@", t.name.clone()),
            ("@MODULE@", t.module_name.clone()),
            ("@STRUCT@", t.struct_name.clone()),
            ("@STRUCT@Service", format!("{}Service", t.struct_name)),
            ("@STRUCT@Repository", format!("{}Repository", t.struct_name)),
            ("@STRUCT@Mapper", format!("{}Mapper", t.struct_name)),
            ("@PK@", pk.kind.rust_type()),
            ("@TYPE_IMPORTS@", imports),
            ("@EXTRA_IMPL@", extra_impl),
        ],
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// <table>_messaging.rs
// ─────────────────────────────────────────────────────────────────────────────

fn messaging_rs(t: &Table) -> String {
    let pk = t.pk().expect("pk required for codegen");
    let struct_name = &t.struct_name;
    let pk_ty = pk.kind.rust_type();

    // ── command/query blocks ────────────────────────────────────────────
    let mut blocks = String::new();

    blocks.push_str(&sub(
        r#"/// Persists a new `@TABLE@` record.
pub struct Create@STRUCT@Command {
    /// Validated payload from the HTTP layer.
    pub input: Create@STRUCT@Input,
}

impl Command for Create@STRUCT@Command {
    type Output = @STRUCT@;
}

#[async_trait]
impl CommandHandler<Create@STRUCT@Command> for @STRUCT@Handlers {
    async fn handle(&self, command: Create@STRUCT@Command) -> Result<@STRUCT@, BusError> {
        Ok(self.service.create(command.input).await?)
    }
}

/// Applies a partial update to an existing `@TABLE@` record.
pub struct Update@STRUCT@Command {
    /// Primary key of the record.
    pub id: @PK@,
    /// Fields to overwrite (absent fields keep their value).
    pub input: Update@STRUCT@Input,
}

impl Command for Update@STRUCT@Command {
    type Output = Option<@STRUCT@>;
}

#[async_trait]
impl CommandHandler<Update@STRUCT@Command> for @STRUCT@Handlers {
    async fn handle(&self, command: Update@STRUCT@Command) -> Result<Option<@STRUCT@>, BusError> {
        Ok(self.service.update(command.id, command.input).await?)
    }
}

/// Removes a `@TABLE@` record (hard delete).
pub struct Delete@STRUCT@Command {
    /// Primary key of the record.
    pub id: @PK@,
}

impl Command for Delete@STRUCT@Command {
    type Output = bool;
}

#[async_trait]
impl CommandHandler<Delete@STRUCT@Command> for @STRUCT@Handlers {
    async fn handle(&self, command: Delete@STRUCT@Command) -> Result<bool, BusError> {
        Ok(self.service.delete(command.id).await?)
    }
}

"#,
        &[
            ("@TABLE@", t.name.clone()),
            ("@STRUCT@", struct_name.clone()),
            ("@PK@", pk_ty.clone()),
        ],
    ));

    if let Some(status) = status_type(t) {
        blocks.push_str(&sub(
            r#"/// Changes the status of a `@TABLE@` record.
pub struct Set@STRUCT@StatusCommand {
    /// Primary key of the record.
    pub id: @PK@,
    /// New status value.
    pub status: @STATUS@,
}

impl Command for Set@STRUCT@StatusCommand {
    type Output = Option<@STRUCT@>;
}

#[async_trait]
impl CommandHandler<Set@STRUCT@StatusCommand> for @STRUCT@Handlers {
    async fn handle(&self, command: Set@STRUCT@StatusCommand) -> Result<Option<@STRUCT@>, BusError> {
        Ok(self.service.set_status(command.id, command.status).await?)
    }
}

"#,
            &[
                ("@TABLE@", t.name.clone()),
                ("@STRUCT@", struct_name.clone()),
                ("@PK@", pk_ty.clone()),
                ("@STATUS@", status),
            ],
        ));
    }

    if t.analysis.soft_flag_col.is_some() || t.analysis.deleted_at_col.is_some() {
        blocks.push_str(&sub(
            r#"/// Soft-deletes (tombstones) a `@TABLE@` record.
pub struct SoftDelete@STRUCT@Command {
    /// Primary key of the record.
    pub id: @PK@,
}

impl Command for SoftDelete@STRUCT@Command {
    type Output = bool;
}

#[async_trait]
impl CommandHandler<SoftDelete@STRUCT@Command> for @STRUCT@Handlers {
    async fn handle(&self, command: SoftDelete@STRUCT@Command) -> Result<bool, BusError> {
        Ok(self.service.soft_delete(command.id).await?)
    }
}

/// Restores a soft-deleted `@TABLE@` record.
pub struct Restore@STRUCT@Command {
    /// Primary key of the record.
    pub id: @PK@,
}

impl Command for Restore@STRUCT@Command {
    type Output = Option<@STRUCT@>;
}

#[async_trait]
impl CommandHandler<Restore@STRUCT@Command> for @STRUCT@Handlers {
    async fn handle(&self, command: Restore@STRUCT@Command) -> Result<Option<@STRUCT@>, BusError> {
        Ok(self.service.restore(command.id).await?)
    }
}

"#,
            &[
                ("@TABLE@", t.name.clone()),
                ("@STRUCT@", struct_name.clone()),
                ("@PK@", pk_ty.clone()),
            ],
        ));
    }

    // ── queries ─────────────────────────────────────────────────────────
    blocks.push_str(&sub(
        r#"/// Fetches one `@TABLE@` record by primary key.
pub struct Get@STRUCT@Query {
    /// Primary key.
    pub id: @PK@,
}

impl Query for Get@STRUCT@Query {
    type Output = Option<@STRUCT@>;
}

#[async_trait]
impl QueryHandler<Get@STRUCT@Query> for @STRUCT@Handlers {
    async fn handle(&self, query: Get@STRUCT@Query) -> Result<Option<@STRUCT@>, BusError> {
        Ok(self.service.get(query.id).await?)
    }
}

/// Lists `@TABLE@` records page by page.
pub struct List@STRUCT@Query {
    /// Pagination request.
    pub page: Page,
}

impl Query for List@STRUCT@Query {
    type Output = Paginated<@STRUCT@>;
}

#[async_trait]
impl QueryHandler<List@STRUCT@Query> for @STRUCT@Handlers {
    async fn handle(&self, query: List@STRUCT@Query) -> Result<Paginated<@STRUCT@>, BusError> {
        Ok(self.service.list(query.page).await?)
    }
}

"#,
        &[
            ("@TABLE@", t.name.clone()),
            ("@STRUCT@", struct_name.clone()),
            ("@PK@", pk_ty.clone()),
        ],
    ));

    if let Some(status) = status_type(t) {
        blocks.push_str(&sub(
            r#"/// Lists `@TABLE@` records sharing a status value.
pub struct List@STRUCT@ByStatusQuery {
    /// Status to filter on.
    pub status: @STATUS@,
    /// Pagination request.
    pub page: Page,
}

impl Query for List@STRUCT@ByStatusQuery {
    type Output = Vec<@STRUCT@>;
}

#[async_trait]
impl QueryHandler<List@STRUCT@ByStatusQuery> for @STRUCT@Handlers {
    async fn handle(&self, query: List@STRUCT@ByStatusQuery) -> Result<Vec<@STRUCT@>, BusError> {
        Ok(self.service.list_by_status(query.status, query.page).await?)
    }
}

"#,
            &[
                ("@TABLE@", t.name.clone()),
                ("@STRUCT@", struct_name.clone()),
                ("@STATUS@", status),
            ],
        ));
    }

    if let Some(user) = user_type(t) {
        blocks.push_str(&sub(
            r#"/// Lists `@TABLE@` records belonging to a user.
pub struct List@STRUCT@ByUserQuery {
    /// Owning user's key.
    pub user_id: @USER@,
    /// Pagination request.
    pub page: Page,
}

impl Query for List@STRUCT@ByUserQuery {
    type Output = Vec<@STRUCT@>;
}

#[async_trait]
impl QueryHandler<List@STRUCT@ByUserQuery> for @STRUCT@Handlers {
    async fn handle(&self, query: List@STRUCT@ByUserQuery) -> Result<Vec<@STRUCT@>, BusError> {
        Ok(self.service.list_by_user(query.user_id, query.page).await?)
    }
}

"#,
            &[
                ("@TABLE@", t.name.clone()),
                ("@STRUCT@", struct_name.clone()),
                ("@USER@", user),
            ],
        ));
    }

    if let Some(ts) = ts_type(t, &t.analysis.created_at_col) {
        blocks.push_str(&sub(
            r#"/// Lists `@TABLE@` records created at or after a timestamp.
pub struct List@STRUCT@CreatedAfterQuery {
    /// Inclusive lower bound.
    pub ts: @TS@,
    /// Pagination request.
    pub page: Page,
}

impl Query for List@STRUCT@CreatedAfterQuery {
    type Output = Vec<@STRUCT@>;
}

#[async_trait]
impl QueryHandler<List@STRUCT@CreatedAfterQuery> for @STRUCT@Handlers {
    async fn handle(&self, query: List@STRUCT@CreatedAfterQuery) -> Result<Vec<@STRUCT@>, BusError> {
        Ok(self.service.list_created_after(query.ts, query.page).await?)
    }
}

"#,
            &[
                ("@TABLE@", t.name.clone()),
                ("@STRUCT@", struct_name.clone()),
                ("@TS@", ts),
            ],
        ));
    }

    if let Some(ts) = ts_type(t, &t.analysis.updated_at_col) {
        blocks.push_str(&sub(
            r#"/// Lists `@TABLE@` records updated at or after a timestamp.
pub struct List@STRUCT@UpdatedAfterQuery {
    /// Inclusive lower bound.
    pub ts: @TS@,
    /// Pagination request.
    pub page: Page,
}

impl Query for List@STRUCT@UpdatedAfterQuery {
    type Output = Vec<@STRUCT@>;
}

#[async_trait]
impl QueryHandler<List@STRUCT@UpdatedAfterQuery> for @STRUCT@Handlers {
    async fn handle(&self, query: List@STRUCT@UpdatedAfterQuery) -> Result<Vec<@STRUCT@>, BusError> {
        Ok(self.service.list_updated_after(query.ts, query.page).await?)
    }
}

"#,
            &[
                ("@TABLE@", t.name.clone()),
                ("@STRUCT@", struct_name.clone()),
                ("@TS@", ts),
            ],
        ));
    }

    // ── header + handler bundle ─────────────────────────────────────────
    let mut imports = String::new();
    // The PK type appears throughout command/query payloads.
    let mut cols: Vec<&crate::ir::Column> = vec![pk];
    if t.analysis.created_at_col.is_some() || t.analysis.updated_at_col.is_some() {
        for which in [&t.analysis.created_at_col, &t.analysis.updated_at_col] {
            if let Some(c) = t.columns.iter().find(|c| Some(&c.name) == which.as_ref()) {
                cols.push(c);
            }
        }
    }
    for line in ImportSet::from_cols(cols).render() {
        imports.push_str(&line);
        imports.push('\n');
    }
    if status_type(t).is_some() {
        for (_, rust_name) in t.used_enums() {
            imports.push_str(&format!(
                "use crate::domain::{}::enums::{};\n",
                t.module_name, rust_name
            ));
        }
    }

    sub(
        r#"//! Enterprise-bus wiring for `@TABLE@` — commands (write side), queries
//! (read side) and the handler bundle that executes them.
//!
//! One handler struct implements [`CommandHandler`]/[`QueryHandler`] for
//! many message types; the bus erases each registration behind a
//! `TypeId`-keyed adapter, so HTTP handlers never see services directly.

use std::sync::Arc;

use async_trait::async_trait;

@TYPE_IMPORTS@use crate::domain::@MODULE@::entity::@STRUCT@;
use crate::domain::@MODULE@::input::{Create@STRUCT@Input, Update@STRUCT@Input};
use crate::domain::@MODULE@::service::@STRUCT@Service;
use crate::enterprise::bus::{BusError, Command, CommandHandler, Query, QueryHandler};
use crate::enterprise::repository::{Page, Paginated};

/// Handler bundle for the `@TABLE@` context.
///
/// The same instance can be registered for every command and query of
/// this context because the bus wraps each registration individually.
#[derive(Clone)]
pub struct @STRUCT@Handlers {
    service: Arc<dyn @STRUCT@Service>,
}

impl @STRUCT@Handlers {
    /// Creates the bundle; clone it per registration.
    pub fn new(service: Arc<dyn @STRUCT@Service>) -> Self {
        Self { service }
    }
}

// ─── Commands (write side) ─────────────────────────────────────────────────

@BLOCKS@
"#,
        &[
            ("@TABLE@", t.name.clone()),
            ("@MODULE@", t.module_name.clone()),
            ("@STRUCT@", struct_name.clone()),
            ("@STRUCT@Service", format!("{}Service", struct_name)),
            ("@TYPE_IMPORTS@", imports),
            ("@BLOCKS@", blocks),
        ],
    )
}
