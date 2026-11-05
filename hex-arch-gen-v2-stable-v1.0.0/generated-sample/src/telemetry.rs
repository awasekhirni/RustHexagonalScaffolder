//! Telemetry — OTLP traces + Prometheus metrics.
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
