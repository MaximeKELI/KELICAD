//! Errors for KELI tooling and agent.

use thiserror::Error;

/// KELI / tool-calling errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AiError {
    /// Unknown tool name.
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    /// JSON schema / argument validation failed.
    #[error("invalid tool arguments for {tool}: {reason}")]
    InvalidArgs {
        /// Tool name.
        tool: String,
        /// Human-readable reason.
        reason: String,
    },
    /// Destructive action blocked without confirmation.
    #[error("destructive action blocked ({0}); set confirm_destructive=true")]
    DestructiveBlocked(String),
    /// Tool could not run against current document.
    #[error("tool execution failed: {0}")]
    Execution(String),
    /// No pending proposal to accept/reject.
    #[error("no pending proposal")]
    NoProposal,
    /// Planner could not interpret the utterance.
    #[error("could not plan for utterance: {0}")]
    Unplanned(String),
    /// Operation apply error.
    #[error("operation failed: {0}")]
    Op(String),
}
