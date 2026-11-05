//! Infrastructure layer — outbound adapters (SQLx/PostgreSQL + in-process bus).
//!
//! Each adapter implements the repository port of its domain context;
//! nothing in this layer is referenced by the domain or application code
//! except through `main`'s constructor wiring.
//!
//! v0.4.0 adds `stmt_cache` (closes G-H05) and `n_plus_one` (closes G-H06).

pub mod bus;
pub mod db;
pub mod n_plus_one;
pub mod persistence;
pub mod stmt_cache;
pub mod timed_query;
