# hex-arch-gen — Hexagonal Architecture REST API Generator (Rust)

Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
Author: Awase Khirni Syed

Rust rewrite of the Python *“Hexagonal Flask Architecture — artefacts
generation from database”* script. A Rust binary (**`hex-gen`**) introspects a
PostgreSQL schema and emits a **second, fully-compiling Rust project** — an
Axum + SQLx REST API with OpenAPI 3 / Swagger UI, wired with the
**Enterprise Bus** (Command/Query) and **Enterprise Repository** patterns.

```
 ┌─────────────────────────────┐          ┌────────────────────────────────────┐
 │  hex-gen (generator crate)  │          │  generated-api (emitted project)   │
 │                             │          │                                    │
 │  PostgreSQL                 │          │  api/          inbound adapters    │
 │    │ information_schema     │          │    │         (axum + utoipa)       │
 │    │ pg_catalog / pg_enum   │  ──────► │    ▼                               │
 │    ▼                        │          │  enterprise/   CommandBus·QueryBus │
 │  IR (Schema/Table/Column/   │          │               Repository<T, ID>    │
 │      ForeignKey/Analysis)   │          │    ▼                               │
 │    ▼                        │          │  application/  services·mappers    │
 │  codegen (token templates)  │          │               bus handlers         │
 │    ▼                        │          │    ▼                               │
 │  56-file cargo project      │          │  domain/       entities·inputs     │
 │                             │          │               ports (traits)       │
 └─────────────────────────────┘          │    ▲                               │
                                          │  infrastructure/  SQLx adapters    │
                                          └────────────────────────────────────┘
```

---

## 1. Summarized steps

| # | step | command |
|---|------|---------|
| 1 | Build the generator | `cd generator && cargo build --release` |
| 2 | Point it at your database | `export DATABASE_URL=postgresql://awasekhirnisyed:YOUR_PASSWORD_HERE@localhost:5432/profxdb` |
| 3 | Generate the API project | `./target/release/hex-gen --project-name profx-api ./profx-api` |
| 4 | Run the generated API | `cd profx-api && cargo run` |
| 5 | Explore the API | open `http://localhost:8080/swagger-ui` (OpenAPI 3 doc at `/api-docs/openapi.json`) |
| 6 | Consume it | `GET/POST /api/v1/<table>`, `GET/PUT/DELETE /api/v1/<table>/{id}` + feature endpoints below |
| 7 | Evolve the schema, regenerate | re-run step 3 with `--force` |
| 8 | No database at hand? | `hex-gen --fixture ./generated-sample` (identical output, demo schema) |
| 9 | Want the live demo too? | `psql -d profxdb -f fixture.sql`, then repeat step 3 |

Everything the generator emits is plain Rust you own: no ORM, no macros that
need a live `DATABASE_URL` at compile time, no hidden codegen after
generation.

---

## 2. What the generator detects per table

The Rust counterpart of the Python `analyze_table()`:

| detector | columns matched | generated artefacts |
|---|---|---|
| **status** | `status`, `*_status` | `GET …/status/{status}`, `PATCH …/{id}/status`, port methods `find_by_status` / `set_status` |
| **soft delete** | `is_deleted`, `deleted`, `deleted_at` | `DELETE …/{id}/soft`, `POST …/{id}/restore`, port methods `soft_delete` / `restore` |
| **user relation** | `user_id`, `*_user_id` | `GET …/by-user/{user_id}`, port method `find_by_user` |
| **timestamps** | `created_at`, `updated_at` | `GET …/created-after`, `GET …/updated-after`; `updated_at = now()` bumped on UPDATE |
| **native enums** | `pg_enum` types with snake_case labels | typed Rust enums (`#[sqlx(type_name=…, rename_all="snake_case")]` + serde) with `FromStr` |
| **types** | uuid, int2/4/8, float4/8, numeric, bool, text/varchar, bytea, date, time, timestamp(±tz), json(b), arrays | precise Rust types (`Uuid`, `i32`, `Decimal`, `DateTime<Utc>`, `Vec<String>`, `serde_json::Value`, …) |

Tables that are skipped (with a warning): composite primary keys, no primary
key, names that are not valid Rust modules, plus built-in exclusions
(`schema_migrations`, `_sqlx_migrations`, `alembic_version`, … — extend with
`--exclude-tables a,b`).

---

## 3. Generated project layout (per table `orders`)

```
generated-api/
├── Cargo.toml                        axum 0.8 · sqlx 0.8 · utoipa 5 · swagger-ui 9
├── .env / .env.example               DATABASE_URL, BIND_ADDRESS, DB_MAX_CONNECTIONS
├── README.md                         endpoints of THIS project
└── src/
    ├── main.rs                       pool + bus wiring + router + SwaggerUi
    ├── config.rs                     12-factor env config
    ├── error.rs                      ApiError → 400/404/409/500 JSON
    ├── enterprise/
    │   ├── bus.rs                    CommandBus / QueryBus (TypeId-erased dispatch)
    │   └── repository.rs             Repository<T, ID> · Page · Paginated
    ├── domain/orders/
    │   ├── entity.rs                 Orders  (FromRow + Serialize + ToSchema)
    │   ├── enums.rs                  OrderStatusEnum (native PG enum)
    │   ├── input.rs                  Create/UpdateOrdersInput + validate()
    │   ├── repository.rs             OUTBOUND port  (trait, extends Repository)
    │   └── service.rs                INBOUND port   (trait)
    ├── application/
    │   ├── orders_mapper.rs          input → entity, partial-update merge
    │   ├── orders_service.rs         implements the inbound port
    │   └── orders_messaging.rs       commands · queries · bus handlers
    ├── infrastructure/
    │   └── orders_repository.rs      PostgresOrdersRepository (SQLx)
    └── api/orders/
        ├── dto.rs                    page params, status/after params
        └── handler.rs                #[utoipa::path] handlers → buses
```

