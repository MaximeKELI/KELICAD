//! Macro record / replay from the operation journal.

use kelicad_ops::{DocSession, Operation};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::PluginError;

/// One recorded macro step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroStep {
    /// Human label.
    pub label: String,
    /// Serialized operation.
    pub op: Operation,
}

/// Readable macro script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroScript {
    /// Format version.
    pub version: u32,
    /// Steps in order.
    pub steps: Vec<MacroStep>,
}

/// Export applied journal entries as a macro script.
pub fn record_macro(session: &DocSession) -> MacroScript {
    let mut steps = Vec::new();
    for (i, entry) in session.journal.applied().enumerate() {
        steps.push(MacroStep {
            label: format!("step_{i}"),
            op: entry.op.clone(),
        });
    }
    MacroScript { version: 1, steps }
}

/// Serialize macro to JSON text.
pub fn macro_to_json(script: &MacroScript) -> Result<String, PluginError> {
    serde_json::to_string_pretty(script).map_err(|e| PluginError::Execution(e.to_string()))
}

/// Parse macro JSON.
pub fn macro_from_json(text: &str) -> Result<MacroScript, PluginError> {
    serde_json::from_str(text).map_err(|e| PluginError::InvalidArgs(e.to_string()))
}

/// Replay macro onto a session (each step as local apply).
pub fn replay_macro(session: &mut DocSession, script: &MacroScript) -> Result<usize, PluginError> {
    let mut n = 0;
    for step in &script.steps {
        session
            .apply(step.op.clone())
            .map_err(|e| PluginError::Execution(e.to_string()))?;
        n += 1;
    }
    Ok(n)
}

/// Replay from a JSON value array of operations (batch-friendly).
pub fn replay_ops_json(session: &mut DocSession, ops: &Value) -> Result<usize, PluginError> {
    let arr = ops
        .as_array()
        .ok_or_else(|| PluginError::InvalidArgs("ops array".into()))?;
    let mut n = 0;
    for v in arr {
        let op: Operation = serde_json::from_value(v.clone())
            .map_err(|e| PluginError::InvalidArgs(e.to_string()))?;
        session
            .apply(op)
            .map_err(|e| PluginError::Execution(e.to_string()))?;
        n += 1;
    }
    Ok(n)
}
