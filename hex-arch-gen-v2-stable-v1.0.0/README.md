# hex-arch-gen v2 — Deliverables

This directory contains the enhanced generator and a generated sample
project. The generator introspects a PostgreSQL schema (db-first) or uses
the built-in fixture, and emits a compiling hexagonal-architecture Rust
REST API matching the v0.6.0 desired-state platform.

## Contents

| Path                       | Description                                                  |
|----------------------------|--------------------------------------------------------------|
| `hex-arch-gen-v2/`         | The enhanced generator source (Rust crate).                 |
| `hex-arch-gen-v2/README.md`| Generator usage and architecture overview.                   |
| `hex-arch-gen-v2/ARCHITECTURE.md` | Pipeline diagram + layered architecture.             |
| `hex-arch-gen-v2/ERROR_INVENTORY.md` | Persistent error log (20 errors, all resolved).   |
| `generated-sample/`        | A sample generated project (fixture mode, 111 files).       |
| `generated-sample/README.md`| Per-project README with endpoint matrix.                    |
| `generated-sample/GENERATION_REPORT.md` | What was generated + per-table features.      |

## Quick start

```bash
# 1. Build the generator.
cd hex-arch-gen-v2
cargo build --release

# 2. Generate a sample project from the built-in fixture (no database needed).
./target/release/hex-gen --fixture --force ../generated-sample

# 3. Verify the generated project compiles.
cd ../generated-sample
cargo check          # zero errors, ~150 unused-import warnings
cargo run            # Swagger UI at http://localhost:8080/swagger-ui
```

## Live db-first run

```bash
export DATABASE_URL=postgresql://user:pass@localhost:5432/profxdb
hex-gen --database-url "$DATABASE_URL" --project-name profx-api ./profx-api
```

## Status

- Generator: **builds cleanly**, **2 smoke tests passing**.
- Generated sample (fixture mode): **`cargo check` passes with zero errors**.
- ERROR_INVENTORY: **20 errors resolved, 10 deferred scope items tracked**.

See `hex-arch-gen-v2/ERROR_INVENTORY.md` for the full error history and
deferred-work list.
