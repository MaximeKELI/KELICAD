//! WebSocket JSON message types (ADR 0008).

use serde::{Deserialize, Serialize};

use crate::comments::Comment;
use crate::envelope::OpEnvelope;
use crate::presence::PresenceState;
use crate::version::RestorePoint;

/// Client ↔ gateway message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WireMessage {
    /// Initial hello after WS connect.
    Hello {
        /// Presence bootstrap.
        state: PresenceState,
    },
    /// Full room snapshot after join.
    Snapshot {
        /// Ordered envelopes.
        ops: Vec<OpEnvelope>,
        /// Comments.
        comments: Vec<Comment>,
        /// Named versions.
        versions: Vec<RestorePoint>,
    },
    /// CAD operation envelope.
    Op {
        /// Envelope.
        envelope: OpEnvelope,
    },
    /// Presence update.
    Presence {
        /// State.
        state: PresenceState,
    },
    /// Comment upsert.
    CommentUpsert {
        /// Comment.
        comment: Comment,
    },
    /// Comment resolve.
    CommentResolve {
        /// Comment id string.
        id: String,
        /// Lamport of resolve.
        lamport: u64,
        /// Actor.
        actor_id: String,
    },
    /// Save version on server.
    VersionSave {
        /// Point.
        point: RestorePoint,
    },
    /// Keepalive.
    Ping,
}
