//! KELI copilote: tool calling, Propose/Apply, journal, deterministic planner.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod agent;
pub mod error;
pub mod ghost;
pub mod id_gen;
pub mod journal;
pub mod plan;
pub mod planner;
pub mod provider;
pub mod ruleset;
pub mod safety;
pub mod tools;

#[cfg(test)]
mod rigorous;

pub use agent::{AskResult, KeliAgent, ProposalDto};
pub use error::AiError;
pub use ghost::ghosts_to_vertices;
pub use id_gen::{IdGen, SequentialIdGen, UuidV7Gen};
pub use journal::{AiJournal, AiJournalEntry, JournalOutcome};
pub use plan::{AgentMode, PlanStep, Proposal};
pub use provider::{DeterministicProvider, LlmProvider};
pub use ruleset::{load_ruleset, run_ruleset, CheckFinding, CheckReport, Ruleset};
pub use tools::{execute_tool, tool_names, ToolCall, ToolContext, ToolExecResult};
