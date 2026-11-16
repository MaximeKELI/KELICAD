//! 2D parametric constraint graph and solver (ADR 0010).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod error;
mod expr;
mod solver;
mod system;
mod types;

#[cfg(test)]
mod rigorous;

pub use error::ConstraintError;
pub use expr::eval_formula;
pub use solver::{solve, SolveResult, SolveStatus};
pub use system::ConstraintSystem;
pub use types::{Constraint, ConstraintId, Endpoint};
