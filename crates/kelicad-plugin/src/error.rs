//! Plugin host errors.

use thiserror::Error;

/// Errors from the plugin host / guest.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PluginError {
    /// Permission denied.
    #[error("denied: {0}")]
    Denied(String),
    /// Unknown hostcall.
    #[error("unknown hostcall: {0}")]
    UnknownHostcall(String),
    /// Invalid arguments.
    #[error("invalid args: {0}")]
    InvalidArgs(String),
    /// Hostcall budget exhausted.
    #[error("hostcall budget exceeded")]
    BudgetExceeded,
    /// Document / op error.
    #[error("execution: {0}")]
    Execution(String),
    /// Guest failed.
    #[error("guest: {0}")]
    Guest(String),
    /// WASM runtime (feature-gated).
    #[error("wasm: {0}")]
    Wasm(String),
}
