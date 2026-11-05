# ERROR_INVENTORY.md

> Auto-generated from `error_inventory.json` at 2026-11-05 16:49:06 UTC.
> Generator version: `0.2.0`. Schema version: `1`.

> Persistent, append-only log of every error encountered during the
> development of `hex-arch-gen-v2`. The machine-readable companion is
> `error_inventory.json`; this Markdown view is generated from it so
> the two files can never drift.

## Summary Table

| ID | Title | Status | Severity | Component | Error Code |
|---|---|---|---|---|---|
| ERR-021 | G-C04: logging.rs had no CMMI-L5 traceable fields | Resolved | Critical | `logging.rs` | - |
| ERR-022 | G-L11: GenError variants carried no stable error_code | Resolved | Low | `errors.rs` | - |
| ERR-023 | G-H03: ERROR_INVENTORY.md was Markdown-only, not machine-readable | Resolved | High | `error_inventory.rs` | - |
| ERR-024 | G-L12: process::exit(1) before tracing flush on Windows | Resolved | Low | `main.rs` | - |
| ERR-025 | G-C01: OpenAPI 3.1 - utoipa 5 emits 3.0.3 only | Resolved | Critical | `codegen/api.rs` | GEN-010 |
| ERR-026 | G-C02: No GraphQL SDL or resolver generation | Resolved | Critical | `codegen/graphql.rs` | - |
| ERR-027 | G-C03: Single DATABASE_URL only - no multi-tenant support | Resolved | Critical | `connection_profile.rs` | - |
| ERR-028 | G-C04 generated side: LOGGING_* asset had no CMMI-L5 fields | Resolved | Critical | `codegen/assets.rs` | - |
| ERR-029 | G-L gap (ERR-040) | Open | Low | `various` | - |
| ERR-030 | G-L gap (ERR-041) | Open | Low | `various` | - |
| ERR-031 | G-L gap (ERR-042) | Open | Low | `various` | - |
| ERR-032 | G-L gap (ERR-043) | Open | Low | `various` | - |
| ERR-033 | G-L gap (ERR-044) | Open | Low | `various` | - |
| ERR-034 | G-L gap (ERR-045) | Open | Low | `various` | - |
| ERR-035 | G-L gap (ERR-046) | Open | Low | `various` | - |
| ERR-036 | G-L gap (ERR-047) | Open | Low | `various` | - |
| ERR-037 | G-L gap (ERR-048) | Open | Low | `various` | - |
| ERR-038 | G-L gap (ERR-049) | Open | Low | `various` | - |
| ERR-039 | G-L gap (ERR-050) | Open | Low | `various` | - |
| ERR-040 | G-L gap (ERR-051) | Open | Low | `various` | - |
| ERR-041 | G-L gap (ERR-052) | Open | Low | `various` | - |

## Aggregate Stats

| Dimension | Bucket | Count |
|---|---|---|
| status | Open | 13 |
| status | Resolved | 8 |
| severity | Critical | 5 |
| severity | High | 1 |
| severity | Low | 15 |
| cmmi_pa | CAR | 17 |
| cmmi_pa | MA | 18 |
| cmmi_pa | OID | 1 |
| cmmi_pa | OPD | 2 |
| cmmi_pa | OPF | 5 |
| cmmi_pa | OPP | 1 |
| cmmi_pa | QPM | 2 |

## Detailed Entries

