//! Constraint graph storage.

use std::collections::HashMap;

use kelicad_doc::EntityId;

use crate::error::ConstraintError;
use crate::types::{Constraint, ConstraintId};

/// Collection of constraints for a drawing.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConstraintSystem {
    constraints: HashMap<ConstraintId, Constraint>,
}

impl ConstraintSystem {
    /// Empty system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of constraints.
    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Iterate constraints.
    pub fn iter(&self) -> impl Iterator<Item = (ConstraintId, &Constraint)> {
        self.constraints.iter().map(|(k, v)| (*k, v))
    }

    /// Add a constraint with a fresh id.
    pub fn add(&mut self, constraint: Constraint) -> Result<ConstraintId, ConstraintError> {
        self.validate(&constraint)?;
        let id = ConstraintId::new();
        self.constraints.insert(id, constraint);
        Ok(id)
    }

    /// Add with an explicit id (tests / restore).
    pub fn insert(
        &mut self,
        id: ConstraintId,
        constraint: Constraint,
    ) -> Result<(), ConstraintError> {
        self.validate(&constraint)?;
        self.constraints.insert(id, constraint);
        Ok(())
    }

    /// Remove by id.
    pub fn remove(&mut self, id: ConstraintId) -> Option<Constraint> {
        self.constraints.remove(&id)
    }

    /// Get by id.
    pub fn get(&self, id: ConstraintId) -> Option<&Constraint> {
        self.constraints.get(&id)
    }

    /// All constraints as owned vec.
    pub fn constraints(&self) -> Vec<(ConstraintId, Constraint)> {
        self.constraints
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }

    /// Entities referenced by any constraint.
    pub fn touched_entities(&self) -> Vec<EntityId> {
        let mut ids: Vec<_> = self
            .constraints
            .values()
            .flat_map(|c| c.entity_ids())
            .collect();
        ids.sort_by_key(|e| e.as_uuid());
        ids.dedup();
        ids
    }

    fn validate(&self, c: &Constraint) -> Result<(), ConstraintError> {
        if let Constraint::Distance { value, .. } = c {
            if !value.is_finite() || *value <= 0.0 {
                return Err(ConstraintError::Invalid(
                    "distance value must be finite and > 0".into(),
                ));
            }
        }
        Ok(())
    }
}
