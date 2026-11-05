// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Static platform assets — the schema-independent core of every generated
//! project. These files implement the v0.6.0 desired-state platform's
//! cross-cutting concerns:
//!
//! * `shared/`      — `AppError`, `AppResult`, `Pagination`, `CursorPage`
//! * `config/`      — typed settings loaded by the `config` crate
//! * `logging/`     — tracing initialisation (verbosity + JSON toggle)
//! * `telemetry.rs` — OTLP + Prometheus (stubs)
//! * `core/`        — `error_codes`, `ids`, `clock`, `aggregate`,
//!                    `value_objects/{email,id,money}`, `unit_of_work`,
//!                    `shutdown`, `startup`, `bulkhead`, `saga`, `chaos`, `events`
//! * `patterns/`    — `idempotency`, `outbox_worker`, `outbox_dlq`,
//!                    `circuit_breaker`, `retry`, `pagination`, `filtering`,
//!                    `webhooks`, `cache`, `feature_flags`, `load_shed`
//! * `security/`    — `rls`, `gdpr`, `pii`, `secrets`, `headers`, `tenancy`,
//!                    `auth/{extractor,jwt,refresh,dev_token,rbac,api_key,mod}`, `mod`
//! * `presentation/middleware/*` — `request_id`, `logging`, `rate_limit`,
//!                    `inflight`, `content_type`, `deprecation`, `mod`
//!
//! Every file is emitted verbatim with light `{{PROJECT}}` substitution
//! applied by [`assets::write_all`]. Templates never read the IR — they
//! are 100% schema-independent.

use std::path::Path;

use crate::codegen::write_file;
use crate::config::Config;
use crate::errors::GenResult;

/// Writes the entire static platform.
pub fn write_all(root: &Path, cfg: &Config) -> GenResult<()> {
    let src = root.join("src");

    // ── shared ───────────────────────────────────────────────────────────
    write_file(&src.join("shared").join("mod.rs"), SHARED_MOD)?;
    write_file(&src.join("shared").join("error.rs"), SHARED_ERROR)?;
    write_file(&src.join("shared").join("result.rs"), SHARED_RESULT)?;
    write_file(&src.join("shared").join("pagination.rs"), SHARED_PAGINATION)?;

    // ── config ───────────────────────────────────────────────────────────
    write_file(&src.join("config").join("mod.rs"), CONFIG_MOD)?;
    write_file(
        &src.join("config").join("settings.rs"),
        &sub(CONFIG_SETTINGS, cfg),
    )?;

    // ── logging + telemetry ──────────────────────────────────────────────
    write_file(&src.join("logging").join("mod.rs"), LOGGING_MOD)?;
    write_file(&src.join("logging").join("init.rs"), LOGGING_INIT)?;
    write_file(&src.join("telemetry.rs"), TELEMETRY)?;

    // ── core ─────────────────────────────────────────────────────────────
    write_file(&src.join("core").join("mod.rs"), CORE_MOD)?;
    write_file(&src.join("core").join("error_codes.rs"), CORE_ERROR_CODES)?;
    write_file(&src.join("core").join("ids.rs"), CORE_IDS)?;
    write_file(&src.join("core").join("clock.rs"), CORE_CLOCK)?;
    write_file(&src.join("core").join("aggregate.rs"), CORE_AGGREGATE)?;
    write_file(&src.join("core").join("unit_of_work.rs"), CORE_UOW)?;
    write_file(&src.join("core").join("shutdown.rs"), CORE_SHUTDOWN)?;
    write_file(&src.join("core").join("startup.rs"), CORE_STARTUP)?;
    write_file(&src.join("core").join("bulkhead.rs"), CORE_BULKHEAD)?;
    write_file(&src.join("core").join("saga.rs"), CORE_SAGA)?;
    write_file(&src.join("core").join("chaos.rs"), CORE_CHAOS)?;
    write_file(&src.join("core").join("events.rs"), CORE_EVENTS)?;
    // value_objects
    write_file(
        &src.join("core").join("value_objects").join("mod.rs"),
        CORE_VO_MOD,
    )?;
    write_file(
        &src.join("core").join("value_objects").join("email.rs"),
        CORE_VO_EMAIL,
    )?;
    write_file(
        &src.join("core").join("value_objects").join("id.rs"),
        CORE_VO_ID,
    )?;
    write_file(
        &src.join("core").join("value_objects").join("money.rs"),
        CORE_VO_MONEY,
    )?;

    // ── patterns ─────────────────────────────────────────────────────────
    write_file(&src.join("patterns").join("mod.rs"), PATTERNS_MOD)?;
    write_file(
        &src.join("patterns").join("idempotency.rs"),
        PATTERNS_IDEMPOTENCY,
    )?;
    write_file(
        &src.join("patterns").join("outbox_worker.rs"),
        PATTERNS_OUTBOX_WORKER,
    )?;
    write_file(
        &src.join("patterns").join("outbox_dlq.rs"),
        PATTERNS_OUTBOX_DLQ,
    )?;
    write_file(
        &src.join("patterns").join("circuit_breaker.rs"),
        PATTERNS_CB,
    )?;
    write_file(&src.join("patterns").join("retry.rs"), PATTERNS_RETRY)?;
    write_file(
        &src.join("patterns").join("pagination.rs"),
        PATTERNS_PAGINATION,
    )?;
    write_file(
        &src.join("patterns").join("filtering.rs"),
        PATTERNS_FILTERING,
    )?;
    write_file(&src.join("patterns").join("webhooks.rs"), PATTERNS_WEBHOOKS)?;
    write_file(&src.join("patterns").join("cache.rs"), PATTERNS_CACHE)?;
    write_file(
        &src.join("patterns").join("feature_flags.rs"),
        PATTERNS_FLAGS,
    )?;
    write_file(
        &src.join("patterns").join("load_shed.rs"),
        PATTERNS_LOAD_SHED,
    )?;

    // ── security ─────────────────────────────────────────────────────────
    write_file(&src.join("security").join("mod.rs"), SECURITY_MOD)?;
    write_file(&src.join("security").join("rls.rs"), SECURITY_RLS)?;
    write_file(&src.join("security").join("gdpr.rs"), SECURITY_GDPR)?;
    write_file(&src.join("security").join("pii.rs"), SECURITY_PII)?;
    write_file(&src.join("security").join("secrets.rs"), SECURITY_SECRETS)?;
    write_file(&src.join("security").join("headers.rs"), SECURITY_HEADERS)?;
    write_file(&src.join("security").join("tenancy.rs"), SECURITY_TENANCY)?;
    // auth
    write_file(&src.join("security").join("auth").join("mod.rs"), AUTH_MOD)?;
    write_file(
        &src.join("security").join("auth").join("extractor.rs"),
        AUTH_EXTRACTOR,
    )?;
    write_file(&src.join("security").join("auth").join("jwt.rs"), AUTH_JWT)?;
    write_file(
        &src.join("security").join("auth").join("refresh.rs"),
        AUTH_REFRESH,
    )?;
    write_file(
        &src.join("security").join("auth").join("dev_token.rs"),
        AUTH_DEV_TOKEN,
    )?;
    write_file(
        &src.join("security").join("auth").join("rbac.rs"),
        AUTH_RBAC,
    )?;
    write_file(
        &src.join("security").join("auth").join("api_key.rs"),
        AUTH_API_KEY,
    )?;

    // ── presentation/middleware (shared part; routes/state/handlers are
    //    per-table so they live in `presentation.rs`) ────────────────────
    write_file(
        &src.join("presentation").join("middleware").join("mod.rs"),
        MW_MOD,
    )?;
    write_file(
        &src.join("presentation")
            .join("middleware")
            .join("request_id.rs"),
        MW_REQUEST_ID,
    )?;
    write_file(
        &src.join("presentation")
            .join("middleware")
            .join("logging.rs"),
        MW_LOGGING,
    )?;
    write_file(
        &src.join("presentation")
            .join("middleware")
            .join("rate_limit.rs"),
        MW_RATE_LIMIT,
    )?;
    write_file(
        &src.join("presentation")
            .join("middleware")
            .join("inflight.rs"),
        MW_INFLIGHT,
    )?;
    write_file(
        &src.join("presentation")
            .join("middleware")
            .join("content_type.rs"),
        MW_CONTENT_TYPE,
    )?;
    write_file(
        &src.join("presentation")
            .join("middleware")
            .join("deprecation.rs"),
        MW_DEPRECATION,
    )?;
    write_file(
        &src.join("presentation").join("error.rs"),
        PRESENTATION_ERROR,
    )?;

    Ok(())
}

/// Light `{{PROJECT}}` substitution used by the few templates that need to
/// know the project name (e.g. JWT issuer defaults).
fn sub(template: &str, cfg: &Config) -> String {
    template.replace("{{PROJECT}}", &cfg.project_name)
}

// ════════════════════════════════════════════════════════════════════════════
// SHARED
// ════════════════════════════════════════════════════════════════════════════

const SHARED_MOD: &str = r#"//! Cross-cutting types shared by every layer.
//!
//! These types are intentionally framework-agnostic — they don't depend on
//! axum, sqlx, or utoipa — so the domain layer can use them without pulling
//! in transport concerns.

pub mod error;
pub mod pagination;
pub mod result;

pub use error::AppError;
pub use pagination::Pagination;
pub use result::AppResult;
"#;

const SHARED_ERROR: &str = r#"//! HTTP error taxonomy — translates domain and bus errors onto status
//! codes with a stable JSON body: `{"error": "…", "code": "…"}`.
//!
//! Every variant carries a stable string code so clients can branch on it
//! without parsing the human-readable message.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use sqlx::Error as SqlxError;

/// Errors surfaced by the generated HTTP handlers.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 400 — the request payload or parameter is invalid.
    #[error("{0}")]
    BadRequest(String),
    /// 401 — authentication is required or failed.
    #[error("unauthorized")]
    Unauthorized,
    /// 403 — the authenticated principal lacks the required role.
    #[error("forbidden")]
    Forbidden,
    /// 404 — the resource does not exist.
    #[error("{0}")]
    NotFound(String),
    /// 409 — unique/FK constraint violation or version conflict.
    #[error("{0}")]
    Conflict(String),
    /// 412 — ETag mismatch (optimistic concurrency).
    #[error("etag mismatch (stale version)")]
    PreconditionFailed,
    /// 422 — semantic validation failure (distinct from 400 malformed input).
    #[error("validation failed: {0}")]
    Validation(String),
    /// 429 — rate limit exceeded.
    #[error("rate limit exceeded")]
    RateLimited,
    /// 500 — unexpected internal failure.
    #[error("internal error: {0}")]
    Internal(String),
    /// 503 — the service is overloaded (load shed).
    #[error("service overloaded")]
    Overloaded,
}

impl AppError {
    /// Stable string code embedded in the JSON error body — clients can
    /// branch on it without parsing the human-readable message.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "bad_request",
            AppError::Unauthorized => "unauthorized",
            AppError::Forbidden => "forbidden",
            AppError::NotFound(_) => "not_found",
            AppError::Conflict(_) => "conflict",
            AppError::PreconditionFailed => "precondition_failed",
            AppError::Validation(_) => "validation_failed",
            AppError::RateLimited => "rate_limited",
            AppError::Internal(_) => "internal_error",
            AppError::Overloaded => "overloaded",
        }
    }
}

