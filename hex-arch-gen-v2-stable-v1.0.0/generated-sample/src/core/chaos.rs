//! Chaos engineering — never enabled unless `APP__CHAOS__ENABLED=true`.
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
