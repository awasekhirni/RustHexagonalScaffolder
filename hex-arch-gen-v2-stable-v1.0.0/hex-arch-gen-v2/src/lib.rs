// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Library crate root for `hex_arch_gen`.
//!
//! Re-exports the public modules so the generator's logic can be reused
//! from integration tests (`tests/smoke.rs`) without going through the CLI.
//!
//! ## Module map
//!
//! | module          | responsibility                                               |
//! |-----------------|--------------------------------------------------------------|
//! | [`config`]      | Resolved generator configuration (project name, schema, url).|
//! | [`idents`]      | Identifier helpers (snake_case → UpperCamel, keyword escape).|
//! | [`errors`]      | Typed [`GenError`] taxonomy + [`GenResult`] alias.           |
//! | [`logging`]     | `tracing` initialisation (verbosity + JSON toggle).          |
//! | [`ir`]          | IR v2 — Schema/Table/Column/ForeignKey/TableAnalysis.        |
//! | [`analysis`]    | Feature analysis v2 (timestamps, status, soft-delete, …).    |
//! | [`introspect`]  | Live PostgreSQL introspection (db-first).                    |
//! | [`fixture`]     | Built-in demo schema (no database required).                 |
//! | [`ddl`]         | DDL/migration SQL generation for the introspected schema.    |
//! | [`report`]      | On-disk generation report (`GENERATION_REPORT.md`).          |
//! | [`codegen`]     | Codegen orchestrator + per-layer emitters.                   |
//! | [`error_inventory`] | Machine-readable `error_inventory.json` schema + writer. |
//! | [`connection_profile`] | Multi-tenant Postgres connection-profile schema (closes G-C03). |

pub mod analysis;
pub mod codegen;
pub mod config;
pub mod connection_profile;
pub mod ddl;
pub mod error_inventory;
pub mod errors;
pub mod fixture;
pub mod idents;
pub mod introspect;
pub mod ir;
pub mod logging;
pub mod report;

// Convenience re-exports for downstream tests.
pub use errors::{GenError, GenResult};
pub use ir::Schema;
