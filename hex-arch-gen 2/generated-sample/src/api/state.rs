//! Shared application state handed to every Axum handler.
//!
//! Handlers never touch repositories or services directly — they dispatch
//! through these two buses, which is what keeps the hexagon upright.

use std::sync::Arc;

use crate::enterprise::bus::{CommandBus, QueryBus};

/// Dependencies shared across the whole HTTP surface.
#[derive(Clone)]
pub struct AppState {
    /// Write-side dispatch (commands).
    pub cmd: Arc<CommandBus>,
    /// Read-side dispatch (queries).
    pub qry: Arc<QueryBus>,
}
