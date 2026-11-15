//! Consultable AI journal (utterance, tools, outcome).

use serde::Serialize;

use crate::plan::PlanStep;

/// Outcome of a journaled interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalOutcome {
    /// Proposal created (Propose mode).
    Proposed,
    /// Mutations applied.
    Applied,
    /// Proposal rejected.
    Rejected,
    /// Read-only answer (no mutation).
    Answered,
    /// Planning / execution failed.
    Failed,
}

/// One AI journal entry.
#[derive(Debug, Clone, Serialize)]
pub struct AiJournalEntry {
    /// Monotonic index.
    pub index: u64,
    /// User utterance.
    pub utterance: String,
    /// Tool steps involved.
    pub steps: Vec<PlanStep>,
    /// Outcome.
    pub outcome: JournalOutcome,
    /// Optional error / note.
    pub note: String,
}

/// Append-only AI journal.
#[derive(Debug, Clone, Default)]
pub struct AiJournal {
    entries: Vec<AiJournalEntry>,
    next_index: u64,
}

impl AiJournal {
    /// Empty journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an entry.
    pub fn push(
        &mut self,
        utterance: String,
        steps: Vec<PlanStep>,
        outcome: JournalOutcome,
        note: impl Into<String>,
    ) {
        let index = self.next_index;
        self.next_index += 1;
        self.entries.push(AiJournalEntry {
            index,
            utterance,
            steps,
            outcome,
            note: note.into(),
        });
    }

    /// All entries.
    pub fn entries(&self) -> &[AiJournalEntry] {
        &self.entries
    }

    /// Serialize as JSON array.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.entries)
    }
}
