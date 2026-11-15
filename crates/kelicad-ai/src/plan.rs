//! Proposal / plan types produced before document mutation.

use kelicad_doc::Geometry;
use kelicad_ops::Operation;
use serde::Serialize;
use serde_json::Value;

/// One tool call in a proposal.
#[derive(Debug, Clone, Serialize)]
pub struct PlanStep {
    /// Tool name.
    pub tool: String,
    /// Validated arguments.
    pub args: Value,
    /// Short French description for the UI.
    pub summary_fr: String,
    /// Short English description.
    pub summary_en: String,
    /// True if this step only reads.
    pub read_only: bool,
    /// Read result JSON when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_result: Option<Value>,
}

/// Pending proposal awaiting accept/reject.
#[derive(Debug, Clone, Serialize)]
pub struct Proposal {
    /// Original user utterance (data channel).
    pub utterance: String,
    /// Ordered steps.
    pub steps: Vec<PlanStep>,
    /// Operations to apply as one batch on accept (empty for read-only).
    #[serde(skip)]
    pub ops: Vec<Operation>,
    /// Ghost geometries for preview.
    #[serde(skip)]
    pub ghosts: Vec<Geometry>,
    /// French plan summary.
    pub summary_fr: String,
    /// English plan summary.
    pub summary_en: String,
}

impl Proposal {
    /// True if accepting would mutate the document.
    pub fn has_mutations(&self) -> bool {
        !self.ops.is_empty()
    }
}

/// Agent interaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Plan + ghost; wait for accept.
    #[default]
    Propose,
    /// Plan then auto-apply batch.
    Apply,
}