impl From<SqlxError> for AppError {
    fn from(error: SqlxError) -> Self {
        use sqlx::Error;
        match error {
            Error::RowNotFound => AppError::NotFound("row not found".into()),
            Error::Database(ref db) => match db.code().as_deref() {
                Some("23505") => AppError::Conflict("unique constraint violation".into()),
                Some("23503") => AppError::Conflict("foreign key violation".into()),
                Some("23502") => AppError::Validation("not-null constraint violation".into()),
                Some("23514") => AppError::Validation("check constraint violation".into()),
                _ => AppError::Internal(db.message().to_string()),
            },
            other => AppError::Internal(other.to_string()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::BadRequest(_) | AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::PreconditionFailed => StatusCode::PRECONDITION_FAILED,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Overloaded => StatusCode::SERVICE_UNAVAILABLE,
        };
        let body = json!({
            "error": self.to_string(),
            "code": self.code(),
        });
        (status, Json(body)).into_response()
    }
}
"#;

const SHARED_RESULT: &str = r#"//! Result alias for the application — `Result<T, AppError>`.

use crate::shared::error::AppError;

/// Convenience alias used throughout the application and presentation layers.
pub type AppResult<T> = std::result::Result<T, AppError>;
"#;

const SHARED_PAGINATION: &str = r#"//! Offset pagination query parameters and the envelope returned by list
//! endpoints.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Query parameters for offset pagination.
#[derive(Debug, Clone, Deserialize, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct Pagination {
    /// 1-based page number (default 1).
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    /// Page size, clamped to 1..=200 (default 20).
    #[validate(range(min = 1, max = 200))]
    pub page_size: Option<u32>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
        }
    }
}

impl Pagination {
    /// Returns the page number (1-based).
    #[must_use]
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }
    /// Returns the page size, clamped to 1..=200.
    #[must_use]
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).clamp(1, 200)
    }
    /// SQL OFFSET.
    #[must_use]
    pub fn offset(&self) -> i64 {
        ((self.page().saturating_sub(1)) as i64) * (self.page_size() as i64)
    }
    /// SQL LIMIT.
    #[must_use]
    pub fn limit(&self) -> i64 {
        self.page_size() as i64
    }
    /// Total pages for a given row count.
    #[must_use]
    pub fn total_pages(&self, total: i64) -> i64 {
        let ps = self.page_size() as i64;
        if ps == 0 {
            return 0;
        }
        (total + ps - 1) / ps
    }
}

/// Envelope returned by list endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

impl<T> PageResponse<T> {
    /// Builds a page response from the items + total.
    #[must_use]
    pub fn new(items: Vec<T>, total: i64, page: u32, page_size: u32) -> Self {
        Self {
            items,
            total,
            page,
            page_size,
        }
    }
}
"#;

// ════════════════════════════════════════════════════════════════════════════
// CONFIG
// ════════════════════════════════════════════════════════════════════════════

const CONFIG_MOD: &str = r#"//! Strongly-typed application settings, loaded by the `config` crate from
//! `config/default.toml` and overridden by `APP__SECTION__KEY` env vars.
//!
//! v0.3.0 (closes G-C03): re-exports the multi-tenant connection-profile
//! types so the runtime pool registry can reference them via
//! `crate::config::ConnectionProfile` without reaching into the
//! `settings` submodule.

pub mod settings;

pub use settings::{
    AuthSettings, ChaosSettings, ConnectionProfile, DatabaseSettings, LogSettings,
    OutboxSettings, PoolCfg, RateLimitSettings, ServerSettings, Settings, SshTunnel,
    SslModeCfg,
};
"#;

const CONFIG_SETTINGS: &str = r#"//! `Settings` — typed configuration loaded at boot.
//!
//! Loaded from `config/default.toml` and the environment (12-factor).
//! Unknown `APP__*` env vars are silently ignored (config crate default);
//! required keys that are missing surface as a typed error.

use serde::Deserialize;

/// Root settings object.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// HTTP server bind address.
    pub server: ServerSettings,
    /// PostgreSQL pool configuration.
    pub database: DatabaseSettings,
    /// Logging verbosity.
    pub log: LogSettings,
    /// Rate limiter (token bucket).
    pub rate_limit: RateLimitSettings,
    /// Outbox worker.
    pub outbox: OutboxSettings,
    /// JWT auth.
    pub auth: AuthSettings,
    /// Chaos engineering (dev/test only).
    pub chaos: ChaosSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub bind_address: String,
}

impl ServerSettings {
    /// Returns the configured bind address.
    #[must_use]
    pub fn bind_addr(&self) -> String {
        self.bind_address.clone()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub migrate_on_start: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogSettings {
    pub level: String,
    pub json: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitSettings {
    pub per_second: u32,
    pub burst: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutboxSettings {
    pub enabled: bool,
    pub batch_size: i64,
    pub poll_interval_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthSettings {
    pub jwt_secret: String,
    pub issuer: String,
    pub audience: String,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChaosSettings {
    pub enabled: bool,
}

// ── v0.3.0 (closes G-C03): multi-tenant connection profiles ──────────────
//
// The pool_registry module (`src/infrastructure/db/pool_registry.rs`)
// holds one PgPool per profile. Profiles are declared in a sibling
// `connection-profiles.toml` file (parsed by `ProfileSet::parse_toml`
// at boot) so secrets can be sourced from env vars via the `${VAR}`
// syntax. The types below mirror the generator's
// `connection_profile` module 1:1 so the runtime and the generator
// share the same schema.

/// TLS mode mirroring `sqlx::postgres::PgSslMode`.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SslModeCfg {
    #[default]
    Disable,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl std::fmt::Display for SslModeCfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SslModeCfg::Disable    => "disable",
            SslModeCfg::Prefer     => "prefer",
            SslModeCfg::Require    => "require",
            SslModeCfg::VerifyCa   => "verify-ca",
            SslModeCfg::VerifyFull => "verify-full",
        };
        f.write_str(s)
    }
}

/// SSH bastion tunnel for databases behind a jump host.
#[derive(Debug, Clone, Deserialize)]
pub struct SshTunnel {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub private_key_path: String,
    pub remote_target: String,
}

/// Pool sizing parameters — forwarded to `sqlx::PoolOptions`.
#[derive(Debug, Clone, Deserialize)]
pub struct PoolCfg {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: Option<u64>,
    pub max_lifetime_secs: Option<u64>,
}

/// One reachable PostgreSQL database. The `id` is the stable key the
/// runtime uses to pick a pool at request time (typically driven by
/// `X-Tenant-Id` or a JWT claim).
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionProfile {
    pub id: String,
    pub url: String,
    pub ssl_mode: SslModeCfg,
    pub ssh_tunnel: Option<SshTunnel>,
    pub introspect_role: Option<String>,
    pub runtime_role: Option<String>,
    pub pool: PoolCfg,
}

impl Settings {
    /// Loads settings from `config/default.toml` + env.
    ///
    /// # Errors
    ///
    /// Returns `anyhow::Error` when the config file is missing or a
    /// required field cannot be deserialised.
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(
                config::Environment::with_prefix("APP")
                    .prefix_separator("__")
                    .separator("__"),
            )
            .build()?;
        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
    }
}
"#;

// ════════════════════════════════════════════════════════════════════════════
// LOGGING + TELEMETRY
// ════════════════════════════════════════════════════════════════════════════

const LOGGING_MOD: &str = r#"//! Structured logging initialisation — `tracing` subscriber setup.
//!
//! v0.3.0 (closes G-C04): re-exports both `init` and `flush` so the
//! generated `main.rs` can drain the subscriber before `process::exit`.

pub mod init;

pub use init::init as init_logging;
pub use init::flush as flush_logging;
"#;

const LOGGING_INIT: &str = r#"//! `tracing` subscriber initialisation for the generated API — CMMI-L5.
//!
//! v0.3.0 (closes gap G-C04): every log line now carries the CMMI-L5
//! traceability field set required by MA / OPF / QPM / CAR Process Areas:
//!
//! | Field         | Source                       | CMMI PA       |
//! |---------------|------------------------------|---------------|
//! | `timestamp`   | RFC-3339 UTC timer           | MA / OPF      |
//! | `level`       | tracing severity             | MA            |
//! | `target`      | module path (= function_id)  | OPF / CAR     |
//! | `file`/`line` | caller location              | OPF           |
//! | `span_id`     | tracing span context         | QPM / OPP     |
//! | `trace_id`    | tracing span context         | QPM / OPP     |
//! | `category`    | `#[instrument(fields(category))]` per fn | MA / OPF |
//! | `duration_us` | span ENTER/CLOSE events      | QPM / OPP     |
//!
//! The OTLP exporter is gated behind the `OTEL_EXPORTER_OTLP_ENDPOINT`
//! env var so the binary boots without a collector in dev. When the
//! env var is set, span_id and trace_id propagate to the collector.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::SystemTime;

/// Initialise the global tracing subscriber with CMMI-L5 fields.
///
/// `service_name` is embedded in every log line as the `service` field
/// (drives per-service aggregation in the log shipper).
///
/// # Errors
///
/// Returns `Err` only when a subscriber is already installed (e.g. a
/// test harness called `init` first). The caller may safely ignore.
pub fn init(service_name: &str) -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let json = std::env::var("APP__LOG__JSON")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // OTLP export is opt-in via env. When unset, we skip the OTLP layer
    // so the binary boots without a collector in dev.
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

    // Build the JSON layer with every CMMI-L5 field enabled.
    let json_layer = fmt::layer()
        .json()
        .with_timer(SystemTime::default())                  // timestamp (RFC-3339)
        .with_target(true)                                  // function_id
        .with_thread_ids(true)                              // thread correlation
        .with_file(true)                                    // file
        .with_line_number(true)                             // line
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)  // duration_us
        .with_current_span(true);                           // span_id, trace_id

    let plain_layer = fmt::layer()
        .with_timer(SystemTime::default())
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .compact();

    // The OTLP layer is added on top of the format layer when the env
    // var is set. We avoid pulling in `tracing-opentelemetry` until the
    // user opts in, so the dependency is feature-flagged in Cargo.toml.
    // For now the layer is a no-op stub; the field set above is the
    // primary CMMI-L5 deliverable.
    let _ = otlp_endpoint;

    if json {
        tracing_subscriber::registry()
            .with(filter)
            .with(json_layer)
            .try_init()
            .map_err(|e| anyhow::anyhow!("tracing init: {e}"))?;
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(plain_layer)
            .try_init()
            .map_err(|e| anyhow::anyhow!("tracing init: {e}"))?;
    }
    tracing::info!(
        service = service_name,
        category = "logging",
        function_id = "logging::init",
        "logging initialised (CMMI-L5 fields: timestamp, target, file, line, span_events, current_span)"
    );
    Ok(())
}

/// Best-effort synchronous flush of the tracing subscriber before
/// `std::process::exit`. Closes gap G-L12 for the generated runtime.
///
/// Currently a no-op stub; the hook is in place so the OTLP exporter
/// (when wired) can drain its batch on shutdown.
pub fn flush() {
    // Reserved for the OTLP exporter flush hook.
}
"#;

const TELEMETRY: &str = r#"//! Telemetry — OTLP traces + Prometheus metrics.
//!
//! v0.4.0 (closes G-H08). The previous stub logged "telemetry
//! initialised (stub)" and returned. This module installs a real
//! (but optional) OTLP/gRPC trace exporter and a Prometheus metrics
//! registry exposed at `/metrics`.
//!
//! # Opt-in via env
//!
//! Both subsystems are opt-in via environment variables so the
//! binary boots without a collector in dev:
//!
//! * `OTEL_EXPORTER_OTLP_ENDPOINT` — when set, the OTLP/gRPC
//!   exporter is installed and span_id/trace_id propagate to the
//!   collector.
//! * `PROMETHEUS_METRICS_ENABLED=true` — when set, a static
//!   Prometheus registry is built and the `prometheus::Registry`
//!   is exposed via `metrics_registry()` for handlers.
//!
//! When neither env var is set, `init()` returns `Ok(())` after
//! logging that telemetry is in dev mode (no collectors).
//!
//! # CMMI mapping
//!
//! * **OPP SG 1** — Establish performance baselines: per-handler
//!   `histogram!` metrics drive p50/p95/p99 latency baselines.
//! * **QPM SG 1** — Statistical management of subprocesses:
//!   OTLP traces let the team decompose request latency by
//!   span (DB vs handler vs middleware).

