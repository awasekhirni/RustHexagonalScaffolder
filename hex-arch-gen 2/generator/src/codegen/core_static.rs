//! Static core artefacts shared by every generated project.
//!
//! These files do not depend on the introspected schema:
//!   * `enterprise/bus.rs`        — Enterprise Bus (CommandBus / QueryBus)
//!   * `enterprise/repository.rs` — Enterprise Repository (`Repository<T, ID>`)
//!   * `error.rs`                 — HTTP error taxonomy
//!   * `config.rs`                — environment configuration
//!   * `api/state.rs`             — shared application state

use std::path::Path;

use anyhow::Result;

use crate::codegen::write_file;

/// Writes the schema-independent core of the generated project.
pub fn write_all(root: &Path) -> Result<()> {
    let src = root.join("src");

    write_file(&src.join("enterprise").join("mod.rs"), ENTERPRISE_MOD)?;
    write_file(&src.join("enterprise").join("bus.rs"), ENTERPRISE_BUS)?;
    write_file(&src.join("enterprise").join("repository.rs"), ENTERPRISE_REPOSITORY)?;
    write_file(&src.join("error.rs"), ERROR_RS)?;
    write_file(&src.join("config.rs"), CONFIG_RS)?;
    write_file(&src.join("api").join("state.rs"), STATE_RS)?;
    Ok(())
}

const ENTERPRISE_MOD: &str = r#"//! Enterprise integration patterns shared by every bounded context.
//!
//! * [`bus`]        — in-process Command/Query dispatch (Enterprise Bus)
//! * [`repository`] — the generic repository contract (Enterprise Repository)

pub mod bus;
pub mod repository;
"#;

const ENTERPRISE_BUS: &str = r#"//! # Enterprise Bus Pattern
//!
//! Two lightweight in-process buses decouple the HTTP surface from the
//! application layer:
//!
//! * [`CommandBus`] — the **write** side (create / update / delete / …).
//!   Every command names its own output type via the [`Command`] trait.
//! * [`QueryBus`]   — the **read** side (get / list / …), likewise typed
//!   through [`Query`].
//!
//! Handlers are registered per *message type*. Registration wraps the
//! handler in a type-erased adapter keyed by the message's [`TypeId`],
//! so a single handler struct may serve many messages without tripping
//! Rust coherence rules. Dispatch returns a boxed [`Any`] that `send` /
//! `ask` downcasts back to the declared output type.
//!
//! ```text
//! handler ──send(CreateOrdersCommand)──▶ CommandBus ──▶ OrdersHandlers
//!            Result<Orders, BusError>  ◀───────  .handle(command)
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use crate::enterprise::repository::RepositoryError;

/// Boxed send-able future used by the erased handler machinery.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Errors raised while routing or executing a message.
#[derive(Debug, thiserror::Error)]
pub enum BusError {
    /// No handler was registered for the message type.
    #[error("no handler registered for this message type")]
    NoHandler,
    /// The erased payload did not downcast to the expected type.
    #[error("message payload did not match the registered handler")]
    WrongType,
    /// The underlying application/repository layer failed.
    #[error(transparent)]
    Repo(#[from] RepositoryError),
}

/// A write-side message with an associated output type.
pub trait Command: Send + 'static {
    /// Value returned to the caller after the command executes.
    type Output: Send + 'static;
}

/// A read-side message with an associated output type.
pub trait Query: Send + 'static {
    /// Value returned to the caller after the query executes.
    type Output: Send + 'static;
}

/// Contract for handlers of a specific [`Command`].
#[async_trait::async_trait]
pub trait CommandHandler<C: Command>: Send + Sync {
    /// Executes the command.
    async fn handle(&self, command: C) -> Result<C::Output, BusError>;
}

/// Contract for handlers of a specific [`Query`].
#[async_trait::async_trait]
pub trait QueryHandler<Q: Query>: Send + Sync {
    /// Executes the query.
    async fn handle(&self, query: Q) -> Result<Q::Output, BusError>;
}

/// Type-erased dispatch target shared by both buses.
trait ErasedHandler: Send + Sync {
    /// Consumes an erased message and produces an erased output.
    fn invoke<'a>(
        &'a self,
        message: Box<dyn Any + Send>,
    ) -> BoxFuture<'a, Result<Box<dyn Any + Send>, BusError>>;
}

/// Wraps a handler for one specific command type.
struct CommandAdapter<C, H> {
    inner: H,
    _command: PhantomData<fn() -> C>,
}

/// Wraps a handler for one specific query type.
struct QueryAdapter<Q, H> {
    inner: H,
    _query: PhantomData<fn() -> Q>,
}