**Hexagonal rule enforced by construction:** `domain` imports nothing but
`enterprise`; `application` depends only on domain ports; `infrastructure`
implements ports; `api` only talks to the buses. Writes travel
`handler → CommandBus → handler → service → repository port → SQLx adapter`;
reads travel the `QueryBus`.

---

## 4. Enterprise Bus — how dispatch works

* Commands (`Create/Update/Delete/SetStatus/SoftDelete/Restore<X>Command`) and
  queries (`Get/List/ListBy*/<X>CreatedAfterQuery`) are plain structs with an
  associated `Output` type.
* One `XHandlers` struct implements `CommandHandler<C>` / `QueryHandler<Q>`
  for **every** message of its context. Registration wraps it in a
  `CommandAdapter<C, H>` / `QueryAdapter<Q, H>` keyed by `TypeId`, so a single
  struct can safely serve many messages without clashing with Rust coherence
  rules (a latent bug in the draft this port replaces — its blanket
  `impl<H: CommandHandler<C>>` would break the moment one struct handled two
  commands).
* `send`/`ask` erase to `Box<dyn Any>`, dispatch in O(1), and downcast back to
  the declared output type.

## 5. Enterprise Repository — the contract

`Repository<T, ID>` gives every aggregate six canonical operations
(`find_by_id`, `find_all` + total count, `create` returning the DB-generated
row via `RETURNING *`, `update`, `delete`, `count`) with `Page`
(1-based, clamped) pagination, and a shared error taxonomy
(`NotFound / Validation / Constraint(23505→409) / Database`) mapped onto HTTP
by `error.rs`. Context ports add the feature methods from section 2.

---

## 6. Verified in this workspace (not just written — executed)

| verification | result |
|---|---|
| `cargo build` of `hex-gen` | ✅ 0 errors / 0 warnings |
| `hex-gen --fixture` | ✅ 3 tables, 56 files, deterministic output |
| `cargo build` of generated project | ✅ 0 errors / 0 warnings |
| Live introspection of a real PostgreSQL 17 (embedded, `fixture.sql` loaded) | ✅ identical IR, defaults detected |
| Generated server booted against live DB | ✅ connected, Swagger UI + OpenAPI 3 served (20 paths, 23 schemas) |
| `POST /users` → 201 (DB-generated id/status/timestamps via `RETURNING *`) | ✅ |
| `PUT` partial update (`updated_at = now()` bumped) | ✅ |
| `GET /orders/status/paid` + `PATCH …/status` (typed PG enum round-trip) | ✅ |
| soft delete → 204, restore → `is_deleted=false` | ✅ |
| `by-user`, `created-after`, pagination envelope `{items,total,page,per_page}` | ✅ |
| jsonb / `text[]` / `numeric` / keyword column `type` → `r#type` round-trips | ✅ |
| empty-email → `400 {"error":"email must not be empty"}`; unknown id → 404 | ✅ |

Reproduce the runtime proof with the embedded-postgres recipe in
`e2e_smoke.sh` (any PostgreSQL ≥ 13 with `gen_random_uuid()` works).

---

## 7. Design decisions vs. the Python original

| concern | Python original | this Rust port |
|---|---|---|
| async | `async def` inside Flask-RESTX (not actually async) | native tokio + axum |
| ORM | SQLAlchemy automap + hand-written `Column(...)` strings | `sqlx::FromRow`, no ORM, runtime-checked SQL |
| validation | Pydantic (broken `orm_mode`), duplicate status enums | typed input models + generated `validate()` + type system |
| repository | duplicated per-table code, undefined `filters` | one generic `Repository<T, ID>` + thin per-table ports |
| service wiring | `controllers[0].capitalize()` (broken) | explicit `Arc<dyn Port>` wiring in `main.rs` |
| enterprise bus | absent | real Command/Query bus, actually in the request path |
| OpenAPI | hand-written dict | `#[derive(OpenApi)]` + Swagger UI |
| native PG enums | hardcoded `VALUE1/2/3` | introspected from `pg_enum`, typed end-to-end |
| generated code compiles | ❌ (f-string/undefined-symbol bugs) | ✅ verified by building the output |

Deliberate simplifications (extensible in `generator/src/codegen/`):
CSV import/export and fuzzy search were dropped (the Python versions called
undefined helpers); one primary-key column per table; status/user/timestamp
detectors take the first matching column; enum columns with non-snake_case
labels fall back to `String` (sqlx cannot rename per-variant).

## 8. Repository layout of this workspace

```
hex-arch-gen/
├── README.md              ← you are here
├── fixture.sql            demo schema for the live-introspection path
├── e2e_smoke.sh           14-step runtime smoke test against a live DB
├── generator/             the hex-gen crate (this is the “whole script” rewrite)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs        CLI + pipeline orchestration
│       ├── config.rs      env/CLI configuration + table exclusions
│       ├── ir.rs          intermediate representation (Column/Table/Analysis)
│       ├── idents.rs      Rust identifier hygiene (keywords → r#, renames)
│       ├── analysis.rs    feature detection (port of analyze_table)
│       ├── introspect.rs  information_schema + pg_catalog → IR
│       ├── fixture.rs     built-in demo schema (no DB needed)
│       └── codegen/       mod · core_static · domain · application ·
│                          infrastructure · api · main_app · project_files
└── generated-sample/      a complete example produced by --fixture (builds clean)
```

All generator code and all templates carry inline `///` / `//!` documentation
explaining the pattern role of every artefact — start reading at
`generator/src/main.rs` and follow the pipeline top-down.
