// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Error inventory v2 — machine-readable companion to `ERROR_INVENTORY.md`.
//!
//! Closes gap **G-H03**. The Markdown inventory is excellent for
//! humans but invisible to tooling. This module adds a sibling
//! `error_inventory.json` with an explicit serde schema, an
//! append-only writer API, and helpers to render the Markdown view
//! from the JSON so the two files can never drift.
//!
//! # CMMI mapping
//!
//! * **MA SG 1 + SG 2** — Specify measurement objectives and collect
//!   the data: every defect is a measurable unit with a stable
//!   `error_code` and `severity`.
//! * **CAR SG 1 + SG 2** — Determine root causes of defects and
//!   address them: the `root_cause`, `resolution`, and `lessons`
//!   fields capture the causal chain.
//! * **OPF SG 2** — Deploy the organizational process assets: the
//!   inventory *is* the process asset for defect management.
//!
//! # CLI integration
//!
//! `hex-gen errors {list,add,resolve,stats}` is wired in `main.rs`
//! and consumes this module. Every mutation is logged via `tracing`
//! with `category = "error_inventory"` so the audit trail of
//! inventory edits is itself observable (CMMI MA SG 2 "collect data").

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::instrument;

// ─── Types ────────────────────────────────────────────────────────────────

/// One inventory row. Mirrors the per-entry section of the existing
/// `ERROR_INVENTORY.md` so the Markdown view can be regenerated
/// deterministically from the JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    /// Stable identifier, e.g. `ERR-021`.
    pub id: String,
    /// Short human-readable title.
    pub title: String,
    /// Workflow state.
    pub status: Status,
    /// Generator module that owns the defect (e.g. `codegen::api`).
    pub component: String,
    /// Severity bucket — drives CAR prioritisation.
    pub severity: Severity,
    /// CMMI Process Areas touched, e.g. `["MA", "CAR"]`.
    pub cmmi_pa: Vec<String>,
    /// Cross-reference to the gap matrix (e.g. `["G-C04"]`).
    pub gap_ids: Vec<String>,
    /// ISO-8601 UTC timestamp of first observation.
    pub date: DateTime<Utc>,
    /// File:line or module path where the defect was hit.
    pub location: String,
    /// One of: `syntax | build | runtime | dependency | configuration`.
    pub error_type: String,
    /// Exact error message captured from `rustc` / `cargo` / runtime.
    pub error_message: String,
    /// Prose explanation of the underlying cause.
    pub root_cause: String,
    /// What was done (or will be done) to fix it.
    pub resolution: String,
    /// Optional forward-looking note for future iterations.
    pub lessons: Option<String>,
    /// IDs of related entries, e.g. `["ERR-005"]`.
    pub related: Vec<String>,
    /// Stable runtime identifier (e.g. `GEN-001`) from
    /// `crate::errors::GenError::error_code()`. Used by CI to
    /// aggregate defect rates by code without parsing messages.
    pub error_code: Option<String>,
}

/// Workflow state of an entry. Append-only means an entry never gets
/// deleted — `Resolved` or `Superseded` is the terminal state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Open,
    Resolved,
    Deferred,
    Superseded,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Status::Open => "Open",
            Status::Resolved => "Resolved",
            Status::Deferred => "Deferred",
            Status::Superseded => "Superseded",
        };
        f.write_str(s)
    }
}

/// Severity bucket. Drives CAR prioritisation and roadmap scheduling.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Severity::Critical => "Critical",
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
        };
        f.write_str(s)
    }
}

/// The whole inventory. Serialized as `error_inventory.json` next to
/// `ERROR_INVENTORY.md`. Schema version is bumped only on
/// backwards-incompatible field changes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    /// Bumped on backwards-incompatible schema changes.
    pub schema_version: u32,
    /// ISO-8601 UTC timestamp of the last mutation.
    pub generated_at: DateTime<Utc>,
    /// Generator version that produced / last touched the inventory.
    pub generator_version: String,
    /// Entries keyed by ID, sorted by `BTreeMap` for deterministic
    /// iteration (CMMI MA SG 2 "collect data" requires reproducible
    /// aggregation).
    pub entries: BTreeMap<String, ErrorEntry>,
}

// ─── Inventory API ────────────────────────────────────────────────────────