impl<C, H> ErasedHandler for CommandAdapter<C, H>
where
    C: Command + 'static,
    H: CommandHandler<C> + 'static,
{
    fn invoke<'a>(
        &'a self,
        message: Box<dyn Any + Send>,
    ) -> BoxFuture<'a, Result<Box<dyn Any + Send>, BusError>> {
        Box::pin(async move {
            let command = *message.downcast::<C>().map_err(|_| BusError::WrongType)?;
            let output = self.inner.handle(command).await?;
            Ok(Box::new(output) as Box<dyn Any + Send>)
        })
    }
}

impl<Q, H> ErasedHandler for QueryAdapter<Q, H>
where
    Q: Query + 'static,
    H: QueryHandler<Q> + 'static,
{
    fn invoke<'a>(
        &'a self,
        message: Box<dyn Any + Send>,
    ) -> BoxFuture<'a, Result<Box<dyn Any + Send>, BusError>> {
        Box::pin(async move {
            let query = *message.downcast::<Q>().map_err(|_| BusError::WrongType)?;
            let output = self.inner.handle(query).await?;
            Ok(Box::new(output) as Box<dyn Any + Send>)
        })
    }
}

/// In-process command bus (write side of the enterprise bus).
#[derive(Default)]
pub struct CommandBus {
    handlers: HashMap<TypeId, Arc<dyn ErasedHandler>>,
}

impl CommandBus {
    /// Creates an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler for command type `C`.
    ///
    /// Because registration goes through the type-erased adapter, one
    /// handler struct can be registered for many command types.
    pub fn register<C, H>(&mut self, handler: H)
    where
        C: Command + 'static,
        H: CommandHandler<C> + 'static,
    {
        self.handlers.insert(
            TypeId::of::<C>(),
            Arc::new(CommandAdapter { inner: handler, _command: PhantomData }),
        );
    }

    /// Dispatches a command to its registered handler.
    pub async fn send<C: Command + 'static>(&self, command: C) -> Result<C::Output, BusError> {
        let handler = self
            .handlers
            .get(&TypeId::of::<C>())
            .ok_or(BusError::NoHandler)?;
        let boxed = handler.invoke(Box::new(command)).await?;
        boxed
            .downcast::<C::Output>()
            .map(|b| *b)
            .map_err(|_| BusError::WrongType)
    }
}

/// In-process query bus (read side of the enterprise bus).
#[derive(Default)]
pub struct QueryBus {
    handlers: HashMap<TypeId, Arc<dyn ErasedHandler>>,
}

impl QueryBus {
    /// Creates an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler for query type `Q`.
    pub fn register<Q, H>(&mut self, handler: H)
    where
        Q: Query + 'static,
        H: QueryHandler<Q> + 'static,
    {
        self.handlers.insert(
            TypeId::of::<Q>(),
            Arc::new(QueryAdapter { inner: handler, _query: PhantomData }),
        );
    }

    /// Dispatches a query to its registered handler.
    pub async fn ask<Q: Query + 'static>(&self, query: Q) -> Result<Q::Output, BusError> {
        let handler = self
            .handlers
            .get(&TypeId::of::<Q>())
            .ok_or(BusError::NoHandler)?;
        let boxed = handler.invoke(Box::new(query)).await?;
        boxed
            .downcast::<Q::Output>()
            .map(|b| *b)
            .map_err(|_| BusError::WrongType)
    }
}
"#;

const ENTERPRISE_REPOSITORY: &str = r#"//! # Enterprise Repository Pattern
//!
//! One generic, async, storage-agnostic repository contract shared by all
//! bounded contexts. The domain declares a thin port per aggregate that
//! *supertraits* this contract; infrastructure provides SQLx adapters.
//!
//! ```text
//! domain/orders/repository.rs   trait OrdersRepository: Repository<Orders, Uuid>
//!                                            │ implemented by
//!                                            ▼
//! infrastructure/orders_repository.rs   PostgresOrdersRepository (SQLx)
//! ```

use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;

/// Error taxonomy shared by every repository adapter.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// The requested entity does not exist.
    #[error("entity not found")]
    NotFound,
    /// Input failed application-level validation.
    #[error("validation failed: {0}")]
    Validation(String),
    /// Database constraint was violated (unique / FK / not-null).
    #[error("constraint violation: {0}")]
    Constraint(String),
    /// Anything else raised by the driver.
    #[error("database error: {0}")]
    Database(String),
}

/// Maps a raw [`sqlx::Error`] into the repository taxonomy, translating
/// common PostgreSQL SQLSTATE codes into semantic variants.
pub fn map_db_error(error: sqlx::Error) -> RepositoryError {
    use sqlx::Error;
    match error {
        Error::RowNotFound => RepositoryError::NotFound,
        Error::Database(ref db) => match db.code().as_deref() {
            Some("23505") | Some("23503") | Some("23502") => {
                RepositoryError::Constraint(db.message().to_string())
            }
            _ => RepositoryError::Database(error.to_string()),
        },
        other => RepositoryError::Database(other.to_string()),
    }
}

