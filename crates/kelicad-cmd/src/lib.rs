//! Command engine for KELICAD.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod engine;
pub mod id_gen;
pub mod input;
pub mod registry;
pub mod selection;

pub use engine::{CommandEngine, CommandEvent, CommandOutput, EnginePrompt};
pub use id_gen::{IdGen, SequentialIdGen, UuidV7Gen};
pub use input::{parse_point, InputError, ParsedPoint};
pub use registry::CommandId;
pub use selection::Selection;
