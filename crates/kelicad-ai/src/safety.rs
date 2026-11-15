//! Non-negotiable safety guards for KELI tools.

/// Maximum entities deletable without explicit confirmation.
pub const DESTRUCTIVE_DELETE_THRESHOLD: usize = 20;

/// Returns true if deleting `count` entities requires confirmation.
pub fn requires_destructive_confirm(count: usize) -> bool {
    count > DESTRUCTIVE_DELETE_THRESHOLD
}