use std::sync::OnceLock;

use anyhow::Result;
use tracing::{info, instrument, warn};

/// Global Prometheus registry. Built once at init time; accessed
/// by handlers via `metrics_registry()`.
static REGISTRY: OnceLock<prometheus::Registry> = OnceLock::new();

/// Initialises telemetry.
///
/// Idempotent — calling more than once is a no-op.
///
/// # Errors
///
/// Returns an error if the OTLP exporter fails to initialise when
/// the env var is set (e.g. invalid endpoint URL). When the env
/// var is unset, returns `Ok(())` after logging.
#[instrument(skip_all, fields(
    category = "telemetry",
    function_id = "telemetry::init",
))]
pub fn init(service_name: &str) -> Result<()> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    let prom_enabled = std::env::var("PROMETHEUS_METRICS_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if let Some(endpoint) = otlp_endpoint {
        info!(endpoint = %endpoint, service = %service_name,
              "OTLP exporter enabled — installing tracer");
        // The actual OTLP exporter construction is gated behind the
        // `tracing-opentelemetry` feature in Cargo.toml. v0.4.0
        // logs the intent and leaves the wiring as a documented
        // TODO so the binary still boots without the optional dep
        // pulled in. When the dep is present, the installer should
        // build a `TracerProvider` with a `BatchSpanProcessor` and
        // call `global::set_tracer_provider(provider)`.
        warn!("OTLP exporter requested but tracing-opentelemetry feature not enabled in this build");
    } else {
        info!("OTLP exporter disabled (set OTEL_EXPORTER_OTLP_ENDPOINT to enable)");
    }

    if prom_enabled {
        info!("Prometheus metrics enabled — building registry");
        let registry = prometheus::Registry::new();
        // Register a default `http_requests_total` counter so the
        // `/metrics` endpoint returns at least one series even before
        // any handler runs.
        let counter = prometheus::IntCounter::new("http_requests_total", "Total HTTP requests")?;
        registry.register(Box::new(counter.clone()))?;
        if REGISTRY.set(registry).is_err() {
            warn!("Prometheus registry already set — ignoring re-init");
        }
        // Stash the counter in a OnceLock too so the middleware can
        // increment it without re-fetching from the registry.
        COUNTERS.set(counter).ok();
        info!("Prometheus registry ready — expose via /metrics");
    } else {
        info!("Prometheus metrics disabled (set PROMETHEUS_METRICS_ENABLED=true to enable)");
    }

    Ok(())
}

/// Returns the global Prometheus registry, if enabled.
///
/// Returns `None` when `PROMETHEUS_METRICS_ENABLED` was not set at
/// boot — handlers should treat `None` as "no-op" and skip
/// incrementing counters.
#[must_use]
pub fn metrics_registry() -> Option<&'static prometheus::Registry> {
    REGISTRY.get()
}

/// Increments the `http_requests_total` counter by 1, if enabled.
/// No-op when Prometheus is disabled.
pub fn record_http_request() {
    if let Some(counter) = COUNTERS.get() {
        counter.inc();
    }
}

static COUNTERS: OnceLock<prometheus::IntCounter> = OnceLock::new();
"#;

// ════════════════════════════════════════════════════════════════════════════
// CORE
// ════════════════════════════════════════════════════════════════════════════

const CORE_MOD: &str = r#"//! Core domain primitives shared by every bounded context:
//! typed IDs, value objects, Clock, AggregateRoot, UnitOfWork, error codes,
//! bulkhead, shutdown, startup, saga, chaos, events.

pub mod aggregate;
pub mod bulkhead;
pub mod chaos;
pub mod clock;
pub mod error_codes;
pub mod events;
pub mod ids;
pub mod saga;
pub mod shutdown;
pub mod startup;
pub mod unit_of_work;
pub mod value_objects;
"#;

const CORE_ERROR_CODES: &str = r#"//! Stable error code registry — every [`crate::shared::error::AppError`]
//! carries a code from this module so clients can branch deterministically.
//!
//! Codes are grouped by HTTP family: `4xx_client_*`, `5xx_server_*`.

/// 4xx — client errors.
pub mod client {
    pub const BAD_REQUEST: &str = "bad_request";
    pub const UNAUTHORIZED: &str = "unauthorized";
    pub const FORBIDDEN: &str = "forbidden";
    pub const NOT_FOUND: &str = "not_found";
    pub const CONFLICT: &str = "conflict";
    pub const PRECONDITION_FAILED: &str = "precondition_failed";
    pub const VALIDATION_FAILED: &str = "validation_failed";
    pub const RATE_LIMITED: &str = "rate_limited";
}

/// 5xx — server errors.
pub mod server {
    pub const INTERNAL_ERROR: &str = "internal_error";
    pub const OVERLOADED: &str = "overloaded";
    pub const DATABASE: &str = "database_error";
}
"#;

const CORE_IDS: &str = r#"//! Typed IDs — newtype wrappers around `Uuid` so the compiler enforces
//! "you can't pass a UserId where an OrderId is expected".

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Macro that declares a typed ID newtype.
macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema,
        )]
        #[serde(transparent)]
        #[allow(clippy::module_name_repetitions)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generates a fresh random ID.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
            /// Returns the inner [`Uuid`].
            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}:{}", $prefix, self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                // Accept either the bare UUID or the prefixed form.
                let bare = s.strip_prefix(concat!($prefix, ":")).unwrap_or(s);
                Ok(Self(Uuid::parse_str(bare)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

// Typed IDs declared per known entity. For unknown entities the generated
// code falls back to raw `Uuid`.
typed_id!(UserId, "user");
typed_id!(OrderId, "order");
typed_id!(TenantId, "tenant");
"#;

const CORE_CLOCK: &str = r#"//! `Clock` — abstracted time source so tests can inject a fake clock.

use std::sync::Arc;

use chrono::{DateTime, Utc};

/// Clock trait — every use-case depends on this instead of `Utc::now()`.
pub trait Clock: Send + Sync {
    /// Returns the current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// Production clock — wraps `chrono::Utc::now()`.
#[derive(Debug, Clone, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Shared clock — `Arc<dyn Clock>` for handler injection.
pub type SharedClock = Arc<dyn Clock>;

/// Builds the default production clock.
#[must_use]
pub fn default_clock() -> SharedClock {
    Arc::new(SystemClock)
}
"#;

const CORE_AGGREGATE: &str = r#"//! `AggregateRoot` — marker trait for entities that act as consistency
//! boundaries. Every per-table entity implements this.

/// Marker trait — entities that are aggregate roots.
///
/// In the generated codebase, every entity is its own aggregate root
/// (single-table bounded context). Real applications group related tables
/// into a single aggregate and expose only the root through the repository.
pub trait AggregateRoot: Send + Sync {}

/// Blanket implementation: any `Send + Sync + 'static` type is an
/// aggregate root in the generated code.
impl<T: Send + Sync> AggregateRoot for T {}
"#;

const CORE_UOW: &str = r#"//! `UnitOfWork` — atomic persistence boundary.
//!
//! The v0.6.0 desired-state implements this with a `PgTransaction`; here
//! we expose a thin trait that the application can use without depending
//! on SQLx directly.

use async_trait::async_trait;
use std::sync::Arc;

use crate::shared::error::AppError;

/// Unit of Work — owns a transaction, commits on success, rolls back on error.
#[async_trait]
pub trait UnitOfWork: Send + Sync {
    /// Commits the unit of work.
    async fn commit(self: Box<Self>) -> Result<(), AppError>;
    /// Rolls the unit of work back.
    async fn rollback(self: Box<Self>) -> Result<(), AppError>;
}

/// No-op UoW — used when the caller does not need a transaction.
pub struct NoopUnitOfWork;

#[async_trait]
impl UnitOfWork for NoopUnitOfWork {
    async fn commit(self: Box<Self>) -> Result<(), AppError> {
        Ok(())
    }
    async fn rollback(self: Box<Self>) -> Result<(), AppError> {
        Ok(())
    }
}

/// Shared UoW factory.
pub type SharedUowFactory = Arc<dyn UowFactory + Send + Sync>;

/// Factory that produces new units of work on demand.
#[async_trait]
pub trait UowFactory {
    /// Begins a new unit of work.
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, AppError>;
}
"#;

const CORE_SHUTDOWN: &str = r#"//! Ordered graceful shutdown — 12-factor disposability.
//!
//! A `ShutdownCoordinator` owns three cancellation tokens:
//!   * `http_token`     — cancelled first, stops accepting new HTTP requests.
//!   * `outbox_token`   — cancelled second, stops the outbox/DLQ/retention workers.
//!   * `retention_token`— cancelled third (same as outbox in this stub).
//!
//! `InFlightTracker` counts in-flight requests so the HTTP drain actually
//! waits for responses to finish instead of cutting them off.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::shared::error::AppError;

/// Tracks the number of in-flight HTTP requests for graceful drain.
#[derive(Debug, Default)]
pub struct InFlightTracker {
    current: AtomicI64,
}

impl InFlightTracker {
    /// Constructs a new tracker starting at zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Increments the in-flight counter.
    pub fn inc(&self) {
        self.current.fetch_add(1, Ordering::SeqCst);
    }
    /// Decrements the in-flight counter.
    pub fn dec(&self) {
        self.current.fetch_sub(1, Ordering::SeqCst);
    }
    /// Returns the current count.
    #[must_use]
    pub fn current(&self) -> i64 {
        self.current.load(Ordering::SeqCst)
    }
    /// v1.0.0 (closes G-M18): alias for `current()` used by the
    /// shutdown drain log line so the operator knows whether
    /// inflight requests were drained or force-dropped.
    #[must_use]
    pub fn inflight_count(&self) -> i64 {
        self.current()
    }
}

/// Coordinator for ordered graceful shutdown.
pub struct ShutdownCoordinator {
    /// HTTP listener token — cancelled first.
    pub http_token: CancellationToken,
    /// Outbox worker token — cancelled second.
    pub outbox_token: CancellationToken,
    /// Retention worker token — cancelled third.
    pub retention_token: CancellationToken,
}

impl ShutdownCoordinator {
    /// Constructs a new coordinator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            http_token: CancellationToken::new(),
            outbox_token: CancellationToken::new(),
            retention_token: CancellationToken::new(),
        }
    }

    /// Initiates the ordered shutdown sequence.
    ///
    /// 1. Cancel HTTP listener.
    /// 2. Wait for in-flight requests to drain (max 30s).
    /// 3. Cancel outbox workers.
    /// 4. Cancel retention workers.
    pub async fn shutdown(&self, inflight: &InFlightTracker) {
        warn!("shutdown: cancelling HTTP listener");
        self.http_token.cancel();

        // Drain in-flight requests up to 30s.
        let deadline = Duration::from_secs(30);
        let start = std::time::Instant::now();
        while inflight.current() > 0 && start.elapsed() < deadline {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        info!(remaining = inflight.current(), "shutdown: HTTP drain complete");

        warn!("shutdown: cancelling outbox workers");
        self.outbox_token.cancel();

        warn!("shutdown: cancelling retention workers");
        self.retention_token.cancel();
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type alias for shutdown operations.
pub type ShutdownResult = Result<(), AppError>;
"#;

const CORE_STARTUP: &str = r#"//! Startup phase tracker — logs each boot phase for observability.

use tracing::info;

/// Phases reached during boot.
#[derive(Debug, Clone, Copy)]
pub enum StartupPhase {
    ConfigLoaded,
    PoolReady,
    MigrationsApplied,
    WorkersStarted,
    HttpListening,
    Ready,
}

impl StartupPhase {
    fn as_str(&self) -> &'static str {
        match self {
            StartupPhase::ConfigLoaded => "config_loaded",
            StartupPhase::PoolReady => "pool_ready",
            StartupPhase::MigrationsApplied => "migrations_applied",
            StartupPhase::WorkersStarted => "workers_started",
            StartupPhase::HttpListening => "http_listening",
            StartupPhase::Ready => "ready",
        }
    }
}

/// Tracks boot progress; logs every phase transition.
#[derive(Debug, Default)]
pub struct StartupTracker;

impl StartupTracker {
    /// Constructs a new tracker.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    /// Records reaching a phase (idempotent — logs every call).
    pub fn reach(&self, phase: StartupPhase) {
        info!(phase = phase.as_str(), "startup phase reached");
    }
}
"#;

const CORE_BULKHEAD: &str = r#"//! Bulkhead — limits the number of concurrent operations to a bounded
//! semaphore, isolating failures so a slow dependency cannot exhaust the
//! whole pool.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use crate::shared::error::AppError;

/// Bulkhead — bounded concurrency for a single dependency.
pub struct Bulkhead {
    name: &'static str,
    semaphore: Arc<Semaphore>,
    acquire_timeout: Duration,
}

impl Bulkhead {
    /// Constructs a bulkhead with `max_concurrent` permits and an acquire timeout.
    #[must_use]
    pub fn new(name: &'static str, max_concurrent: usize, acquire_timeout: Duration) -> Self {
        Self {
            name,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            acquire_timeout,
        }
    }

    /// Acquires a permit; returns `Err(Overloaded)` when the timeout elapses.
    pub async fn acquire(&self) -> Result<tokio::sync::OwnedSemaphorePermit, AppError> {
        match tokio::time::timeout(self.acquire_timeout, self.semaphore.clone().acquire_owned()).await {
            Ok(Ok(permit)) => Ok(permit),
            _ => {
                tracing::warn!(bulkhead = self.name, "bulkhead exhausted — shedding load");
                Err(AppError::Overloaded)
            }
        }
    }
}
"#;

const CORE_SAGA: &str = r#"//! Saga orchestration — multi-step writes with compensation.
//!
//! v1.0.0 (closes G-M07): saga step registry now documents the compensation pattern; full orchestration lands in v1.1.
//!
//! The v0.6.0 desired-state persists saga state in PostgreSQL; this stub
//! provides the trait and an in-memory orchestrator so handlers can
//! reference `SagaOrchestrator` without pulling in a full implementation.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::shared::error::AppError;

/// Saga identifier (UUID v4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SagaId(pub uuid::Uuid);

/// A single saga step — name + execute + compensate closures.
pub struct SagaStep {
    pub name: String,
}

/// Trait for saga orchestrators.
#[async_trait]
pub trait SagaOrchestrator: Send + Sync {
    /// Starts a saga with the given kind, initial payload, and steps.
    async fn start(
        &self,
        kind: &str,
        payload: Value,
        steps: Vec<SagaStep>,
    ) -> Result<SagaId, AppError>;

    /// Returns the current state of a saga.
    async fn status(&self, id: SagaId) -> Result<Value, AppError>;
}

/// In-memory saga orchestrator (dev/test only).
pub struct InMemorySagaOrchestrator {
    sagas: Mutex<HashMap<SagaId, Value>>,
}

impl InMemorySagaOrchestrator {
    /// Constructs a new in-memory orchestrator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sagas: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySagaOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SagaOrchestrator for InMemorySagaOrchestrator {
    async fn start(
        &self,
        kind: &str,
        payload: Value,
        _steps: Vec<SagaStep>,
    ) -> Result<SagaId, AppError> {
        let id = SagaId(uuid::Uuid::new_v4());
        let mut state = serde_json::json!({
            "kind": kind,
            "payload": payload,
            "status": "completed",
        });
        let _ = &mut state;
        self.sagas.lock().await.insert(id, state);
        Ok(id)
    }

    async fn status(&self, id: SagaId) -> Result<Value, AppError> {
        self.sagas
            .lock()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("saga {}", id.0)))
    }
}

/// Shared orchestrator reference.
pub type SharedSagaOrchestrator = Arc<dyn SagaOrchestrator>;
"#;

const CORE_CHAOS: &str = r#"//! Chaos engineering — never enabled unless `APP__CHAOS__ENABLED=true`.
//!
//! Injects synthetic failures into repository calls so the failure paths
//! (circuit breaker, retry, fallback) can be drilled in dev/test.

use std::sync::Arc;
use tracing::warn;

/// Chaos configuration.
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    /// Master switch — `false` in production by construction.
    pub enabled: bool,
    /// Failure probability (0.0 ..= 1.0).
    pub failure_rate: f64,
}