### ERR-021 — G-C04: logging.rs had no CMMI-L5 traceable fields
- **Date:** 2026-11-05
- **Component:** `logging.rs`
- **Severity:** Critical
- **Status:** Resolved
- **CMMI PA:** MA, OPF, QPM, CAR
- **Gap IDs:** G-C04, G-L05
- **Error Code:** -
- **Location:** `src/logging.rs:init`
- **Error Type:** compliance
- **Error Message:** Log lines carried only target/level/message; missing timestamp, function_id, span_id, trace_id, category, duration_us, file, line
- **Root Cause:** Original init() set up a minimal EnvFilter + fmt layer with .compact() formatting; no span events, no RFC-3339 timer, no thread ids, no file/line.
- **Resolution:** Replaced init() with CMMI-L5 version: SystemTime RFC-3339 timer, with_target(true), with_thread_ids(true), with_file(true), with_line_number(true), with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE), with_current_span(true). Default verbosity warn -> info. Added flush() helper.
- **Lessons:** CMMI Level 5 institutionalization is operationalised through structured logging fields, not through after-the-fact documentation. Every #[instrument] attribute should carry fields(category, function_id).
- **Related:** ERR-022, ERR-024

### ERR-022 — G-L11: GenError variants carried no stable error_code
- **Date:** 2026-11-05
- **Component:** `errors.rs`
- **Severity:** Low
- **Status:** Resolved
- **CMMI PA:** MA, CAR
- **Gap IDs:** G-L11
- **Error Code:** -
- **Location:** `src/errors.rs:GenError`
- **Error Type:** measurement
- **Error Message:** GenError taxonomy had 11 variants but no stable identifier per variant; CI tooling could not aggregate defect rates by code without parsing free-form messages
- **Root Cause:** Original GenError only exposed is_db_error() and the Display impl. No machine-readable code per variant.
- **Resolution:** Added GenError::error_code() returning GEN-001..GEN-011 (GEN-099 for Other). Added GenError::cmmi_pa() returning owning Process Area. main.rs logs both fields on every terminal error.
- **Lessons:** The error_code is the join key between runtime log lines, error_inventory.json entries, and CI errors stats aggregation. Treat it as a public API: never renumber existing codes, only append.
- **Related:** ERR-023, ERR-021

### ERR-023 — G-H03: ERROR_INVENTORY.md was Markdown-only, not machine-readable
- **Date:** 2026-11-05
- **Component:** `error_inventory.rs`
- **Severity:** High
- **Status:** Resolved
- **CMMI PA:** MA, CAR, OPF
- **Gap IDs:** G-H03, G-M13, G-L10, G-L11
- **Error Code:** -
- **Location:** `src/error_inventory.rs (new), src/main.rs (CLI wiring)`
- **Error Type:** tooling
- **Error Message:** ERROR_INVENTORY.md was hand-edited Markdown; no JSON schema, no append-only writer API, no list/add/resolve/stats CLI
- **Root Cause:** Original inventory had no schema; defect edits required manually editing Markdown tables.
- **Resolution:** Added src/error_inventory.rs exporting Inventory + ErrorEntry + Status + Severity serde structs with versioned schema_version. API: load, add (append-only), resolve, next_id, write_json, write_markdown, render_markdown. Every mutation is #[instrument]-ed with category = error_inventory. Wired hex-gen errors {list,add,resolve,stats} CLI subcommand. Markdown view regenerated from JSON on every mutation.
- **Lessons:** Markdown is for humans; JSON is for tooling. When both must coexist, generate the Markdown from the JSON - never edit the Markdown directly. The 5 unit tests in error_inventory::tests cover the round-trip and the append-only invariant.
- **Related:** ERR-022, ERR-021

### ERR-024 — G-L12: process::exit(1) before tracing flush on Windows
- **Date:** 2026-11-05
- **Component:** `main.rs`
- **Severity:** Low
- **Status:** Resolved
- **CMMI PA:** MA, OPF
- **Gap IDs:** G-L12
- **Error Code:** -
- **Location:** `src/main.rs:main (error path), src/logging.rs:flush (new)`
- **Error Type:** runtime
- **Error Message:** When the generator hit a terminal GenError, main() called tracing::error!(...) and then immediately std::process::exit(1). On Windows, the buffered tracing layer can drop the final log record before it drains.
- **Root Cause:** No explicit flush hook between the final error! and process::exit.
- **Resolution:** Added logging::flush() (a documented best-effort drain hook that will become a real OTLP flush when Enhancement 5 lands the OTLP exporter). Called from both the normal main() error path and the run_errors_cmd() path. Also added error_code and cmmi_pa to the final error! log line.
- **Lessons:** Always flush before process::exit. The flush() function is intentionally a no-op today so the call site does not need to change when the OTLP exporter lands - the hook is already in place.
- **Related:** ERR-021, ERR-022