/// Pagination request. `page` is 1-based; `per_page` is clamped to 1..=200.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    /// 1-based page number.
    pub page: i64,
    /// Number of items per page.
    pub per_page: i64,
}

impl Page {
    /// Builds a clamped page from raw query parameters.
    pub fn new(page: i64, per_page: i64) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, 200),
        }
    }

    /// SQL OFFSET for this page.
    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.per_page
    }

    /// SQL LIMIT for this page.
    pub fn limit(&self) -> i64 {
        self.per_page
    }
}

/// Pagination envelope returned by [`Repository::find_all`].
#[derive(Debug, Clone, Serialize)]
pub struct Paginated<T> {
    /// Items of the requested page.
    pub items: Vec<T>,
    /// Total number of rows matching the query.
    pub total: i64,
    /// Current page number.
    pub page: i64,
    /// Requested page size.
    pub per_page: i64,
}

/// The enterprise repository contract.
///
/// `T` is the aggregate/entity type and `ID` its primary-key type
/// (`Uuid`, `i32`, …). Every generated repository port extends this
/// trait, so adapters always provide the six canonical operations.
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{
    /// Loads one entity by primary key.
    async fn find_by_id(&self, id: ID) -> Result<Option<T>, RepositoryError>;
    /// Loads a page of entities together with the total row count.
    async fn find_all(&self, page: Page) -> Result<Paginated<T>, RepositoryError>;
    /// Persists a new entity and returns it (including DB-generated cols).
    async fn create(&self, entity: &T) -> Result<T, RepositoryError>;
    /// Updates an entity; returns `None` when the ID does not exist.
    async fn update(&self, id: ID, entity: &T) -> Result<Option<T>, RepositoryError>;
    /// Hard-deletes an entity; returns whether a row was affected.
    async fn delete(&self, id: ID) -> Result<bool, RepositoryError>;
    /// Total number of rows.
    async fn count(&self) -> Result<i64, RepositoryError>;
}
"#;

const ERROR_RS: &str = r#"//! HTTP error taxonomy — translates domain and bus errors onto status
//! codes with a stable JSON body: `{"error": "…"}`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::enterprise::bus::BusError;
use crate::enterprise::repository::RepositoryError;

/// Errors surfaced by the generated HTTP handlers.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// 404 — the resource does not exist.
    #[error("resource not found")]
    NotFound,
    /// 400 — the request payload or parameter is invalid.
    #[error("{0}")]
    BadRequest(String),
    /// 409 — unique/FK constraint violation.
    #[error("{0}")]
    Conflict(String),
    /// 500 — unexpected internal failure.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<RepositoryError> for ApiError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::NotFound => ApiError::NotFound,
            RepositoryError::Validation(message) => ApiError::BadRequest(message),
            RepositoryError::Constraint(message) => ApiError::Conflict(message),
            RepositoryError::Database(message) => ApiError::Internal(message),
        }
    }
}

impl From<BusError> for ApiError {
    fn from(error: BusError) -> Self {
        match error {
            BusError::Repo(inner) => inner.into(),
            other => ApiError::Internal(other.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}
"#;

const CONFIG_RS: &str = r#"//! Runtime configuration resolved from the environment (12-factor style).
//!
//! `.env` is loaded by `dotenvy` at boot; every value can be overridden
//! by real environment variables (container-friendly).

use anyhow::Context;

/// Application settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// PostgreSQL connection string.
    pub database_url: String,
    /// Socket the HTTP server binds to, e.g. `0.0.0.0:8080`.
    pub bind_address: String,
    /// Maximum PostgreSQL pool connections.
    pub db_max_connections: u32,
}

impl Config {
    /// Loads configuration, failing fast when `DATABASE_URL` is missing.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set (see .env.example)")?,
            bind_address: std::env::var("BIND_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            db_max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        })
    }
}
"#;

const STATE_RS: &str = r#"//! Shared application state handed to every Axum handler.
//!
//! Handlers never touch repositories or services directly — they dispatch
//! through these two buses, which is what keeps the hexagon upright.

use std::sync::Arc;

use crate::enterprise::bus::{CommandBus, QueryBus};

/// Dependencies shared across the whole HTTP surface.
#[derive(Clone)]
pub struct AppState {
    /// Write-side dispatch (commands).
    pub cmd: Arc<CommandBus>,
    /// Read-side dispatch (queries).
    pub qry: Arc<QueryBus>,
}
"#;
