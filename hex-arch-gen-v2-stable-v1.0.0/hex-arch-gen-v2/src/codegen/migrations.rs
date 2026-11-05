// Copyright 2025 β ORI Inc. Canada. All Rights Reserved.
// Author: Awase Khirni Syed
//! Migration file emission. Delegates the SQL rendering to [`crate::ddl`].
//!
//! Kept as a separate codegen module so the orchestrator's call graph is
//! uniform (`project_files`, `assets`, `migrations`, then per-table layers).

use std::path::Path;

use crate::ddl;
use crate::errors::GenResult;
use crate::ir::Schema;

/// Writes the DDL migration under `<root>/migrations/`.
pub fn write_all(schema: &Schema, root: &Path) -> GenResult<()> {
    ddl::write_migration(schema, root)
}
