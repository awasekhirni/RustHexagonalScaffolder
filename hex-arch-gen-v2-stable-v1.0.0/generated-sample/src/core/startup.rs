//! Startup phase tracker — logs each boot phase for observability.

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