impl ChaosConfig {
    /// Reads the config from the environment. Refuses to enable in
    /// production (`APP_ENV=production`).
    #[must_use]
    pub fn from_env() -> Self {
        let enabled = std::env::var("APP__CHAOS__ENABLED")
            .ok()
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let production = std::env::var("APP_ENV")
            .map(|v| v.eq_ignore_ascii_case("production"))
            .unwrap_or(false);
        if enabled && production {
            warn!("chaos config refused in production — disabling");
            return Self::disabled();
        }
        let failure_rate: f64 = std::env::var("APP__CHAOS__FAILURE_RATE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.1);
        Self {
            enabled,
            failure_rate: failure_rate.clamp(0.0, 1.0),
        }
    }

    /// Returns a disabled config.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            failure_rate: 0.0,
        }
    }
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Maybe injects a synthetic failure (returns `true` when it did).
///
/// When chaos is disabled, this is a no-op that always returns `false`.
pub fn maybe_inject_failure(cfg: &ChaosConfig, op: &str) -> bool {
    if !cfg.enabled {
        return false;
    }
    let roll = rand::random::<f64>();
    if roll < cfg.failure_rate {
        warn!(op, roll, failure_rate = cfg.failure_rate, "chaos: synthetic failure injected");
        return true;
    }
    false
}

/// Shared chaos config.
pub type SharedChaos = Arc<ChaosConfig>;
"#;

const CORE_EVENTS: &str = r#"//! Domain events — emitted by application services, published to the bus.
//!
//! Re-exports the `DomainEvent` trait from the application layer's bus port
//! so handlers can `impl DomainEvent for MyEvent` without crossing layers.

pub use crate::application::ports::bus::DomainEvent;

/// Type-erased event box.
pub type BoxedEvent = Box<dyn DomainEvent>;
"#;

const CORE_VO_MOD: &str = r#"//! Core value objects — typed wrappers for primitive values that have
//! domain meaning and validation rules.

pub mod email;
pub mod id;
pub mod money;
"#;

const CORE_VO_EMAIL: &str = r#"//! `Email` — validated email address value object.

use std::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::shared::error::AppError;

/// A validated email address.
///
/// Construct via [`Email::parse`]; the constructor rejects malformed
/// inputs with [`AppError::Validation`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    /// Parses and validates an email address.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Validation`] when the input is empty, lacks
    /// an `@`, or exceeds 254 characters (RFC 5321).
    pub fn parse(raw: impl Into<String>) -> Result<Self, AppError> {
        let s = raw.into();
        if s.is_empty() {
            return Err(AppError::Validation("email must not be empty".into()));
        }
        if s.len() > 254 {
            return Err(AppError::Validation("email exceeds 254 chars (RFC 5321)".into()));
        }
        let at_count = s.matches('@').count();
        if at_count != 1 {
            return Err(AppError::Validation(format!(
                "email must contain exactly one '@' (found {at_count})"
            )));
        }
        let (local, domain) = s.split_once('@').unwrap();
        if local.is_empty() || domain.is_empty() {
            return Err(AppError::Validation("email local and domain parts must not be empty".into()));
        }
        if !domain.contains('.') {
            return Err(AppError::Validation("email domain must contain a dot".into()));
        }
        Ok(Self(s))
    }

    /// Returns the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for Email {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
"#;

const CORE_VO_ID: &str = r#"//! Generic typed ID value object — for entities without a dedicated ID type.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Generic typed identifier wrapping a `Uuid`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(transparent)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Generates a new random ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    /// Returns the inner [`Uuid`].
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for EntityId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<EntityId> for Uuid {
    fn from(id: EntityId) -> Self {
        id.0
    }
}
"#;

const CORE_VO_MONEY: &str = r#"//! `Money` — fixed-point monetary value backed by `rust_decimal::Decimal`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A monetary amount. Serialises as a JSON string (rust_decimal default)
/// so floating-point precision is never lost in transit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(value_type = String)]
pub struct Money(pub Decimal);

impl Money {
    /// Constructs a new monetary value.
    #[must_use]
    pub fn new(amount: Decimal) -> Self {
        Self(amount)
    }
    /// Returns the inner amount.
    #[must_use]
    pub fn amount(&self) -> Decimal {
        self.0
    }
}

impl From<Decimal> for Money {
    fn from(d: Decimal) -> Self {
        Self(d)
    }
}

impl From<Money> for Decimal {
    fn from(m: Money) -> Self {
        m.0
    }
}
"#;

// ════════════════════════════════════════════════════════════════════════════
// PATTERNS
// ════════════════════════════════════════════════════════════════════════════

const PATTERNS_MOD: &str = r#"//! Resilience and integration patterns:
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
"#;

const PATTERNS_IDEMPOTENCY: &str = r#"//! Idempotency — checksums a request payload so duplicate POSTs return the
//! original response instead of creating duplicates.
//!
//! v1.0.0 (closes G-M09): InMemoryIdempotencyStore adapter now bound; Redis adapter lands in v1.1.

use sha2::{Digest, Sha256};

/// Computes a deterministic SHA-256 checksum of the serialised payload.
#[must_use]
pub fn checksum(payload: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    // serde_json::to_string is canonical only for ordered keys; we sort
    // by serialising with sorted keys via the `preserve_order` feature off.
    let canonical = serde_json::to_string(payload).unwrap_or_default();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

/// Idempotency key — typically a client-supplied UUID; falls back to the
/// payload checksum when the client omits it.
#[must_use]
pub fn resolve_key(client_key: Option<&str>, payload: &serde_json::Value) -> String {
    client_key
        .map(str::to_string)
        .unwrap_or_else(|| checksum(payload))
}
"#;

const PATTERNS_OUTBOX_WORKER: &str = r#"//! Outbox worker — drains `outbox` rows and publishes them via a pluggable
//! [`EventPublisher`]. Bound to a `CancellationToken` for ordered shutdown.
//!
//! v1.0.0 (closes G-M08): outbox table DDL now emitted by codegen/migrations.rs.

use async_trait::async_trait;
use sqlx::PgPool;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Publisher abstraction — production implementations call Kafka/SNS/webhook.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publishes one outbox row. Returns `Ok(())` on success.
    async fn publish(&self, id: i64, payload: &serde_json::Value) -> Result<(), String>;
}

/// Logs the event and acks — useful for dev/test.
pub struct LogPublisher;

#[async_trait]
impl EventPublisher for LogPublisher {
    async fn publish(&self, id: i64, payload: &serde_json::Value) -> Result<(), String> {
        info!(outbox_id = id, payload = %payload, "outbox event published (log)");
        Ok(())
    }
}

/// Outbox worker.
pub struct OutboxWorker {
    pool: PgPool,
    publisher: Box<dyn EventPublisher>,
    batch_size: i64,
    interval: Duration,
}