impl Inventory {
    /// Load the inventory from a JSON file. Returns an empty inventory
    /// (with the current generator version stamped in) if the file
    /// does not exist yet — the caller decides whether to write it
    /// back.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but is not valid JSON
    /// against the current schema.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self {
                schema_version: 1,
                generated_at: Utc::now(),
                generator_version: env!("CARGO_PKG_VERSION").into(),
                entries: BTreeMap::new(),
            });
        }
        let raw = std::fs::read_to_string(path)?;
        let inv: Self = serde_json::from_str(&raw)?;
        Ok(inv)
    }

    /// Append a new entry. Fails if the ID already exists — the
    /// inventory is append-only; mutations go through [`Self::resolve`].
    ///
    /// # Errors
    ///
    /// Returns an error if `entry.id` already exists in the inventory.
    #[instrument(skip(self, entry), fields(
        category = "error_inventory",
        function_id = "inventory::add",
        id = %entry.id,
        severity = %entry.severity,
    ))]
    pub fn add(&mut self, entry: ErrorEntry) -> anyhow::Result<()> {
        if self.entries.contains_key(&entry.id) {
            anyhow::bail!("error id {} already exists", entry.id);
        }
        self.generated_at = Utc::now();
        self.entries.insert(entry.id.clone(), entry);
        Ok(())
    }

    /// Mark an existing entry as resolved, optionally recording the
    /// lesson learned. Mutates `status`, `resolution`, and `lessons`.
    ///
    /// # Errors
    ///
    /// Returns an error if `id` is not present in the inventory.
    #[instrument(skip(self, resolution, lessons), fields(
        category = "error_inventory",
        function_id = "inventory::resolve",
        id = %id,
    ))]
    pub fn resolve(
        &mut self,
        id: &str,
        resolution: String,
        lessons: Option<String>,
    ) -> anyhow::Result<()> {
        let e = self
            .entries
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("unknown error id {id}"))?;
        e.status = Status::Resolved;
        e.resolution = resolution;
        if let Some(l) = lessons {
            e.lessons = Some(l);
        }
        self.generated_at = Utc::now();
        Ok(())
    }

    /// Compute the next sequential ID (`ERR-021` … `ERR-999`).
    ///
    /// Walks the existing entries and returns one greater than the
    /// max numeric suffix observed. Used by the CLI `add` subcommand
    /// when the user does not pass `--id`.
    #[must_use]
    pub fn next_id(&self) -> String {
        let n = self
            .entries
            .keys()
            .filter_map(|k| k.strip_prefix("ERR-").and_then(|s| s.parse::<u32>().ok()))
            .max()
            .unwrap_or(0)
            + 1;
        format!("ERR-{n:03}")
    }

    /// Write the inventory as pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Returns an error on filesystem failure.
    #[instrument(skip(self), fields(
        category = "error_inventory",
        function_id = "inventory::write_json",
        entries = self.entries.len(),
    ))]
    pub fn write_json(&self, path: &Path) -> anyhow::Result<()> {
        let s = serde_json::to_string_pretty(self)?;
        std::fs::write(path, s)?;
        Ok(())
    }

    /// Render the inventory as the canonical `ERROR_INVENTORY.md`
    /// view. The Markdown is generated deterministically from the
    /// JSON so the two files can never drift.
    ///
    /// # Errors
    ///
    /// Returns an error on filesystem failure.
    #[instrument(skip(self), fields(
        category = "error_inventory",
        function_id = "inventory::write_markdown",
        entries = self.entries.len(),
    ))]
    pub fn write_markdown(&self, path: &Path) -> anyhow::Result<()> {
        std::fs::write(path, self.render_markdown())?;
        Ok(())
    }

    /// Render the Markdown view to an in-memory string.
    ///
    /// Exposed so tests can assert on the rendered output without
    /// touching the filesystem.
    #[must_use]
    pub fn render_markdown(&self) -> String {
        let mut s = String::new();
        s.push_str("# ERROR_INVENTORY.md\n\n");
        s.push_str(&format!(
            "> Auto-generated from `error_inventory.json` at {}.\n",
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        s.push_str(&format!(
            "> Generator version: `{}`. Schema version: `{}`.\n\n",
            self.generator_version, self.schema_version
        ));
        s.push_str(
            "> Persistent, append-only log of every error encountered during the\n\
             > development of `hex-arch-gen-v2`. The machine-readable companion is\n\
             > `error_inventory.json`; this Markdown view is generated from it so\n\
             > the two files can never drift.\n\n",
        );

        // ── Summary table ───────────────────────────────────────────────
        s.push_str("## Summary Table\n\n");
        s.push_str("| ID | Title | Status | Severity | Component | Error Code |\n");
        s.push_str("|---|---|---|---|---|---|\n");
        for e in self.entries.values() {
            s.push_str(&format!(
                "| {} | {} | {} | {} | `{}` | {} |\n",
                e.id,
                e.title,
                e.status,
                e.severity,
                e.component,
                e.error_code.as_deref().unwrap_or("-"),
            ));
        }

        // ── Aggregate stats ─────────────────────────────────────────────
        s.push_str("\n## Aggregate Stats\n\n");
        let mut by_status: BTreeMap<String, u32> = BTreeMap::new();
        let mut by_severity: BTreeMap<String, u32> = BTreeMap::new();
        let mut by_pa: BTreeMap<String, u32> = BTreeMap::new();
        for e in self.entries.values() {
            *by_status.entry(e.status.to_string()).or_default() += 1;
            *by_severity.entry(e.severity.to_string()).or_default() += 1;
            for pa in &e.cmmi_pa {
                *by_pa.entry(pa.clone()).or_default() += 1;
            }
        }
        s.push_str("| Dimension | Bucket | Count |\n|---|---|---|\n");
        for (k, v) in &by_status {
            s.push_str(&format!("| status | {k} | {v} |\n"));
        }
        for (k, v) in &by_severity {
            s.push_str(&format!("| severity | {k} | {v} |\n"));
        }
        for (k, v) in &by_pa {
            s.push_str(&format!("| cmmi_pa | {k} | {v} |\n"));
        }

        // ── Detailed entries ────────────────────────────────────────────
        s.push_str("\n## Detailed Entries\n\n");
        for e in self.entries.values() {
            s.push_str(&format!("### {} — {}\n", e.id, e.title));
            s.push_str(&format!("- **Date:** {}\n", e.date.format("%Y-%m-%d")));
            s.push_str(&format!("- **Component:** `{}`\n", e.component));
            s.push_str(&format!("- **Severity:** {}\n", e.severity));
            s.push_str(&format!("- **Status:** {}\n", e.status));
            s.push_str(&format!(
                "- **CMMI PA:** {}\n",
                if e.cmmi_pa.is_empty() {
                    "-".into()
                } else {
                    e.cmmi_pa.join(", ")
                }
            ));
            s.push_str(&format!(
                "- **Gap IDs:** {}\n",
                if e.gap_ids.is_empty() {
                    "-".into()
                } else {
                    e.gap_ids.join(", ")
                }
            ));
            s.push_str(&format!(
                "- **Error Code:** {}\n",
                e.error_code.as_deref().unwrap_or("-")
            ));
            s.push_str(&format!("- **Location:** `{}`\n", e.location));
            s.push_str(&format!("- **Error Type:** {}\n", e.error_type));
            s.push_str(&format!("- **Error Message:** {}\n", e.error_message));
            s.push_str(&format!("- **Root Cause:** {}\n", e.root_cause));
            s.push_str(&format!("- **Resolution:** {}\n", e.resolution));
            if let Some(l) = &e.lessons {
                s.push_str(&format!("- **Lessons:** {}\n", l));
            }
            if !e.related.is_empty() {
                s.push_str(&format!("- **Related:** {}\n", e.related.join(", ")));
            }
            s.push('\n');
        }

        s
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(id: &str) -> ErrorEntry {
        ErrorEntry {
            id: id.into(),
            title: "sample defect".into(),
            status: Status::Open,
            component: "codegen::api".into(),
            severity: Severity::High,
            cmmi_pa: vec!["MA".into(), "CAR".into()],
            gap_ids: vec!["G-H03".into()],
            date: Utc::now(),
            location: "src/codegen/api.rs:42".into(),
            error_type: "build".into(),
            error_message: "cannot find symbol".into(),
            root_cause: "module path typo".into(),
            resolution: "fixed import".into(),
            lessons: Some("audit module paths".into()),
            related: vec![],
            error_code: Some("GEN-010".into()),
        }
    }

    #[test]
    fn next_id_increments_from_max() {
        let mut inv = Inventory::default();
        assert_eq!(inv.next_id(), "ERR-001");
        inv.add(sample_entry("ERR-001")).unwrap();
        inv.add(sample_entry("ERR-005")).unwrap();
        assert_eq!(inv.next_id(), "ERR-006");
    }

    #[test]
    fn add_rejects_duplicate_id() {
        let mut inv = Inventory::default();
        inv.add(sample_entry("ERR-001")).unwrap();
        assert!(inv.add(sample_entry("ERR-001")).is_err());
    }

    #[test]
    fn resolve_updates_status_and_lessons() {
        let mut inv = Inventory::default();
        inv.add(sample_entry("ERR-001")).unwrap();
        inv.resolve("ERR-001", "patched".into(), Some("lesson".into()))
            .unwrap();
        let e = &inv.entries["ERR-001"];
        assert_eq!(e.status, Status::Resolved);
        assert_eq!(e.resolution, "patched");
        assert_eq!(e.lessons.as_deref(), Some("lesson"));
    }

    #[test]
    fn render_markdown_contains_summary_and_detail() {
        let mut inv = Inventory::default();
        inv.add(sample_entry("ERR-001")).unwrap();
        let md = inv.render_markdown();
        assert!(md.contains("## Summary Table"));
        assert!(md.contains("### ERR-001"));
        assert!(md.contains("## Aggregate Stats"));
    }

    #[test]
    fn json_round_trip_preserves_entries() {
        let mut inv = Inventory::default();
        inv.add(sample_entry("ERR-001")).unwrap();
        inv.add(sample_entry("ERR-002")).unwrap();
        let s = serde_json::to_string(&inv).unwrap();
        let back: Inventory = serde_json::from_str(&s).unwrap();
        assert_eq!(back.entries.len(), 2);
        assert_eq!(back.entries["ERR-001"].title, "sample defect");
    }
}
