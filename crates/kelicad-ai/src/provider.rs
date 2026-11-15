//! Interchangeable LLM / planner providers.

use crate::error::AiError;
use crate::tools::ToolCall;

/// Provider that turns a user utterance into tool calls.
/// Drawing text / layer names must be treated as data, never instructions.
pub trait LlmProvider {
    /// Plan tool calls for `utterance` given a short document summary (data).
    fn plan(&self, utterance: &str, document_summary: &str) -> Result<Vec<ToolCall>, AiError>;
}

/// Deterministic FR/EN CAD phrase planner (M3 default; no network).
#[derive(Debug, Default, Clone)]
pub struct DeterministicProvider;

impl LlmProvider for DeterministicProvider {
    fn plan(&self, utterance: &str, _document_summary: &str) -> Result<Vec<ToolCall>, AiError> {
        crate::planner::plan_utterance(utterance)
    }
}