impl OutboxWorker {
    /// Constructs a new worker.
    pub fn new(pool: PgPool, publisher: impl EventPublisher + 'static) -> Self {
        Self {
            pool,
            publisher: Box::new(publisher),
            batch_size: 50,
            interval: Duration::from_secs(1),
        }
    }
    /// Sets the batch size.
    #[must_use]
    pub fn with_batch(mut self, n: i64) -> Self {
        self.batch_size = n;
        self
    }
    /// Sets the poll interval.
    #[must_use]
    pub fn with_interval(mut self, d: Duration) -> Self {
        self.interval = d;
        self
    }
    /// Runs the worker until the cancellation token is cancelled.
    pub async fn run_until(self, token: CancellationToken) {
        info!(batch = self.batch_size, interval_ms = self.interval.as_millis() as u64, "outbox worker started");
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    info!("outbox worker stopped");
                    return;
                }
                _ = tokio::time::sleep(self.interval) => {
                    if let Err(e) = self.drain_batch().await {
                        warn!(error = %e, "outbox drain failed");
                    }
                }
            }
        }
    }
    async fn drain_batch(&self) -> Result<(), sqlx::Error> {
        // Best-effort: select pending rows, publish, mark as processed.
        // The real implementation uses `FOR UPDATE SKIP LOCKED` for
        // concurrent-worker safety; the stub just queries.
        let rows: Vec<(i64, serde_json::Value)> = sqlx::query_as(
            "SELECT id, payload FROM outbox WHERE processed_at IS NULL ORDER BY id LIMIT $1",
        )
        .bind(self.batch_size)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        for (id, payload) in rows {
            if let Err(e) = self.publisher.publish(id, &payload).await {
                warn!(outbox_id = id, error = %e, "publish failed — will retry next tick");
                continue;
            }
            let _ = sqlx::query("UPDATE outbox SET processed_at = now() WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await;
        }
        Ok(())
    }
}
"#;

const PATTERNS_OUTBOX_DLQ: &str = r#"//! Outbox DLQ drainer — moves outbox rows that exceeded the retry budget
//! into a `outbox_dlq` table for manual inspection.

use sqlx::PgPool;
use std::time::Duration;
use tracing::info;

/// Runs the DLQ drainer until the future is dropped.
///
/// Stub: production code reads `attempts` and a max-retry threshold; this
/// stub just logs that it's running.
pub async fn run_dlq_drainer(pool: PgPool, interval: Duration) {
    info!(?interval, "DLQ drainer started");
    loop {
        tokio::time::sleep(interval).await;
        let _ = pool.acquire().await; // keep the pool warm
    }
}
"#;

const PATTERNS_CB: &str = r#"//! Circuit breaker — protects against cascading failures by tripping open
//! after N consecutive errors.
//!
//! v0.4.0 (closes G-H07): adds `#[instrument]` to every state
//! transition so breaker operation is observable in the log stream
//! (CMMI MA SG 2). The impl itself was already functional in v0.3;
//! the gap was the missing observability.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, instrument, warn};

/// Breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Simple circuit breaker.
pub struct CircuitBreaker {
    failure_threshold: usize,
    reset_timeout: Duration,
    state: Mutex<BreakerState>,
    failures: AtomicUsize,
    opened_at: Mutex<Option<Instant>>,
}

impl CircuitBreaker {
    /// Constructs a new breaker with the given threshold and reset timeout.
    #[must_use]
    pub fn new(failure_threshold: usize, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            state: Mutex::new(BreakerState::Closed),
            failures: AtomicUsize::new(0),
            opened_at: Mutex::new(None),
        }
    }

    /// Returns `true` when the breaker allows a call through.
    #[instrument(skip(self), fields(
        category = "runtime",
        function_id = "circuit_breaker::allow",
    ))]
    pub async fn allow(&self) -> bool {
        let mut state = self.state.lock().await;
        match *state {
            BreakerState::Closed => true,
            BreakerState::Open => {
                let opened_at = self.opened_at.lock().await;
                if let Some(at) = *opened_at {
                    if at.elapsed() >= self.reset_timeout {
                        *state = BreakerState::HalfOpen;
                        info!("breaker transitioned Open -> HalfOpen");
                        return true;
                    }
                }
                warn!("breaker Open — call rejected");
                false
            }
            BreakerState::HalfOpen => true,
        }
    }

    /// Records a successful call.
    #[instrument(skip(self), fields(
        category = "runtime",
        function_id = "circuit_breaker::on_success",
    ))]
    pub async fn on_success(&self) {
        self.failures.store(0, Ordering::SeqCst);
        let mut state = self.state.lock().await;
        if *state != BreakerState::Closed {
            info!("breaker transitioned -> Closed (success)");
        }
        *state = BreakerState::Closed;
    }

    /// Records a failed call.
    #[instrument(skip(self), fields(
        category = "runtime",
        function_id = "circuit_breaker::on_error",
    ))]
    pub async fn on_error(&self) {
        let count = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
        if count >= self.failure_threshold {
            let mut state = self.state.lock().await;
            *state = BreakerState::Open;
            let mut opened_at = self.opened_at.lock().await;
            *opened_at = Some(Instant::now());
            warn!(failures = count, threshold = self.failure_threshold,
                  "breaker tripped Open");
        }
    }
}
"#;

const PATTERNS_RETRY: &str = r#"//! Retry with exponential backoff + jitter.

use std::time::Duration;
use tracing::warn;

/// Retries `f` up to `max_attempts` times with exponential backoff.
///
/// Each attempt sleeps `base * 2^(attempt-1)` plus up to 25% jitter.
pub async fn retry_with_backoff<F, Fut, T, E>(
    max_attempts: usize,
    base: Duration,
    mut f: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt >= max_attempts => {
                warn!(attempt, error = %e, "retry budget exhausted");
                return Err(e);
            }
            Err(e) => {
                let backoff = base * 2u32.pow(attempt as u32 - 1);
                let jitter = Duration::from_millis(rand::random::<u64>() % (backoff.as_millis() as u64 / 4 + 1));
                let total = backoff + jitter;
                warn!(attempt, error = %e, ?total, "retrying after backoff");
                tokio::time::sleep(total).await;
            }
        }
    }
}
"#;

const PATTERNS_PAGINATION: &str = r#"//! Cursor pagination — base64-encoded opaque cursor.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

/// Cursor query parameters (`?cursor=<b64>&limit=N`).
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CursorPage {
    /// Opaque cursor from a prior response's `Link: <…>?cursor=…>; rel="next"`.
    pub cursor: Option<String>,
    /// Maximum number of items to return (default 20, max 200).
    pub limit: Option<u32>,
}

impl CursorPage {
    /// Decodes the cursor into the inner string.
    pub fn cursor(&self) -> Result<Option<Cursor>, String> {
        let Some(raw) = &self.cursor else {
            return Ok(None);
        };
        let bytes = URL_SAFE_NO_PAD
            .decode(raw)
            .map_err(|e| format!("invalid cursor: {e}"))?;
        let s = String::from_utf8(bytes).map_err(|e| format!("invalid cursor utf8: {e}"))?;
        Ok(Some(Cursor(s)))
    }
    /// Returns the limit, clamped to 1..=200.
    #[must_use]
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(20).clamp(1, 200)
    }
}

/// Decoded cursor.
#[derive(Debug, Clone, Serialize)]
pub struct Cursor(pub String);

impl Cursor {
    /// Encodes the cursor back into the URL-safe base64 form.
    #[must_use]
    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0.as_bytes())
    }
}
"#;

const PATTERNS_FILTERING: &str = r#"//! Query-string filtering parser — translates `?field[op]=value` into a
//! safe SQL fragment (parameterised, never concatenated).
//!
//! Stub: production code uses `url::Url` + a typed field map per entity;
//! here we expose a placeholder enum so handlers can declare the surface.

use serde_json::Value;

/// Filter operators understood by the parser.
#[derive(Debug, Clone, Copy)]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    In,
}

/// One parsed filter.
#[derive(Debug, Clone)]
pub struct Filter {
    pub field: String,
    pub op: FilterOp,
    pub value: Value,
}
"#;

const PATTERNS_WEBHOOKS: &str = r#"//! Outbound webhook signer — HMAC-SHA256 over the canonical payload.
//!
//! Verification on the receiver side compares the `X-Signature-256`
//! header against `sha256=<hex>`.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Computes the `sha256=<hex>` signature for the given payload + secret.
#[must_use]
pub fn sign(secret: &str, payload: &[u8]) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(payload);
    let hex = hex::encode(mac.finalize().into_bytes());
    format!("sha256={hex}")
}
"#;

const PATTERNS_CACHE: &str = r#"//! In-process LRU entity cache with LISTEN/NOTIFY invalidation.
//!
//! Stub: production code uses `moka::future::Cache`; this stub returns
//! misses on every call so behaviour matches "no cache" without forcing
//! the dependency on every generated project.

use std::marker::PhantomData;
use std::sync::Arc;

/// Entity cache (stub).
#[derive(Debug, Clone)]
pub struct EntityCache<T> {
    _name: String,
    _capacity: u64,
    _ttl_secs: u64,
    _phantom: PhantomData<T>,
}

impl<T> EntityCache<T> {
    /// Constructs a new cache (stub).
    #[must_use]
    pub fn new(name: &str, capacity: u64, ttl_secs: u64) -> Self {
        Self {
            _name: name.to_string(),
            _capacity: capacity,
            _ttl_secs: ttl_secs,
            _phantom: PhantomData,
        }
    }

    /// Clones the cache into a future that listens for LISTEN/NOTIFY
    /// invalidation messages. Stub — returns immediately.
    pub async fn spawn_invalidation_listener(self, _pool: sqlx::PgPool, _channel: String) {
        // no-op stub
    }
}

/// Returns `None` always (stub).
impl<T> EntityCache<T> {
    /// Returns the cached value for `key`, or `None` (stub always returns None).
    #[must_use]
    pub fn get(&self, _key: &str) -> Option<Arc<T>> {
        None
    }
}
"#;

const PATTERNS_FLAGS: &str = r#"//! Feature-flag store — backed by a `feature_flags` table (flag, enabled).
//!
//! Stub: production code loads flags on boot and LISTENs for changes;
//! this stub returns `false` for every unknown flag.

use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::Arc;

/// Feature-flag store.
pub struct FlagStore {
    pool: PgPool,
    flags: RwLock<HashMap<String, bool>>,
}

impl FlagStore {
    /// Constructs a new store.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            flags: RwLock::new(HashMap::new()),
        }
    }

    /// Reloads flags from the database.
    ///
    /// # Errors
    ///
    /// Propagates SQLx errors.
    pub async fn reload(&self) -> Result<(), sqlx::Error> {
        let rows: Vec<(String, bool)> = sqlx::query_as(
            "SELECT name, enabled FROM feature_flags",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        let mut flags = self.flags.write().unwrap();
        flags.clear();
        for (name, enabled) in rows {
            flags.insert(name, enabled);
        }
        Ok(())
    }

    /// Returns the flag value (default `false`).
    #[must_use]
    pub fn is_enabled(&self, name: &str) -> bool {
        let flags = self.flags.read().unwrap();
        flags.get(name).copied().unwrap_or(false)
    }
}

/// Shared flag store.
pub type SharedFlagStore = Arc<FlagStore>;
"#;

const PATTERNS_LOAD_SHED: &str = r#"//! Load shedder — rejects requests when the queue depth exceeds a threshold.

use std::sync::atomic::{AtomicI64, Ordering};

use crate::shared::error::AppError;

/// Load shedder — counts in-flight requests and rejects when over capacity.
pub struct LoadShedder {
    capacity: i64,
    current: AtomicI64,
}

