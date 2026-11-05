//! `AggregateRoot` — marker trait for entities that act as consistency
//! boundaries. Every per-table entity implements this.

/// Marker trait — entities that are aggregate roots.
///
/// In the generated codebase, every entity is its own aggregate root
/// (single-table bounded context). Real applications group related tables
/// into a single aggregate and expose only the root through the repository.
pub trait AggregateRoot: Send + Sync {}

/// Blanket implementation: any `Send + Sync + 'static` type is an
/// aggregate root in the generated code.
impl<T: Send + Sync> AggregateRoot for T {}
