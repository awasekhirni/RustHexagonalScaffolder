//! Typed IDs — newtype wrappers around `Uuid` so the compiler enforces
//! "you can't pass a UserId where an OrderId is expected".

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Macro that declares a typed ID newtype.
macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema,
        )]
        #[serde(transparent)]
        #[allow(clippy::module_name_repetitions)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generates a fresh random ID.
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

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}:{}", $prefix, self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                // Accept either the bare UUID or the prefixed form.
                let bare = s.strip_prefix(concat!($prefix, ":")).unwrap_or(s);
                Ok(Self(Uuid::parse_str(bare)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

// Typed IDs declared per known entity. For unknown entities the generated
// code falls back to raw `Uuid`.
typed_id!(UserId, "user");
typed_id!(OrderId, "order");
typed_id!(TenantId, "tenant");