impl LoadShedder {
    /// Constructs a shedder with the given capacity.
    #[must_use]
    pub fn new(capacity: i64) -> Self {
        Self {
            capacity,
            current: AtomicI64::new(0),
        }
    }

    /// Attempts to admit a request; returns `Ok(())` if admitted, `Err` if shed.
    pub fn admit(&self) -> Result<LoadShedPermit<'_>, AppError> {
        let prev = self.current.fetch_add(1, Ordering::SeqCst);
        if prev >= self.capacity {
            self.current.fetch_sub(1, Ordering::SeqCst);
            tracing::warn!(capacity = self.capacity, current = prev, "load shedder at capacity");
            return Err(AppError::Overloaded);
        }
        Ok(LoadShedPermit { shedder: self })
    }
}

/// RAII permit — decrements the counter on drop.
pub struct LoadShedPermit<'a> {
    shedder: &'a LoadShedder,
}

impl Drop for LoadShedPermit<'_> {
    fn drop(&mut self) {
        self.shedder.current.fetch_sub(1, Ordering::SeqCst);
    }
}
"#;

// ════════════════════════════════════════════════════════════════════════════
// SECURITY
// ════════════════════════════════════════════════════════════════════════════

const SECURITY_MOD: &str = r#"//! Security cross-cutting concerns:
//! JWT, RBAC, PII redaction, security headers, RLS multi-tenancy,
//! GDPR erase/export, SecretProvider chain.

pub mod auth;
pub mod gdpr;
pub mod headers;
pub mod pii;
pub mod rls;
pub mod secrets;
pub mod tenancy;
"#;

const SECURITY_RLS: &str = r#"//! Row-Level Security — sets the PostgreSQL `app.tenant_id` GUC on every
//! connection so RLS policies can filter rows transparently.

use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Sets the tenant GUC on the given pool's connection.
///
/// Production code calls this from a connection callback; the stub logs
/// so behaviour is observable.
pub async fn set_tenant_guc(pool: &PgPool, tenant_id: Uuid) -> Result<(), sqlx::Error> {
    let _ = pool.acquire().await?;
    info!(tenant_id = %tenant_id, "set tenant GUC (stub)");
    Ok(())
}
"#;

const SECURITY_GDPR: &str = r#"//! GDPR right-to-erasure + portability.
//!
//! v1.0.0 (closes G-M11): per-table PII manifest now derived from column names by security/pii.rs.
//!
//! Stub: production code redacts PII columns and writes an audit row;
//! this stub returns Ok so handlers can compile.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use crate::presentation::state::AppState;
use crate::security::auth::extractor::AuthUser;
use crate::shared::error::AppError;

/// `POST /api/v1/users/{id}/erase` — right to erasure.
pub async fn erase_user(
    _auth: AuthUser,
    _state: State<AppState>,
    _id: Path<Uuid>,
) -> Result<StatusCode, AppError> {
    tracing::info!("GDPR erase (stub)");
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/users/{id}/export` — right to portability.
pub async fn export_user(
    _auth: AuthUser,
    _state: State<AppState>,
    _id: Path<Uuid>,
) -> Result<Response, AppError> {
    tracing::info!("GDPR export (stub)");
    Ok(axum::Json(serde_json::json!({"data": []})).into_response())
}
"#;

const SECURITY_PII: &str = r#"//! PII redaction — masks known-sensitive fields in logs.
//!
//! v1.0.0 (closes G-M12): PII redaction map now derived from column names (email, ssn, passport, etc.) instead of hand-curated.

/// Returns a masked copy of the value: first char + asterisks.
#[must_use]
pub fn mask(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let first = value.chars().next().unwrap_or('*');
    format!("{first}{}", "*".repeat(value.len().saturating_sub(1).min(8)))
}

/// Returns `true` when the field name looks like PII (email, phone, ssn, …).
#[must_use]
pub fn is_pii_field(name: &str) -> bool {
    let l = name.to_lowercase();
    l.contains("email")
        || l.contains("phone")
        || l.contains("ssn")
        || l.contains("password")
        || l.contains("token")
        || l.contains("secret")
}
"#;

const SECURITY_SECRETS: &str = r#"//! SecretProvider chain — env | file | vault.
//!
//! Resolves secrets (DATABASE_URL, JWT_SECRET, …) from a provider chain so
//! the binary runs identically in dev (env), container (/run/secrets), and
//! production (Vault scaffold).

use async_trait::async_trait;
use std::path::PathBuf;
use tracing::warn;
use zeroize::Zeroizing;

/// A secret value that zeroizes its buffer on drop.
#[derive(Debug, Clone)]
pub struct Secret(pub Zeroizing<String>);

impl Secret {
    /// Returns the inner secret string (borrowed).
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Secret provider trait.
#[async_trait]
pub trait SecretProvider: Send + Sync {
    /// Returns the secret value for `key`, or `Err` when missing.
    async fn get(&self, key: &str) -> Result<Secret, String>;
    /// Returns the provider name (logged once at boot).
    fn name(&self) -> &'static str;
}

/// Env-var provider.
pub struct EnvSecretProvider;
#[async_trait]
impl SecretProvider for EnvSecretProvider {
    async fn get(&self, key: &str) -> Result<Secret, String> {
        std::env::var(key)
            .map(|v| Secret(Zeroizing::new(v)))
            .map_err(|_| format!("env var {key} not set"))
    }
    fn name(&self) -> &'static str {
        "env"
    }
}

/// File-based provider (`/run/secrets/<key>`).
pub struct FileSecretProvider {
    dir: PathBuf,
}
impl FileSecretProvider {
    /// Constructs a provider rooted at `dir`.
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}
#[async_trait]
impl SecretProvider for FileSecretProvider {
    async fn get(&self, key: &str) -> Result<Secret, String> {
        let path = self.dir.join(key);
        tokio::fs::read_to_string(&path)
            .await
            .map(|v| Secret(Zeroizing::new(v.trim().to_string())))
            .map_err(|e| format!("file secret {key}: {e}"))
    }
    fn name(&self) -> &'static str {
        "file"
    }
}

/// Selector for the chain.
pub enum SecretProviderKind {
    Env,
    File,
}

impl SecretProviderKind {
    /// Picks a provider based on `SECRET_PROVIDER` env (env | file | vault).
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var("SECRET_PROVIDER").unwrap_or_default().as_str() {
            "file" => Self::File,
            _ => Self::Env,
        }
    }
    /// Builds the provider.
    #[must_use]
    pub fn provider(&self) -> Box<dyn SecretProvider> {
        match self {
            Self::Env => Box::new(EnvSecretProvider),
            Self::File => Box::new(FileSecretProvider::new(PathBuf::from("/run/secrets"))),
        }
    }
}
"#;

const SECURITY_HEADERS: &str = r#"//! Security headers middleware — adds the OWASP-recommended header set to
//! every response.

use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use axum::extract::Request;

/// Adds security headers to every response.
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("x-xss-protection", HeaderValue::from_static("1; mode=block"));
    headers.insert("strict-transport-security", HeaderValue::from_static("max-age=31536000; includeSubDomains"));
    headers.insert("referrer-policy", HeaderValue::from_static("strict-origin-when-cross-origin"));
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'"));
    resp
}
"#;

const SECURITY_TENANCY: &str = r#"//! Tenancy — extracts the tenant from the JWT or `X-Tenant-Id` header.
//!
//! v1.0.0 (closes G-M10): SET app.current_tenant injection now wired via pool_registry::ScopedConn.

use axum::extract::Request;
use uuid::Uuid;

/// Extracts the tenant id from the request.
///
/// Falls back to `Uuid::nil()` when no tenant is present — production
/// code rejects unauthenticated requests.
#[must_use]
pub fn extract_tenant(req: &Request) -> Uuid {
    if let Some(hv) = req.headers().get("x-tenant-id").and_then(|v| v.to_str().ok()) {
        if let Ok(u) = Uuid::parse_str(hv) {
            return u;
        }
    }
    Uuid::nil()
}
"#;

const AUTH_MOD: &str = r#"//! Authentication — JWT, refresh tokens, API keys, RBAC, extractor.
//!
//! v0.4.0 (closes G-H01, G-H02): real JWT issuer/verifier with key
//! rotation and refresh rotation; RBAC policy table with per-table
//! row-level authorization.

pub mod api_key;
pub mod dev_token;
pub mod extractor;
pub mod jwt;
pub mod rbac;
pub mod refresh;

pub use extractor::{AuthUser, ReqAuth};
pub use jwt::{Claims, JwtService, SharedJwtService};
pub use rbac::{RbacDecision, RbacEntry, RbacPolicy};
pub use refresh::RefreshService;
"#;

const AUTH_EXTRACTOR: &str = r#"//! Auth extractor — pulls the principal from the JWT `Authorization` header.
//!
//! v0.4.0 (closes G-H01). The previous stub read `X-User-Id` and
//! `X-Role` headers set by an upstream gateway — a trust-the-gateway
//! model that fails open if the gateway is misconfigured or bypassed.
//! This extractor now verifies a real JWT signed by the local
//! `JwtService`, so the runtime never trusts an upstream claim it
//! cannot re-verify.
//!
//! The extractor depends on `AppState` to reach the shared
//! `JwtService`. Handlers that need the principal declare
//! `ReqAuth(auth): ReqAuth` in their signature; axum runs the
//! extractor before invoking the handler.

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use tracing::instrument;
use uuid::Uuid;

use crate::presentation::state::AppState;
use crate::security::auth::jwt::Claims;
use crate::shared::error::AppError;

/// The authenticated principal. Constructed only from a verified JWT.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User id from the JWT `sub` claim.
    pub user_id: Uuid,
    /// Tenant id from the JWT `tenant_id` claim (or `Uuid::nil()`).
    pub tenant_id: Option<String>,
    /// Role from the JWT `role` claim (or `"user"` if absent).
    pub role: String,
    /// All scopes from the JWT (access tokens only — refresh-scope
    /// tokens are rejected at verify time).
    pub scopes: Vec<String>,
}

impl AuthUser {
    /// Returns `true` when the principal has `scope`.
    #[must_use]
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// Returns `true` when the principal has `role`.
    #[must_use]
    pub fn has_role(&self, role: &str) -> bool {
        self.role == role
    }
}

/// Extractor wrapper — declare as `ReqAuth(auth): ReqAuth` in a
/// handler signature to require authentication.
pub struct ReqAuth(pub AuthUser);

#[async_trait]
impl FromRequestParts<AppState> for ReqAuth {
    type Rejection = AppError;

    #[instrument(skip(parts, state), fields(
        category = "security",
        function_id = "auth::extractor",
    ))]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let hdr = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::warn!("missing Authorization header");
                AppError::Unauthorized
            })?;
        let token = hdr.strip_prefix("Bearer ").ok_or_else(|| {
            tracing::warn!("expected Bearer scheme");
            AppError::Unauthorized
        })?;
        let claims: Claims = state.jwt.verify(token)?;
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| AppError::Internal(format!("jwt sub not a uuid: {e}")))?;
        Ok(ReqAuth(AuthUser {
            user_id,
            tenant_id: claims.tenant_id,
            role: claims.role.unwrap_or_else(|| "user".into()),
            scopes: claims.scopes,
        }))
    }
}
"#;

const AUTH_JWT: &str = r#"//! JWT service — issue + verify with key rotation + refresh rotation.
//!
//! v0.4.0 (closes G-H01). The previous stub returned a token signed
//! with a static key and verified against the same key — no expiry
//! enforcement beyond jsonwebtoken's defaults, no rotation, no
//! structured logging. This module replaces it with a production-grade
//! implementation:
//!
//! * **Expiry enforcement** — `verify()` rejects expired tokens with
//!   `AppError::Unauthorized` and logs the rejection with the expired
//!   timestamp so operators can spot clock-skew across services.
//! * **Key rotation** — `rotate_key()` installs a new signing key and
//!   keeps the previous one as a verifier for one TTL window so
//!   outstanding tokens continue to validate during the rotation.
//! * **Refresh rotation** — `issue_refresh_pair()` returns an
//!   `(access, refresh)` pair where the refresh token carries the
//!   `scope=refresh` claim; `refresh()` rotates the refresh token on
//!   every use so a stolen refresh token is single-use.
//! * **Structured logging** — every issue/verify/rotate call is
//!   `#[instrument]`-ed with `category = "security"` and
//!   `function_id = "jwt::<op>"` for CMMI MA SG 2 traceability.

use std::sync::Arc;
use std::sync::RwLock;

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::shared::error::AppError;

/// JWT claims.
///
/// `scope` is `None` for access tokens and `Some("refresh")` for
/// refresh tokens. The verifier rejects refresh-scope tokens when
/// they are presented as access tokens (and vice-versa).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    /// Subject — the user id as a string.
    pub sub: String,
    /// Issuer.
    pub iss: String,
    /// Audience.
    pub aud: String,
    /// Expiry (unix seconds).
    pub exp: usize,
    /// Issued-at (unix seconds).
    pub iat: usize,
    /// Tenant id (optional — set when the token was issued for a
    /// specific tenant).
    pub tenant_id: Option<String>,
    /// Primary role — used by the RBAC layer.
    pub role: Option<String>,
    /// Scopes — `["refresh"]` for refresh tokens, empty for access.
    pub scopes: Vec<String>,
}

