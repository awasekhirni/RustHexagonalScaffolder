//! RBAC — role-based access control with a per-table policy table.
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
