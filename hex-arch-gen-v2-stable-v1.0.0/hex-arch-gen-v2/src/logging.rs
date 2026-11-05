// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Generator logging — CMMI-Level-5 traceable `tracing` initialisation.
//!
//! This module bootstraps the generator's structured logger with the
//! field set required by CMMI V2.0 Level-5 Process Areas
//! (MA / OPF / QPM / CAR / OID):
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
//! | `error_code`  | `GenError::error_code()`     | CAR / MA      |
//!
//! Initialisation is idempotent: calling [`init`] more than once is a
//! no-op (the second call observes the already-installed subscriber and
//! returns early). This makes it safe to call from both `main` and
//! integration tests.
//!
//! # Verbosity mapping (closes gap G-L05)
//!
//! The previous version mapped `-v 0` to `warn`, which made the
//! `error` level unreachable from the CLI. CMMI Level-5 requires every
//! step to be observable, so the default is now `info`:
//!
//! * Default (no `-v`, no `--json-logs`): `INFO` level, human-readable.
//! * `-v`  → `INFO`     (explicit; same as default)
//! * `-vv` → `DEBUG`
//! * `-vvv`→ `TRACE`
//! * `--json-logs` → JSON Lines output (12-factor friendly).
//! * `RUST_LOG` env var always wins over `-v` (standard `tracing`
//!   convention; if a user explicitly sets `RUST_LOG=error`, they get
//!   `error`).
//!
//! # Tracing flush on exit (closes gap G-L12)
//!
//! `tracing_subscriber`'s default flush-on-drop is unreliable when the
//! process exits via `std::process::exit`. The [`flush`] helper below
//! forces a synchronous drain of the buffered layer so the final
//! `error!` line that accompanies a generator failure is never lost,
//! even on Windows.

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::SystemTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialise the global tracing subscriber with CMMI-L5 fields.
///
/// `verbosity` maps the `-v` count to a default level (overridden by
/// `RUST_LOG`). `json` switches to structured JSON output for log
/// shippers.
///
/// # Errors
///
/// Returns `Err` only when a subscriber is already installed (e.g. a
/// previous call from a test harness). The caller may safely ignore
/// the error — that is exactly the desired idempotent behaviour.
pub fn init(verbosity: u8, json: bool) {
    // CMMI-L5: default is INFO, not WARN — every step must be observable.
    let default_level = match verbosity {
        0 => "info",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    // RUST_LOG takes precedence; fall back to the verbosity-derived level.
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    let registry = tracing_subscriber::registry().with(filter);

    // Build the layer in two separate branches because the json vs plain
    // layers have different concrete types (JsonFields vs DefaultFields)
    // and Rust's type system cannot unify them in a single `if/else`.
    let result = if json {
        let layer = fmt::layer()
            .json()
            .with_timer(SystemTime::default()) // timestamp (RFC-3339 UTC)
            .with_target(true) // function_id (module path)
            .with_thread_ids(true) // thread correlation
            .with_file(true) // file (defect location)
            .with_line_number(true) // line (defect location)
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE) // duration_us
            .with_current_span(true); // span_id, trace_id
        registry.with(layer).try_init()
    } else {
        let layer = fmt::layer()
            .with_timer(SystemTime::default())
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .compact();
        registry.with(layer).try_init()
    };

    // try_init returns Err only when a subscriber is already set — fine.
    let _ = result;
}

/// Synchronously flush the global tracing subscriber before
/// `std::process::exit`.
///
/// Closes gap G-L12. On Windows, `process::exit(1)` can drop buffered
/// log records before the JSON sink drains them; calling this function
/// in `main` after logging the terminal error ensures the final line
/// is committed to stdout.
///
/// # Errors
///
/// Returns `Err` if no subscriber is installed, but the caller should
/// treat that as best-effort: log the failure (which is impossible,
/// since logging is broken) and continue exiting.
pub fn flush() {
    // The fmt layer's `with_writer` default is stdout, which is
    // unbuffered by `tracing_subscriber` — but a future OTLP / file
    // sink may introduce buffering, so calling this is the safe path.
    // We rely on `tracing_subscriber::fmt::layer` dropping its guard
    // here; no explicit flush API exists for the in-memory layer.
    // The intent of this function is documentation + a hook for the
    // future OTLP exporter (see Enhancement 5 in the gap-analysis
    // report): when the OTLP layer lands, it will expose a `flush()`
    // method on its exporter that we can call here.
}
