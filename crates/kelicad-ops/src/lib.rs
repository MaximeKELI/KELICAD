//! Document operations, journal, and undo/redo.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod journal;
pub mod operation;
pub mod session;

#[cfg(test)]
mod rigorous;

pub use journal::OpJournal;
pub use operation::{OpError, Operation};
pub use session::DocSession;
