# hex-arch-gen v2

<!--// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed-->
**Production-grade db-first generator for hexagonal-architecture Rust REST
APIs.**

Introspects a PostgreSQL schema (db-first) or the built-in fixture, and emits
a compiling Axum + SQLx hexagonal REST API matching the v0.6.0 desired-state
platform: presentation/application/domain/infrastructure layers +
shared/core/patterns/security/api/telemetry/logging/config cross-cutting
concerns, with OpenAPI 3, JWT auth, cursor pagination, ETag/If-Match
concurrency, saga hooks, structured logging, tracing, and a clean error
taxonomy.

```
┌──────────────────────────────────────┐        ┌──────────────────────────────────────┐
│  hex-gen (this crate)                │        │  <output>/  (emitted Rust project)    │
│  introspect → IR v2 → codegen        │  ───▶  │  domain/ application/ infrastructure/ │
│  + tracing + thiserror + ERROR_INV   │        │  presentation/ shared/ core/ patterns/│
│                                      │        │  security/ api/ telemetry/ logging/   │
└──────────────────────────────────────┘        └──────────────────────────────────────┘
```

## Quick start

```bash
# 1. Build the generator.
cargo build --release

# 2. Generate a sample project from the built-in fixture (no database needed).
./target/release/hex-gen --fixture --force ./generated-sample

# 3. Verify the generated project compiles.
cd generated-sample
cargo check
cargo run              # Swagger UI at http://localhost:8080/swagger-ui
```

## Live db-first run

```bash
export DATABASE_URL=postgresql://user:pass@localhost:5432/profxdb
hex-gen --database-url "$DATABASE_URL" --project-name profx-api ./profx-api
```

## CLI reference

| Flag                | Env                    | Default          | Purpose                                  |
|---------------------|------------------------|------------------|------------------------------------------|
| `<output>`          | `HEX_GEN_OUTPUT`       | `generated-api`  | Output directory.                        |
| `--database-url`    | `DATABASE_URL`         | —                | PostgreSQL URL (ignored in `--fixture`). |
| `--schema`          | `HEX_GEN_SCHEMA`       | `public`         | Schema to introspect.                    |
| `--project-name`    | `HEX_GEN_PROJECT_NAME` | `generated-api`  | Crate name of the generated project.     |
| `--fixture`         | `HEX_GEN_FIXTURE`      | false            | Use built-in demo schema (no DB).        |
| `--force`           | `HEX_GEN_FORCE`        | false            | Overwrite existing output directory.     |
| `--exclude-tables`  | `HEX_GEN_EXCLUDE_TABLES` | —              | Comma-separated tables to skip.          |
| `--json-logs`       | `HEX_GEN_JSON_LOGS`    | false            | Emit JSON Lines logs.                    |
| `-v` / `-vv` / `-vvv` | `HEX_GEN_VERBOSE`    | warn             | Verbosity (info/debug/trace).            |

## Architecture

### Generator pipeline

```text
PostgreSQL (information_schema + pg_catalog)         OR    built-in fixture
      │  introspect v2 (tracing, error taxonomy)            │  fixture::demo_schema
      ▼                                                      ▼
IR v2 (Schema → Table → Column / ForeignKey / Analysis v2)
      │  analysis v2 (status, soft-delete, tenancy, audit, timestamps, money, enum, FK)
      ▼
codegen (assets + domain + application + infrastructure + presentation
         + api + wiring + migrations + project_files)
      │
      ▼
./<output>/  — a compiling Axum + SQLx hexagonal REST API
```

### Generator source map

| File                          | Responsibility                                            |
|-------------------------------|-----------------------------------------------------------|
| `src/main.rs`                 | CLI entry; bootstraps logging; dispatches fixture/live.   |
| `src/lib.rs`                  | Library crate root re-exports.                            |
| `src/config.rs`               | Resolved generator config.                                |
| `src/idents.rs`               | snake_case → UpperCamel, keyword escape.                  |
| `src/errors.rs`               | `GenError` taxonomy + `GenResult` + `mask_password`.      |
| `src/logging.rs`              | `tracing` init (verbosity + JSON toggle).                 |
| `src/ir.rs`                   | IR v2 — Schema/Table/Column/ForeignKey/Index.             |
| `src/analysis.rs`             | Feature analysis v2 (timestamps, status, soft-delete, …). |
| `src/introspect.rs`           | Live PostgreSQL introspection (db-first).                 |
| `src/fixture.rs`              | Built-in demo schema.                                     |
| `src/ddl.rs`                  | DDL/migration SQL generation.                             |
| `src/report.rs`               | On-disk `GENERATION_REPORT.md`.                           |
| `src/codegen/mod.rs`          | Orchestrator + `GenerationStats` + `ImportSet`.           |
| `src/codegen/assets.rs`       | Static platform files (core/patterns/security/…).         |
| `src/codegen/domain.rs`       | Per-table entity + native PG enum.                        |
| `src/codegen/application.rs`  | Services + DTOs + ports (bus, repository).                |
| `src/codegen/infrastructure.rs` | SQLx adapters + EnterpriseBus + pool.                   |
| `src/codegen/presentation.rs` | Handlers + routes + state + middleware.                   |
| `src/codegen/api.rs`          | OpenAPI document.                                         |
| `src/codegen/wiring.rs`       | `main.rs` + `lib.rs`.                                     |
| `src/codegen/migrations.rs`   | Delegates to `ddl.rs`.                                    |
| `src/codegen/project_files.rs`| Cargo.toml/.env/README/Makefile/Dockerfile/docker-compose.|
| `tests/smoke.rs`              | Smoke test (fixture mode → cargo check).                  |

