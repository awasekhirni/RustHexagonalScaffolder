//! Core domain primitives shared by every bounded context:
//! typed IDs, value objects, Clock, AggregateRoot, UnitOfWork, error codes,
//! bulkhead, shutdown, startup, saga, chaos, events.

pub mod aggregate;
pub mod bulkhead;
pub mod chaos;
pub mod clock;
pub mod error_codes;
pub mod events;
pub mod ids;
pub mod saga;
pub mod shutdown;
pub mod startup;
pub mod unit_of_work;
pub mod value_objects;
