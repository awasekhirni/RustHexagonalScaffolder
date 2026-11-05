// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! On-disk generation report — `GENERATION_REPORT.md` next to the project.
//!
//! Captures what was generated, when, from what source (fixture/live),
//! the file count, and a per-table feature matrix. Useful for diffing
//! regeneration runs in CI.
//!
//! v0.5.0 (closes G-L10): the report now includes the `hex-gen`
//! version that produced it, so a CI diff between two
//! `GENERATION_REPORT.md` files immediately shows whether a
//! regression was caused by a generator version bump or by a schema
//! change. The version is sourced from `env!("CARGO_PKG_VERSION")`
//! at compile time so it always matches the binary that wrote the
//! report.

use std::path::Path;

use tracing::{info, instrument};

use crate::codegen::GenerationStats;
use crate::config::Config;
use crate::errors::GenResult;
use crate::ir::Schema;

/// Writes `GENERATION_REPORT.md` AND `GENERATION_REPORT.json` next to
/// the generated project.
///
/// v1.0.0 (closes G-M13): the report is now emitted in both Markdown
/// (for humans) and JSON (for CI tooling). The JSON view has a stable
/// schema so a CI pipeline can parse it without regex-grepping the
/// Markdown. Failures are logged but never propagated: the report is
/// informational, not a deliverable.
///
/// # Errors
///
/// This function never returns an error — even when the underlying
/// `std::fs::write` fails, the error is logged via `tracing` and
/// `Ok(())` is returned so the generator's main pipeline does not
/// abort on a non-fatal report-write failure.
///
/// # CMMI mapping
///
/// * **OPF SG 2** — Deploy organisational process assets: the report
///   is the process asset that records what was generated and by
///   which version of the generator.
/// * **MA SG 2** — Collect data: the per-table feature matrix feeds
///   downstream measurement aggregation.
pub fn write_report(
    out: &Path,
    schema: &Schema,
    stats: &GenerationStats,
    cfg: &Config,
) -> GenResult<()> {
    let md_path = out.join("GENERATION_REPORT.md");
    let body = render(schema, stats, cfg);
    if let Err(e) = std::fs::write(&md_path, body) {
        info!(error = %e, "could not write GENERATION_REPORT.md (non-fatal)");
    }
    // v1.0.0 (closes G-M13): JSON view for CI tooling.
    let json_path = out.join("GENERATION_REPORT.json");
    let json = render_json(schema, stats, cfg);
    if let Err(e) = std::fs::write(&json_path, json) {
        info!(error = %e, "could not write GENERATION_REPORT.json (non-fatal)");
    }
    Ok(())
}

/// Renders the report body to a string.
///
/// Exposed via `pub(crate)` so future tests in the generator can
/// assert on the report content without going through the filesystem.
#[instrument(skip(schema, stats, cfg), fields(
    category = "report",
    function_id = "report::render",
    tables = schema.tables.len(),
))]
fn render(schema: &Schema, stats: &GenerationStats, cfg: &Config) -> String {
    let mut out = String::new();
    out.push_str("# Generation Report\n\n");
    out.push_str(&format!("- **Project:** `{}`\n", cfg.project_name));
    out.push_str(&format!("- **Schema:** `{}`\n", cfg.schema));
    // v0.5.0 (closes G-L10): include the hex-gen version so a CI
    // diff between two reports immediately shows whether a change
    // was caused by a generator version bump or a schema change.
    out.push_str(&format!(
        "- **Generator:** `hex-gen v{}`\n",
        env!("CARGO_PKG_VERSION")
    ));
    out.push_str(&format!(
        "- **Generated at:** {}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));
    out.push_str(&format!("- **Files written:** {}\n", stats.files_written));
    out.push_str(&format!(
        "- **Bytes written:** {} ({:.2} KiB)\n",
        stats.bytes_written,
        stats.bytes_written as f64 / 1024.0
    ));
    out.push_str(&format!("- **Tables:** {}\n", schema.tables.len()));
    out.push_str(&format!("- **Native enums:** {}\n", schema.enums.len()));
    out.push_str(&format!(
        "- **Secondary indexes:** {}\n\n",
        schema.indexes.len()
    ));

    out.push_str("## Tables\n\n");
    out.push_str("| table | rust struct | pk | features |\n");
    out.push_str("|---|---|---|---|\n");
    for t in &schema.tables {
        let pk = t
            .pk()
            .map(|c| format!("`{}:{}`", c.name, c.kind.rust_type()))
            .unwrap_or_else(|| "-".into());
        let feats = if t.analysis.features().is_empty() {
            "-".to_string()
        } else {
            t.analysis.features().join(", ")
        };
        out.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            t.name, t.struct_name, pk, feats
        ));
    }

    if !schema.indexes.is_empty() {
        out.push_str("\n## Secondary indexes\n\n");
        out.push_str("| name | table | columns | unique |\n");
        out.push_str("|---|---|---|---|\n");
        for idx in &schema.indexes {
            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                idx.name,
                idx.table,
                idx.columns.join("`, `"),
                if idx.unique { "yes" } else { "no" }
            ));
        }
    }

    out
}