### Generated project layout

```text
<output>/
├── Cargo.toml             / .env / .env.example / .gitignore / README.md
├── Makefile               / Dockerfile / docker-compose.yml
├── config/default.toml    / GENERATION_REPORT.md
├── migrations/<ts>_init_schema.sql
└── src/
    ├── lib.rs             / main.rs
    ├── shared/{error,result,pagination,mod}.rs
    ├── config/{settings,mod}.rs
    ├── logging/{init,mod}.rs
    ├── telemetry.rs
    ├── core/{error_codes,ids,clock,aggregate,unit_of_work,shutdown,startup,
    │        bulkhead,saga,chaos,events,value_objects/{email,id,money},mod}.rs
    ├── patterns/{idempotency,outbox_worker,outbox_dlq,circuit_breaker,retry,
    │            pagination,filtering,webhooks,cache,feature_flags,load_shed,mod}.rs
    ├── security/{rls,gdpr,pii,secrets,headers,tenancy,
    │            auth/{extractor,jwt,refresh,dev_token,rbac,api_key,mod},mod}.rs
    ├── domain/{entities/<table>.rs, value_objects/mod.rs, mod.rs}
    ├── application/{dto/<table>_dto.rs, services/<table>_service.rs,
    │                ports/{bus,repository,mod}.rs, sagas/mod.rs, mod.rs}
    ├── infrastructure/{persistence/<table>_repository.rs, db/pool.rs,
    │                   bus/enterprise_bus.rs, timed_query.rs, mod.rs}
    ├── presentation/{state.rs, routes.rs, error.rs, middleware/*,
    │                 handlers/<table>_handler.rs, mod.rs}
    └── api/{openapi.rs, mod.rs}
```

## Per-table feature detection

The analysis pass detects these features per table and turns on the
corresponding generated artefacts:

| Feature                  | Trigger                                | Generated artefact                            |
|--------------------------|----------------------------------------|-----------------------------------------------|
| `status`                 | column `status` / `*_status`           | `GET /status/{status}`, `PATCH /{id}/status`  |
| `soft-delete`            | `is_deleted`/`deleted`/`deleted_at`    | `DELETE /{id}/soft`, `POST /{id}/restore`     |
| `user-ref`               | `user_id` / `*_user_id`                | `GET /by-user/{user_id}`                      |
| `tenant`                 | `tenant_id` / `*_tenant_id`            | RLS-aware queries (stub)                      |
| `optimistic-concurrency` | `version` integer column               | ETag + If-Match (handler stub)                |
| `email`                  | `email` text column                    | Typed `Email` value object hint               |
| `timestamps`             | `created_at` / `updated_at`            | `GET /created-after`, `GET /updated-after`    |
| `audit-log`              | table named `audit_log`/`*_audit`/…    | Read-only flag in docs                        |
| native enum              | `pg_type.typname` in `pg_enum`         | Rust enum + `FromStr` impl                    |

## ERROR_INVENTORY.md

The persistent error log lives at [`ERROR_INVENTORY.md`](./ERROR_INVENTORY.md)
— read it before starting new work to avoid repeating known mistakes. Every
error encountered during development is recorded with root cause, resolution,
status, and lessons learned.

## Deferred work

These items are tracked in `ERROR_INVENTORY.md` under "Deferred Work":

- Live PostgreSQL run (no DB in the sandbox during development).
- Full ASVS test suite (16 test files in the desired state).
- Full k8s/ops/docker files (curated subset emitted).
- GraphQL resolver generation (stub only).
- Saga step registry (basic stub).
- Outbox/DLQ/retention workers (basic stubs).
- Circuit breaker / load shed / bulkhead production implementations.
- Cache / feature flags / idempotency production implementations.
- Auth (JWT/refresh/api_key) — basic implementation.
- OTLP/Prometheus telemetry — stub.

Future iterations should pick these up incrementally, recording progress in
the ERROR_INVENTORY changelog.

## License

MIT.
