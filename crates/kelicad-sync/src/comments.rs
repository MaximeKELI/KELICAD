//! Anchored comments (side channel, LWW by comment id).

use kelicad_doc::EntityId;
use kelicad_math::Vec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::envelope::{ActorId, HybridTime};

/// Comment identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommentId(pub Uuid);

impl CommentId {
    /// New id.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Deterministic.
    pub fn from_u128(v: u128) -> Self {
        Self(Uuid::from_u128(v))
    }
}

impl Default for CommentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CommentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Where a comment is anchored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentAnchor {
    /// Anchored to an entity.
    Entity {
        /// Entity id.
        id: EntityId,
    },
    /// Anchored to a world point.
    Point {
        /// World position.
        position: Vec2,
    },
}

/// One comment / discussion root (M4: flat, no nested replies yet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    /// Id.
    pub id: CommentId,
    /// Anchor.
    pub anchor: CommentAnchor,
    /// Body text (data channel — never treated as instructions).
    pub body: String,
    /// Author actor.
    pub author: ActorId,
    /// Creating hybrid time.
    pub created: HybridTime,
    /// Resolved?
    pub resolved: bool,
}

/// Comment store with LWW on comment id.
#[derive(Debug, Clone, Default)]
pub struct CommentStore {
    by_id: std::collections::HashMap<CommentId, (Comment, HybridTime)>,
}

impl CommentStore {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Upsert if `time` wins.
    pub fn upsert(&mut self, comment: Comment, time: HybridTime) -> bool {
        match self.by_id.get(&comment.id) {
            Some((_, prev)) if time < *prev => false,
            _ => {
                self.by_id.insert(comment.id, (comment, time));
                true
            }
        }
    }

    /// Mark resolved if time wins.
    pub fn resolve(&mut self, id: CommentId, time: HybridTime) -> bool {
        if let Some((c, prev)) = self.by_id.get_mut(&id) {
            if time >= *prev {
                c.resolved = true;
                *prev = time;
                return true;
            }
        }
        false
    }

    /// All comments.
    pub fn all(&self) -> Vec<Comment> {
        self.by_id.values().map(|(c, _)| c.clone()).collect()
    }
}