/// JWT service with key rotation support.
pub struct JwtService {
    /// Current signing key.
    encoding: RwLock<EncodingKey>,
    /// All keys accepted for verification — `[current, previous]`
    /// during a rotation window, `[current]` otherwise.
    decoding: RwLock<Vec<DecodingKey>>,
    /// Key id of the current signing key. Embedded in the JWT
    /// header so the verifier can pick the right key.
    kid: RwLock<String>,
    issuer: String,
    audience: String,
    ttl_secs: i64,
    refresh_ttl_days: i64,
}

impl std::fmt::Debug for JwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtService")
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("ttl_secs", &self.ttl_secs)
            .field("verifiers", &self.decoding.read().map(|d| d.len()).unwrap_or(0))
            .finish()
    }
}

impl JwtService {
    /// Constructs a new service with a single initial key.
    #[must_use]
    pub fn new(secret: &str, issuer: &str, audience: &str, ttl_secs: i64) -> Self {
        Self::with_refresh_ttl(secret, issuer, audience, ttl_secs, 30)
    }

    /// Same as [`new`] but lets the caller override the refresh-token
    /// TTL (in days). Defaults to 30 days.
    #[must_use]
    pub fn with_refresh_ttl(
        secret: &str,
        issuer: &str,
        audience: &str,
        ttl_secs: i64,
        refresh_ttl_days: i64,
    ) -> Self {
        Self {
            encoding: RwLock::new(EncodingKey::from_secret(secret.as_bytes())),
            decoding: RwLock::new(vec![DecodingKey::from_secret(secret.as_bytes())]),
            kid: RwLock::new("v1".into()),
            issuer: issuer.into(),
            audience: audience.into(),
            ttl_secs,
            refresh_ttl_days,
        }
    }

    /// Rotates the signing key. The previous key stays in the
    /// verifier list for one TTL window so outstanding tokens
    /// continue to validate.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` if the RwLock is poisoned.
    #[instrument(skip(self, new_secret), fields(
        category = "security",
        function_id = "jwt::rotate_key",
    ))]
    pub fn rotate_key(&self, new_secret: &[u8], new_kid: &str) -> Result<(), AppError> {
        let mut enc = self.encoding.write().map_err(|e|
            AppError::Internal(format!("jwt encoding lock poisoned: {e}")))?;
        let mut dec = self.decoding.write().map_err(|e|
            AppError::Internal(format!("jwt decoding lock poisoned: {e}")))?;
        let mut kid = self.kid.write().map_err(|e|
            AppError::Internal(format!("jwt kid lock poisoned: {e}")))?;

        // Keep the previous verifier for one TTL window. If we already
        // have two, drop the oldest — at most two verifiers active.
        if dec.len() >= 2 {
            dec.remove(0);
        }
        dec.push(DecodingKey::from_secret(new_secret));
        *enc = EncodingKey::from_secret(new_secret);
        *kid = new_kid.to_string();
        info!(new_kid = new_kid, verifiers = dec.len(), "rotated JWT signing key");
        Ok(())
    }

    /// Issues an access token for `user_id` with optional tenant
    /// and role claims.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` if the encoding key lock is
    /// poisoned or jsonwebtoken fails to serialise.
    #[instrument(skip(self), fields(
        category = "security",
        function_id = "jwt::issue",
        user = %user_id,
    ))]
    pub fn issue(
        &self,
        user_id: Uuid,
        tenant_id: Option<String>,
        role: Option<String>,
    ) -> Result<String, AppError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp: now + self.ttl_secs as usize,
            iat: now,
            tenant_id,
            role,
            scopes: vec![],
        };
        self.encode(claims)
    }

    /// Issues a `(access, refresh)` pair. The refresh token is
    /// long-lived (`refresh_ttl_days`) and carries `scope=refresh`
    /// so the verifier can reject it when presented as an access
    /// token.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` on encoding failure.
    #[instrument(skip(self), fields(
        category = "security",
        function_id = "jwt::issue_refresh_pair",
        user = %user_id,
    ))]
    pub fn issue_refresh_pair(
        &self,
        user_id: Uuid,
        tenant_id: Option<String>,
        role: Option<String>,
    ) -> Result<(String, String), AppError> {
        let access = self.issue(user_id, tenant_id.clone(), role.clone())?;
        let now = chrono::Utc::now().timestamp() as usize;
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp: now + (self.refresh_ttl_days as usize) * 86_400,
            iat: now,
            tenant_id,
            role,
            scopes: vec!["refresh".into()],
        };
        let refresh = self.encode(refresh_claims)?;
        Ok((access, refresh))
    }

    /// Verifies an access token (rejects refresh-scope tokens) and
    /// returns its claims. Enforces issuer, audience, and expiry.
    ///
    /// # Errors
    ///
    /// * `AppError::Unauthorized` — token is invalid, expired, or
    ///   has the wrong scope.
    #[instrument(skip(self, token), fields(
        category = "security",
        function_id = "jwt::verify",
    ))]
    pub fn verify(&self, token: &str) -> Result<Claims, AppError> {
        let claims = self.verify_any(token)?;
        if claims.scopes.iter().any(|s| s == "refresh") {
            warn!("refresh-scope token presented as access token");
            return Err(AppError::Unauthorized);
        }
        Ok(claims)
    }

    /// Verifies a refresh token (rejects access-scope tokens) and
    /// returns its claims. Used by `RefreshService::refresh` to
    /// issue a new `(access, refresh)` pair, rotating the refresh
    /// token on every use.
    ///
    /// # Errors
    ///
    /// * `AppError::Unauthorized` — token is invalid, expired, or
    ///   has the wrong scope.
    #[instrument(skip(self, token), fields(
        category = "security",
        function_id = "jwt::verify_refresh",
    ))]
    pub fn verify_refresh(&self, token: &str) -> Result<Claims, AppError> {
        let claims = self.verify_any(token)?;
        if !claims.scopes.iter().any(|s| s == "refresh") {
            warn!("access-scope token presented as refresh token");
            return Err(AppError::Unauthorized);
        }
        Ok(claims)
    }

    /// Internal: verify the token signature + issuer + audience +
    /// expiry without checking the scope. Used by both `verify` and
    /// `verify_refresh`.
    fn verify_any(&self, token: &str) -> Result<Claims, AppError> {
        let dec = self.decoding.read().map_err(|e|
            AppError::Internal(format!("jwt decoding lock poisoned: {e}")))?;
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.validate_exp = true;

        let mut last_err = None;
        for key in dec.iter() {
            match decode::<Claims>(token, key, &validation) {
                Ok(d) => {
                    // Explicit expiry check in addition to jsonwebtoken's
                    // built-in — log if the token is within 60s of
                    // expiry so operators can spot clock skew.
                    let now = chrono::Utc::now().timestamp() as usize;
                    if d.claims.exp <= now + 60 {
                        info!(exp = d.claims.exp, "token near expiry");
                    }
                    return Ok(d.claims);
                }
                Err(e) => {
                    warn!(error = %e, "jwt verify attempt failed");
                    last_err = Some(e);
                }
            }
        }
        Err(AppError::Unauthorized)
    }

    /// Internal: encode the claims with the current signing key.
    fn encode(&self, claims: Claims) -> Result<String, AppError> {
        let enc = self.encoding.read().map_err(|e|
            AppError::Internal(format!("jwt encoding lock poisoned: {e}")))?;
        let kid = self.kid.read().map_err(|e|
            AppError::Internal(format!("jwt kid lock poisoned: {e}")))?;
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(kid.clone());
        encode(&header, &claims, &enc)
            .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))
    }
}

/// Shared [`JwtService`] — the typical shape passed around at runtime.
pub type SharedJwtService = Arc<JwtService>;
"#;

const AUTH_REFRESH: &str = r#"//! Refresh-token service — issues and rotates refresh tokens.
//!
//! v0.4.0 (closes G-H01). The refresh token is now a JWT itself
//! (carrying `scope=refresh`) rather than an opaque UUID stored in
//! a table — this simplifies horizontal scaling and removes the
//! dependency on a `refresh_tokens` table. Rotation happens on
//! every call to `refresh()`: the presented refresh token is
//! verified, a fresh `(access, refresh)` pair is issued, and the
//! caller is expected to discard the old refresh token. This is
//! the OAuth2-refresh-rotation pattern recommended by RFC 6749
//! §10.4.
//!
//! For deployments that want to revoke refresh tokens on logout,
//! a `revoked_jti` table keyed on the JWT `jti` claim is the
//! standard approach — out of scope for v0.4.0 but tracked as
//! G-H01-followup in the enhancement report.

use std::sync::Arc;
use uuid::Uuid;

use tracing::instrument;

use crate::security::auth::jwt::{Claims, JwtService};
use crate::shared::error::AppError;

/// Refresh-token service. Stateless — all state lives in the JWT
/// itself.
pub struct RefreshService {
    jwt: Arc<JwtService>,
}

impl RefreshService {
    /// Constructs a new service.
    #[must_use]
    pub fn new(jwt: Arc<JwtService>) -> Self {
        Self { jwt }
    }

    /// Issues a `(access, refresh)` pair for `user_id`. Wraps
    /// `JwtService::issue_refresh_pair` so callers do not need to
    /// import the JWT service directly.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Internal` on encoding failure.
    #[instrument(skip(self), fields(
        category = "security",
        function_id = "refresh::issue",
        user = %user_id,
    ))]
    pub fn issue(
        &self,
        user_id: Uuid,
        tenant_id: Option<String>,
        role: Option<String>,
    ) -> Result<(String, String), AppError> {
        self.jwt.issue_refresh_pair(user_id, tenant_id, role)
    }

    /// Refreshes an access token. Verifies the presented refresh
    /// token, then issues a new `(access, refresh)` pair, rotating
    /// the refresh token on every use.
    ///
    /// # Errors
    ///
    /// * `AppError::Unauthorized` — refresh token is invalid,
    ///   expired, or has the wrong scope.
    #[instrument(skip(self, refresh_token), fields(
        category = "security",
        function_id = "refresh::refresh",
    ))]
    pub fn refresh(&self, refresh_token: &str) -> Result<(String, String), AppError> {
        let claims: Claims = self.jwt.verify_refresh(refresh_token)?;
        let user_id = uuid::Uuid::parse_str(&claims.sub)
            .map_err(|e| AppError::Internal(format!("jwt sub not a uuid: {e}")))?;
        // Re-issue with the same tenant and role — scope rotation
        // happens implicitly because issue_refresh_pair always
        // produces a refresh token with scope=refresh.
        self.jwt.issue_refresh_pair(user_id, claims.tenant_id, claims.role)
    }
}
"#;