/// Renders the report as a JSON string for CI tooling.
///
/// v1.0.0 (closes G-M13). The JSON schema is intentionally flat so
/// a `jq` one-liner can extract any field. The schema:
///
/// ```json
/// {
///   "project": "generated-api",
///   "schema": "public",
///   "generator_version": "1.0.0",
///   "generated_at": "2026-11-05T16:42:56Z",
///   "files_written": 123,
///   "bytes_written": 256935,
///   "tables": [
///     { "name": "orders", "struct": "Orders", "pk": "id:Uuid",
///       "features": ["status", "soft-delete", ...] }
///   ],
///   "native_enums": 2,
///   "secondary_indexes": 3
/// }
/// ```
///
/// # Errors
///
/// This function never returns an error — it builds the JSON string
/// manually (not via serde_json) so it cannot fail on serialisation.
/// The resulting string is guaranteed to be valid JSON.
#[instrument(skip(schema, stats, cfg), fields(
    category = "report",
    function_id = "report::render_json",
    tables = schema.tables.len(),
))]
fn render_json(schema: &Schema, stats: &GenerationStats, cfg: &Config) -> String {
    let tables_json: Vec<String> = schema
        .tables
        .iter()
        .map(|t| {
            let pk = t
                .pk()
                .map(|c| format!("{}:{}", c.name, c.kind.rust_type()))
                .unwrap_or_else(|| "-".into());
            let features = t.analysis.features();
            let features_json = if features.is_empty() {
                "[]".to_string()
            } else {
                let quoted: Vec<String> = features.iter().map(|f| format!("\"{f}\"")).collect();
                format!("[{}]", quoted.join(", "))
            };
            format!(
                r#"{{"name":"{}","struct":"{}","pk":"{}","features":{}}}"#,
                escape_json(&t.name),
                escape_json(&t.struct_name),
                escape_json(&pk),
                features_json
            )
        })
        .collect();
    format!(
        r#"{{"project":"{}","schema":"{}","generator_version":"{}","generated_at":"{}","files_written":{},"bytes_written":{},"tables":[{}],"native_enums":{},"secondary_indexes":{}}}"#,
        escape_json(&cfg.project_name),
        escape_json(&cfg.schema),
        env!("CARGO_PKG_VERSION"),
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        stats.files_written,
        stats.bytes_written,
        tables_json.join(","),
        schema.enums.len(),
        schema.indexes.len(),
    )
}

/// Escapes a string for safe inclusion in a JSON string literal.
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Schema;

    #[test]
    fn render_includes_generator_version() {
        // v0.5.0 (closes G-L10): the report must include the
        // hex-gen version so CI diffs can attribute changes to a
        // generator bump rather than a schema change.
        let schema = Schema::default();
        let stats = GenerationStats {
            files_written: 1,
            bytes_written: 100,
        };
        let cfg = Config {
            database_url: "postgresql://u:p@h:5432/d".into(),
            schema: "public".into(),
            project_name: "test".into(),
        };
        let body = render(&schema, &stats, &cfg);
        assert!(
            body.contains(&format!("hex-gen v{}", env!("CARGO_PKG_VERSION"))),
            "report must contain the generator version"
        );
    }

    #[test]
    fn render_json_is_valid_json() {
        // v1.0.0 (closes G-M13): the JSON report must be parseable
        // by a CI pipeline without regex-grepping the Markdown.
        let schema = Schema::default();
        let stats = GenerationStats {
            files_written: 1,
            bytes_written: 100,
        };
        let cfg = Config {
            database_url: "postgresql://u:p@h:5432/d".into(),
            schema: "public".into(),
            project_name: "test".into(),
        };
        let json_str = render_json(&schema, &stats, &cfg);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("render_json must produce valid JSON");
        assert_eq!(parsed["project"], "test");
        assert_eq!(parsed["generator_version"], env!("CARGO_PKG_VERSION"));
    }
}