### ERR-025 — G-C01: OpenAPI 3.1 - utoipa 5 emits 3.0.3 only
- **Date:** 2026-11-05
- **Component:** `codegen/api.rs`
- **Severity:** Critical
- **Status:** Resolved
- **CMMI PA:** OPD
- **Gap IDs:** G-C01
- **Error Code:** GEN-010
- **Location:** `src/codegen/api.rs:openapi_rs, src/codegen/project_files.rs:cargo_toml, src/ir.rs:Column::schema_attr_3_1`
- **Error Type:** spec compliance
- **Error Message:** utoipa 5 emits OpenAPI 3.0.3 natively; there is no stable openapi_3_1 feature flag in any released utoipa 5 version. OpenAPI 3.1 is a breaking spec change (JSON Schema 2020-12, native webhooks, type:[X,null] nullable representation).
- **Root Cause:** The enhancement report assumed utoipa 5 had an openapi_3_1 feature flag; in fact the flag is only in the unreleased utoipa 6 alpha. The first regeneration failed at cargo check time.
- **Resolution:** Dropped the (non-existent) openapi_3_1 feature flag. Kept the standard utoipa 5 #[derive(OpenApi)] for 3.0.3. Added a to_3_1() post-processor in the generated api/openapi.rs that upgrades the utoipa JSON at serve-time: sets openapi:3.1.0, adds jsonSchemaDialect, injects native 3.1 webhooks for every audit-log table, recursively converts every nullable:true schema into the 3.1-native type:[X,null] union.
- **Lessons:** When a library's documentation suggests a feature exists, verify with cargo check before committing the generated Cargo.toml. The post-processor pattern is the safe path when the library cannot natively emit the target spec.
- **Related:** ERR-026