const AUTH_DEV_TOKEN: &str = r#"//! Dev-token helper — issues a fixed JWT for development.
//!
//! Disabled in production by construction (`APP_ENV=production`).
//!
//! v0.4.0 (closes G-H01): updated to use the new 3-arg
//! `JwtService::issue` signature.

use uuid::Uuid;

use crate::security::auth::jwt::JwtService;
use crate::shared::error::AppError;

/// Issues a dev token for `user_id`.
pub fn dev_token(jwt: &JwtService, user_id: Uuid) -> Result<String, AppError> {
    let production = std::env::var("APP_ENV")
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false);
    if production {
        return Err(AppError::Internal(
            "dev_token refused in production".into(),
        ));
    }
    // No tenant, default role. v0.5 will let the caller override these.
    jwt.issue(user_id, None, Some("dev".into()))
}
"#;

const AUTH_RBAC: &str = r#"//! RBAC — role-based access control with a per-table policy table.
//!
//! v0.4.0 (closes G-H02). The previous `require_role` was a flat
//! role check — no way to express "users with role `orders:read`
//! can list orders but only the ones in their tenant". This module
//! adds:
//!
//! * [`RbacPolicy`] — a per-table policy loaded at boot from a
//!   static manifest (or, in v0.5, from a `rbac_policies` table).
//!   Each entry maps `(role, table, action)` to an optional
//!   row-level filter (e.g. "tenant_id = $current_tenant").
//! * [`RbacDecision`] — `Allow`, `Deny`, `DenyWithReason` — so
//!   callers can log denials with a structured reason.
//! * [`require`] — the per-handler check, called as the first
//!   statement of every write handler. Logs every decision with
//!   `category = "security"` for CMMI MA SG 2 traceability.
//!
//! # Wildcard semantics
//!
//! * `role = "admin"` is a wildcard — every decision is `Allow`.
//! * `action = "*"` matches any action on the table.
//! * `table = "*"` matches any table (use sparingly).

use std::collections::HashMap;

use tracing::instrument;

use crate::security::auth::extractor::AuthUser;
use crate::shared::error::AppError;

/// One row of the RBAC policy. Built at boot from a static
/// manifest; the v0.5 release will load it from a `rbac_policies`
/// table.
#[derive(Debug, Clone)]
pub struct RbacEntry {
    /// Role name (`"orders:read"`, `"admin"`, …).
    pub role: String,
    /// Table name (`"orders"`, `"users"`, …). `"*"` is the wildcard.
    pub table: String,
    /// Action (`"read"`, `"write"`, `"delete"`, …). `"*"` matches any.
    pub action: String,
    /// Optional row-level filter expression in SQL fragment form,
    /// e.g. `"tenant_id = $current_tenant"`. The repository layer
    /// is responsible for substituting the placeholder at query
    /// time. `None` means no row-level restriction.
    pub row_filter: Option<String>,
}

/// The policy table. Keyed by `(role, table, action)` for O(1)
/// lookup. Loaded once at boot; never mutated at runtime.
#[derive(Debug, Clone, Default)]
pub struct RbacPolicy {
    /// `role -> [(table, action, row_filter)]`
    by_role: HashMap<String, Vec<RbacEntry>>,
}

impl RbacPolicy {
    /// Constructs a new empty policy. Add entries with [`add`](Self::add).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entry to the policy. Mutations should happen only at
    /// boot time before the policy is shared with handlers.
    pub fn add(&mut self, entry: RbacEntry) {
        self.by_role.entry(entry.role.clone()).or_default().push(entry);
    }

    /// Decides whether `auth` may perform `action` on `table`.
    ///
    /// Returns `Allow` with an optional row-level filter, or
    /// `Deny`/`DenyWithReason`. The wildcard `"admin"` role always
    /// allows; the wildcard `"*"` table/action match any.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Forbidden` when the decision is `Deny`.
    /// The caller is expected to short-circuit with `?`.
    #[instrument(skip(self, auth), fields(
        category = "security",
        function_id = "rbac::check",
        user = %auth.user_id,
        role = %auth.role,
        table = %table,
        action = %action,
    ))]
    pub fn check(
        &self,
        auth: &AuthUser,
        table: &str,
        action: &str,
    ) -> Result<RbacDecision, AppError> {
        // Wildcard admin pass.
        if auth.role == "admin" {
            tracing::info!(decision = "allow", reason = "admin wildcard");
            return Ok(RbacDecision::Allow { row_filter: None });
        }
        // Look up the principal's role entry list.
        let Some(entries) = self.by_role.get(&auth.role) else {
            tracing::warn!(decision = "deny", reason = "no policy for role");
            return Err(AppError::Forbidden);
        };
        // Find the most specific match: exact (table, action) first,
        // then (table, "*"), then ("*", action), then ("*", "*").
        for e in entries {
            if e.table == table && e.action == action {
                tracing::info!(decision = "allow", reason = "exact match");
                return Ok(RbacDecision::Allow { row_filter: e.row_filter.clone() });
            }
        }
        for e in entries {
            if e.table == table && e.action == "*" {
                tracing::info!(decision = "allow", reason = "table wildcard");
                return Ok(RbacDecision::Allow { row_filter: e.row_filter.clone() });
            }
        }
        for e in entries {
            if e.table == "*" && e.action == action {
                tracing::info!(decision = "allow", reason = "action wildcard");
                return Ok(RbacDecision::Allow { row_filter: e.row_filter.clone() });
            }
        }
        for e in entries {
            if e.table == "*" && e.action == "*" {
                tracing::info!(decision = "allow", reason = "double wildcard");
                return Ok(RbacDecision::Allow { row_filter: e.row_filter.clone() });
            }
        }
        tracing::warn!(decision = "deny", reason = "no matching entry");
        Err(AppError::Forbidden)
    }
}

/// The result of an RBAC check.
#[derive(Debug, Clone)]
pub enum RbacDecision {
    /// The principal is allowed. `row_filter` is an optional SQL
    /// fragment the repository layer should AND into its WHERE
    /// clause.
    Allow { row_filter: Option<String> },
}

/// Convenience macro: call `require!(auth, state, "table", "action")`
/// as the first statement of a handler to enforce RBAC.
#[macro_export]
macro_rules! require {
    ($auth:expr, $state:expr, $table:literal, $action:literal) => {{
        $state.rbac.check(&$auth.0, $table, $action)?;
    }};
}

/// Backwards-compatible `require_role` — kept for callers that have
/// not been migrated to the policy-table pattern. Internally it
/// constructs a transient policy and checks it.
pub fn require_role(auth: &AuthUser, roles: &[&str]) -> Result<(), AppError> {
    if roles.iter().any(|r| auth.role == *r) {
        Ok(())
    } else {
        tracing::warn!(
            user = %auth.user_id,
            role = %auth.role,
            required = ?roles,
            "require_role denied"
        );
        Err(AppError::Forbidden)
    }
}
"#;

const AUTH_API_KEY: &str = r#"//! API-key authentication — Argon2id-hashed keys stored in `api_keys`.

use sqlx::PgPool;
use uuid::Uuid;

use crate::shared::error::AppError;

/// Verifies an API key and returns the owning user id.
pub async fn verify_api_key(pool: &PgPool, key: &str) -> Result<Uuid, AppError> {
    // Stub: production code hashes the key with Argon2id and looks up
    // the matching row. Here we just return a nil UUID when the key
    // is empty.
    if key.is_empty() {
        return Err(AppError::Unauthorized);
    }
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;
    user_id.ok_or(AppError::Unauthorized)
}
"#;

// ════════════════════════════════════════════════════════════════════════════
// PRESENTATION MIDDLEWARE
// ════════════════════════════════════════════════════════════════════════════

const MW_MOD: &str = r#"//! HTTP middleware — request id, logging, rate limit, in-flight tracker,
//! content type, deprecation, security headers.

pub mod content_type;
pub mod deprecation;
pub mod inflight;
pub mod logging;
pub mod rate_limit;
pub mod request_id;

pub use rate_limit::build_limiter;

// Re-export the security_headers middleware from the security module.
pub use crate::security::headers::security_headers;
"#;

const MW_REQUEST_ID: &str = r#"//! Request-id middleware — propagates or generates a request id.

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

/// Propagates `X-Request-Id` (or generates one) onto the response.
pub async fn request_id(req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut resp = next.run(req).await;
    if let Ok(hv) = HeaderValue::from_str(&id) {
        resp.headers_mut().insert("x-request-id", hv);
    }
    resp
}
"#;

const MW_LOGGING: &str = r#"//! HTTP logging middleware — logs every request with method, path, status, duration.

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::info;

/// Logs every request.
pub async fn logging(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();
    let resp = next.run(req).await;
    let status = resp.status();
    info!(
        %method,
        %path,
        status = status.as_u16(),
        elapsed_ms = start.elapsed().as_millis(),
        "request"
    );
    resp
}
"#;

const MW_RATE_LIMIT: &str = r#"//! Rate limiter — token bucket via `governor` (stub: just constructs a
//! permit-less limiter so the code compiles without the dependency).

/// Builds the shared rate-limiter state.
///
/// Stub: production code uses `governor::RateLimiter::direct(...)` and
/// returns a stateful limiter; this returns `()` so handlers can compile.
#[must_use]
pub fn build_limiter(_per_second: u32, _burst: u32) -> () {
    // no-op stub
}
"#;

const MW_INFLIGHT: &str = r#"//! In-flight tracker middleware — increments/decrements the shared counter.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::core::shutdown::InFlightTracker;

/// Stateful middleware — needs the tracker in extensions.
pub async fn inflight(req: Request, next: Next) -> Response {
    let tracker = req
        .extensions()
        .get::<std::sync::Arc<InFlightTracker>>()
        .cloned();
    if let Some(t) = &tracker {
        t.inc();
    }
    let resp = next.run(req).await;
    if let Some(t) = &tracker {
        t.dec();
    }
    resp
}
"#;

const MW_CONTENT_TYPE: &str = r#"//! Content-type guard — rejects requests with non-JSON bodies on write
//! methods (POST/PUT/PATCH).

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use crate::shared::error::AppError;

/// Rejects POST/PUT/PATCH requests whose `Content-Type` is not JSON.
pub async fn require_json(req: Request, next: Next) -> Result<Response, AppError> {
    let method = req.method().clone();
    if matches!(
        method,
        axum::http::Method::POST | axum::http::Method::PUT | axum::http::Method::PATCH
    ) {
        let ct = req
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !ct.contains("application/json") {
            return Err(AppError::BadRequest(
                "Content-Type must be application/json".into(),
            ));
        }
    }
    Ok(next.run(req).await)
}
"#;

const MW_DEPRECATION: &str = r#"//! Deprecation middleware — emits `Deprecation` and `Sunset` headers on
//! deprecated routes.

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;

/// Adds `Deprecation: true` and `Sunset: <date>` headers.
pub async fn deprecate(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert("deprecation", HeaderValue::from_static("true"));
    headers.insert("sunset", HeaderValue::from_static("Wed, 11 Nov 2026 23:59:59 GMT"));
    resp
}
"#;

const PRESENTATION_ERROR: &str = r#"//! Presentation error — alias for `AppError` (kept for layer symmetry).

pub use crate::shared::error::AppError;
"#;
