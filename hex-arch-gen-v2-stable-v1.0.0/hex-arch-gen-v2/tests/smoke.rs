//! Smoke test for the generator — runs the full pipeline in fixture mode
//! into a tempdir and asserts the canonical project files exist.
//!
//! v0.5.0 (closes G-L09): the test now also runs `cargo check` on the
//! generated project so a regression that produces non-compiling
//! Rust is caught at generator-test time, not at generated-sample
//! check time. The `cargo check` is gated behind the
//! `HEX_GEN_SMOKE_CARGO_CHECK=1` env var so it stays opt-in for
//! fast CI loops and local development — the full check takes ~60s.

use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

use hex_arch_gen::fixture::demo_schema;
use hex_arch_gen::config::Config;

#[test]
fn fixture_generates_canonical_project() {
    let schema = demo_schema();
    assert!(!schema.tables.is_empty(), "fixture must yield tables");

    let cfg = Config::from_parts(
        Some("postgresql://user:password@localhost:5432/dbname".into()),
        "public".into(),
        "generated-api".into(),
    )
    .expect("config");

    let dir = tempdir().expect("tempdir");
    let out: PathBuf = dir.path().join("generated-api");

    let stats = hex_arch_gen::codegen::generate(&schema, &out, &cfg, true)
        .expect("generation must succeed");

    // The on-disk report is written separately by main::run; mirror that here.
    hex_arch_gen::report::write_report(&out, &schema, &stats, &cfg)
        .expect("report write must succeed");

    assert!(stats.files_written > 30, "expected many files, got {}", stats.files_written);

    // Spot-check canonical files.
    for path in [
        "Cargo.toml",
        ".env",
        ".env.example",
        "README.md",
        "Makefile",
        "Dockerfile",
        "docker-compose.yml",
        "config/default.toml",
        "migrations/20240101000001_init_schema.sql",
        "src/lib.rs",
        "src/main.rs",
        "src/shared/error.rs",
        "src/core/mod.rs",
        "src/security/mod.rs",
        "src/api/openapi.rs",
        "src/presentation/state.rs",
        "src/presentation/routes.rs",
        "src/domain/entities/mod.rs",
        "src/domain/entities/users.rs",
        "src/domain/entities/orders.rs",
        "src/domain/entities/product_categories.rs",
        "src/application/ports/repository.rs",
        "src/application/ports/bus.rs",
        "src/application/services/users_service.rs",
        "src/infrastructure/persistence/users_repository.rs",
        "src/presentation/handlers/users_handler.rs",
        "GENERATION_REPORT.md",
    ] {
        let p = out.join(path);
        assert!(p.exists(), "missing canonical file: {}", p.display());
    }

    // v0.5.0 (closes G-L09): optionally run `cargo check` on the
    // generated project so a regression that produces non-compiling
    // Rust is caught here, not at the generated-sample check time.
    // The check is opt-in via `HEX_GEN_SMOKE_CARGO_CHECK=1` because
    // it takes ~60s on a warm cache and would slow the fast CI loop.
    if std::env::var("HEX_GEN_SMOKE_CARGO_CHECK")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
    {
        eprintln!("[smoke] HEX_GEN_SMOKE_CARGO_CHECK=1 — running cargo check on {}", out.display());
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(&cargo)
            .arg("check")
            .arg("--offline")
            .current_dir(&out)
            .status()
            .expect("failed to spawn cargo check");
        assert!(
            status.success(),
            "cargo check on generated project failed — see output above"
        );
        eprintln!("[smoke] cargo check passed on generated project");
    } else {
        eprintln!("[smoke] HEX_GEN_SMOKE_CARGO_CHECK not set — skipping cargo check (opt-in)");
    }
}

#[test]
fn fixture_schema_has_expected_features() {
    let schema = demo_schema();
    let users = schema.table("users").expect("users table");
    assert!(users.analysis.email_col.is_some(), "users should detect email");
    assert!(users.analysis.version_col.is_some(), "users should detect version");
    assert!(users.analysis.has_soft_delete(), "users should detect soft delete");

    let orders = schema.table("orders").expect("orders table");
    assert!(orders.analysis.has_tenancy(), "orders should detect tenant_id");
    assert!(orders.analysis.user_col.is_some(), "orders should detect user_id");
}

/// v1.0.0 (closes G-M16): scans every generated file for unreplaced
/// template placeholders. The generator uses `format!()` and string
/// substitution throughout; a bug that leaves a `{placeholder}` or
/// `__TOKEN__` unreplaced would produce broken Rust. This test walks
/// the generated project directory and fails if any file contains a
/// known placeholder pattern.
#[test]
fn generated_files_contain_no_template_placeholders() {
    use std::fs;
    let schema = demo_schema();
    let cfg = Config::from_parts(
        Some("postgresql://user:password@localhost:5432/dbname".into()),
        "public".into(),
        "generated-api".into(),
    )
    .expect("config");
    let dir = tempdir().expect("tempdir");
    let out: PathBuf = dir.path().join("generated-api");
    hex_arch_gen::codegen::generate(&schema, &out, &cfg, true)
        .expect("generation must succeed");

    // Known placeholder patterns the generator uses (or might leak).
    let placeholder_patterns = [
        "{module}",       // format!() named arg
        "{struct_name}",  // format!() named arg
        "{m}",            // shorthand
        "{n}",            // shorthand
        "__BIN_NAME__",  // Dockerfile placeholder
        "__PLACEHOLDER__",
        "TODO_TEMPLATE",
        "@TOKEN@",
    ];

    let mut violations: Vec<String> = Vec::new();
    fn walk(dir: &std::path::Path, patterns: &[&str], violations: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, patterns, violations);
            } else if path.extension().map(|e| e == "rs" || e == "toml" || e == "yml" || e == "md" || e == "sql").unwrap_or(false) {
                let Ok(content) = fs::read_to_string(&path) else { continue };
                for pat in patterns {
                    if content.contains(pat) {
                        violations.push(format!(
                            "{}: contains unreplaced placeholder `{}`",
                            path.display(),
                            pat
                        ));
                    }
                }
            }
        }
    }
    walk(&out, &placeholder_patterns, &mut violations);
    assert!(
        violations.is_empty(),
        "template placeholder leakage detected:\n{}",
        violations.join("\n")
    );
}
