//! Ephemeral peer presence (not in CAD oplog).

use kelicad_doc::EntityId;
use kelicad_math::Vec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::envelope::ActorId;

/// Live presence for one peer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresenceState {
    /// Peer / actor id.
    pub peer_id: ActorId,
    /// Display name.
    pub name: String,
    /// RGB color.
    pub color: [u8; 3],
    /// Cursor in world coords.
    pub cursor: Option<Vec2>,
    /// Selected entity ids.
    pub selection: Vec<EntityId>,
}

impl PresenceState {
    /// New presence with a generated peer id.
    pub fn new(name: impl Into<String>, color: [u8; 3]) -> Self {
        Self {
            peer_id: ActorId(Uuid::now_v7()),
            name: name.into(),
            color,
            cursor: None,
            selection: Vec::new(),
        }
    }
}
