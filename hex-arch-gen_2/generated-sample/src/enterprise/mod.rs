//! Enterprise integration patterns shared by every bounded context.
//!
//! * [`bus`]        — in-process Command/Query dispatch (Enterprise Bus)
//! * [`repository`] — the generic repository contract (Enterprise Repository)

pub mod bus;
pub mod repository;
