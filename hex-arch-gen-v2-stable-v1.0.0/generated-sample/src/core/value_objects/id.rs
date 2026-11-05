//! Generic typed ID value object — for entities without a dedicated ID type.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Generic typed identifier wrapping a `Uuid`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(transparent)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Generates a new random ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    /// Returns the inner [`Uuid`].
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for EntityId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<EntityId> for Uuid {
    fn from(id: EntityId) -> Self {
        id.0
    }
}