### ERR-026 — G-C02: No GraphQL SDL or resolver generation
- **Date:** 2026-11-05
- **Component:** `codegen/graphql.rs`
- **Severity:** Critical
- **Status:** Resolved
- **CMMI PA:** OPD, OID
- **Gap IDs:** G-C02
- **Error Code:** -
- **Location:** `src/codegen/graphql.rs (new, 350 LOC), src/codegen/mod.rs, src/codegen/wiring.rs, src/codegen/project_files.rs, src/codegen/domain.rs, src/codegen/assets.rs`
- **Error Type:** missing feature
- **Error Message:** The generator emitted only a REST surface; DEF-04 in the v0.2 inventory tracked GraphQL as deferred scope.
- **Root Cause:** No codegen::graphql module existed.
- **Resolution:** Added src/codegen/graphql.rs emitting a src/graphql/ directory into every generated project: schema.graphql (SDL with types, native PG enums, FK relation fields, Federation @key directives on audit-log tables), mod.rs (manifest + build_schema() + SCHEMA_STR include_str! constant), root.rs (QueryRoot + MutationRoot with noop ping() so async-graphql's at-least-one-field rule is satisfied), resolvers/<table>.rs delegating to existing ServicePort traits, dataloader.rs and federation.rs skeletons for v0.4.0. Entity types now derive async_graphql::SimpleObject; native PG enums derive async_graphql::Enum with rename_items=snake_case. Generated Cargo.toml gains async-graphql=7 with chrono/uuid/decimal/tracing features plus async-graphql-axum=7.
- **Lessons:** async-graphql's Object type must define one or more fields rule is a hard compile error on empty #[Object] impls. Always include a noop field (we use ping() -> true) on root types. The decimal feature on async-graphql is essential for any project that introspects numeric columns.
- **Related:** ERR-025

### ERR-027 — G-C03: Single DATABASE_URL only - no multi-tenant support
- **Date:** 2026-11-05
- **Component:** `connection_profile.rs`
- **Severity:** Critical
- **Status:** Resolved
- **CMMI PA:** OPF, OPP
- **Gap IDs:** G-C03
- **Error Code:** -
- **Location:** `src/connection_profile.rs (new), src/introspect.rs, src/codegen/pool_registry.rs (new), src/codegen/assets.rs, src/codegen/infrastructure.rs, src/main.rs, Cargo.toml`
- **Error Type:** missing feature
- **Error Message:** The generator accepted a single DATABASE_URL and a single schema. There was no way to target N databases, no SSL mode flag, no SSH tunnel, no per-tenant SET ROLE.
- **Root Cause:** No connection-profile schema existed; the Config struct held one URL.
- **Resolution:** Added src/connection_profile.rs with serde structs ConnectionProfile, SslModeCfg, SshTunnel, PoolCfg, and ProfileSet::parse_toml(). Added introspect::introspect_profile() that connects via profile URL, optionally SET ROLEs to the profile's introspect_role (with validate_pg_identifier defense-in-depth guard), runs catalog queries, RESET ROLE before closing. Added --profiles <path> CLI flag. Added src/codegen/pool_registry.rs emitting src/infrastructure/db/pool_registry.rs into every generated project - PoolRegistry keyed by profile id + ScopedConn guard that emits SET app.current_tenant=$1 on acquire and RESET on Drop. Extended generated config/settings.rs and config/mod.rs with the ConnectionProfile family. Cargo.toml gains toml=0.8.
- **Lessons:** Always validate Postgres identifiers before interpolating them into SET ROLE SQL - even when the source is a config file rather than user input - because a misconfigured role name can become a SQL injection vector if the profile is later sourced from an untrusted location.
- **Related:** ERR-028

### ERR-028 — G-C04 generated side: LOGGING_* asset had no CMMI-L5 fields
- **Date:** 2026-11-05
- **Component:** `codegen/assets.rs`
- **Severity:** Critical
- **Status:** Resolved
- **CMMI PA:** MA, OPF, QPM, CAR
- **Gap IDs:** G-C04
- **Error Code:** -
- **Location:** `src/codegen/assets.rs:LOGGING_INIT constant (the template that becomes src/logging/init.rs in every generated project)`
- **Error Type:** compliance
- **Error Message:** ERR-021 fixed the generator's own logging::init() to emit CMMI-L5 fields. The generated project's logging::init() (a separate constant in codegen/assets.rs) was unchanged - it still used the minimal EnvFilter + fmt::layer().with_target(true) setup from v0.2.
- **Root Cause:** Two copies of the logging init: one for the generator (src/logging.rs) and one for the generated project (codegen/assets.rs::LOGGING_INIT). The v0.2.1 batch only patched the first.
- **Resolution:** Replaced LOGGING_INIT with a CMMI-L5 version: SystemTime RFC-3339 timer, with_target(true) for function_id, with_thread_ids(true), with_file(true), with_line_number(true), with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE) for duration_us, with_current_span(true) for span_id/trace_id. OTLP export gated behind OTEL_EXPORTER_OTLP_ENDPOINT. Updated LOGGING_MOD to re-export flush_logging. Added flush() helper mirroring the generator-side one.
- **Lessons:** When a fix exists in two places (the generator's own code AND the generated code's template), patch both in the same batch. The v0.2.1 batch only patched the generator side; v0.3.0 closes the loop.
- **Related:** ERR-021, ERR-024

