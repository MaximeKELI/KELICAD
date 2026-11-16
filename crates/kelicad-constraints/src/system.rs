//! Constraint graph storage.

use std::collections::HashMap;

use kelicad_doc::EntityId;

use crate::error::ConstraintError;
use crate::types::{Constraint, ConstraintId};

/// Collection of constraints for a drawing.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConstraintSystem {
    constraints: HashMap<ConstraintId, Constraint>,
    /// Named parameters for [`Constraint::FormulaDistance`] (e.g. `W` → 10.0).
    #[serde(default)]
    params: HashMap<String, f64>,
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

    /// Set / overwrite a named parameter.
    pub fn set_param(&mut self, name: impl Into<String>, value: f64) -> Result<(), ConstraintError> {
        if !value.is_finite() {
            return Err(ConstraintError::Invalid("param must be finite".into()));
        }
        self.params.insert(name.into(), value);
        Ok(())
    }

    /// Read a parameter.
    pub fn get_param(&self, name: &str) -> Option<f64> {
        self.params.get(name).copied()
    }

    /// All parameters.
    pub fn params(&self) -> &HashMap<String, f64> {
        &self.params
    }

    fn validate(&self, c: &Constraint) -> Result<(), ConstraintError> {
        match c {
            Constraint::Distance { value, .. } => {
                if !value.is_finite() || *value <= 0.0 {
                    return Err(ConstraintError::Invalid(
                        "distance value must be finite and > 0".into(),
                    ));
                }
            }
            Constraint::Angle { degrees, .. } => {
                if !degrees.is_finite() {
                    return Err(ConstraintError::Invalid("angle must be finite".into()));
                }
            }
            Constraint::FormulaDistance { formula, .. } => {
                if formula.trim().is_empty() {
                    return Err(ConstraintError::Invalid("empty formula".into()));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
