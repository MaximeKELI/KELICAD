//! Immutable append-only operation journal with undo cursor.

use crate::operation::Operation;

/// Journal entry: forward op as committed by the user.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalEntry {
    /// Forward operation.
    pub op: Operation,
    /// Inverse captured at apply time.
    pub inverse: Operation,
}

/// Operation journal. Undo moves the cursor backward and applies inverses;
/// new commits truncate any redo branch.
#[derive(Debug, Clone, Default)]
pub struct OpJournal {
    entries: Vec<JournalEntry>,
    /// Number of committed entries that are currently applied (undo cursor).
    cursor: usize,
}

impl OpJournal {
    /// Empty journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of applied entries.
    pub fn len_applied(&self) -> usize {
        self.cursor
    }

    /// Total entries including undone redo branch.
    pub fn len_total(&self) -> usize {
        self.entries.len()
    }

    /// True if undo is possible.
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// True if redo is possible.
    pub fn can_redo(&self) -> bool {
        self.cursor < self.entries.len()
    }

    /// Record a newly applied op (truncates redo).
    pub fn commit(&mut self, op: Operation, inverse: Operation) {
        self.entries.truncate(self.cursor);
        self.entries.push(JournalEntry { op, inverse });
        self.cursor = self.entries.len();
    }

    /// Inverse of the last applied op, and move cursor back.
    pub fn undo_inverse(&mut self) -> Option<Operation> {
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        Some(self.entries[self.cursor].inverse.clone())
    }

    /// Forward op to redo, and advance cursor.
    pub fn redo_forward(&mut self) -> Option<Operation> {
        if self.cursor >= self.entries.len() {
            return None;
        }
        let op = self.entries[self.cursor].op.clone();
        self.cursor += 1;
        Some(op)
    }

    /// Applied entries in order (for audit / sync later).
    pub fn applied(&self) -> impl Iterator<Item = &JournalEntry> {
        self.entries[..self.cursor].iter()
    }
}