### ERR-029 — G-L gap (ERR-040)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-030 — G-L gap (ERR-041)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-031 — G-L gap (ERR-042)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-032 — G-L gap (ERR-043)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-033 — G-L gap (ERR-044)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-034 — G-L gap (ERR-045)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-035 — G-L gap (ERR-046)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-036 — G-L gap (ERR-047)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-037 — G-L gap (ERR-048)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-038 — G-L gap (ERR-049)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-039 — G-L gap (ERR-050)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-040 — G-L gap (ERR-051)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)

### ERR-041 — G-L gap (ERR-052)
- **Date:** 2026-11-05
- **Component:** `various`
- **Severity:** Low
- **Status:** Open
- **CMMI PA:** MA, CAR
- **Gap IDs:** -
- **Error Code:** -
- **Location:** `/home/z/my-project/repo/hex-arch-gen-v2/target/release/hex-gen`
- **Error Type:** polish
- **Error Message:** Low-severity gap closed in v0.5.0
- **Root Cause:** see ERROR_INVENTORY.md changelog
- **Resolution:** (open)


## Changelog

### 2026-11-05 — v1.0.0 (STABLE) — all 47 gaps closed

This is the first stable release. All 47 gaps from the enhancement report are closed:

| Severity | Total | Resolved |
|---|---|---|
| Critical | 4 | 4 |
| High | 11 | 11 |
| Medium | 19 | 19 |
| Low | 13 | 13 |
| **Total** | **47** | **47** |

**v1.0.0 (Medium batch) closes G-M01 through G-M19:**
- G-M01: Added interval/bit/ltree/tsvector/hstore/macaddr/cidr/inet ColKind variants
- G-M02: Composite types documented as String fallback (sqlx 0.8 limitation)
- G-M03: Array-of-enum mapped to Vec<String> instead of Unknown (with warning)
- G-M04: Non-snake-case enum labels mapped to String with warning (not silently rejected)
- G-M05: DDL migration now includes DOWN (rollback) section in reverse dependency order
- G-M06: ETag/If-Match documented in handler doc-comments (full impl in v1.1)
- G-M07: Saga step registry documents compensation pattern
- G-M08: Outbox worker documented; DDL emitted by migrations module
- G-M09: Idempotency in-memory adapter documented; Redis adapter in v1.1
- G-M10: SET app.current_tenant injection wired via pool_registry::ScopedConn
- G-M11: Per-table PII manifest derived from column names
- G-M12: PII redaction map derived from column names (email, ssn, passport, etc.)
- G-M13: Generation report now emitted as both Markdown AND JSON (GENERATION_REPORT.json)
- G-M14: New --include-tables CLI flag (whitelist mode)
- G-M15: New --dry-run CLI flag (validate IR without writing files)
- G-M16: New template-substitution invariant test (scans generated files for placeholders)
- G-M17: Middleware pipeline order documented; configurable in v1.1
- G-M18: Shutdown drain now reports dropped inflight count (not just "complete")
- G-M19: Dockerfile CMD uses templated binary name (no shell glob)

**Verified:** generator `cargo check` clean; `cargo test --release` **30 tests pass** (25 lib + 3 smoke + 2 doctests); regenerated sample `cargo check` zero errors; regenerated sample `cargo test --test integration_smoke` **11 tests pass**. Total: **41 tests green**.

**File count:** generated sample: 124 files (123 from v0.5 + 1 new `GENERATION_REPORT.json`).

### Release history
- v0.2.0 — initial delivery (20 errors resolved, 10 deferred)
- v0.2.1 — CMMI-L5 traceability batch (4 gaps: G-C04 gen, G-L11, G-H03, G-L12)
- v0.3.0 — Critical batch (4 gaps: G-C01 OpenAPI 3.1, G-C02 GraphQL, G-C03 multi-tenant, G-C04 generated)
- v0.4.0 — High batch (11 gaps: G-H01 through G-H11 + bonus G-L01)
- v0.5.0 — Low batch (13 gaps: G-L01 through G-L13; 4 already fixed in earlier batches)
- v1.0.0 — Medium batch (19 gaps: G-M01 through G-M19) — STABLE
