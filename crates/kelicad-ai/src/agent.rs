//! KeliAgent: ask → propose → accept/reject (or Apply mode).

use kelicad_doc::{EntityId, Geometry};
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use serde::Serialize;
use serde_json::json;

use crate::error::AiError;
use crate::ghost::ghosts_to_vertices;
use crate::id_gen::{IdGen, SequentialIdGen, UuidV7Gen};
use crate::journal::{AiJournal, JournalOutcome};
use crate::plan::{AgentMode, PlanStep, Proposal};
use crate::provider::{DeterministicProvider, LlmProvider};
use crate::tools::{execute_tool, ToolCall, ToolContext};

/// Result returned to UI after `ask`.
#[derive(Debug, Clone, Serialize)]
pub struct AskResult {
    /// Proposal summary (also stored as pending when Propose).
    pub proposal: ProposalDto,
    /// True if already applied (Apply mode or read-only).
    pub applied: bool,
    /// Echo lines (French).
    pub echoes: Vec<String>,
}

/// Serializable proposal (without ops).
#[derive(Debug, Clone, Serialize)]
pub struct ProposalDto {
    /// Utterance.
    pub utterance: String,
    /// Steps.
    pub steps: Vec<PlanStep>,
    /// French summary.
    pub summary_fr: String,
    /// English summary.
    pub summary_en: String,
    /// Ghost vertex count.
    pub ghost_count: usize,
    /// Whether accept would mutate.
    pub has_mutations: bool,
}

impl From<&Proposal> for ProposalDto {
    fn from(p: &Proposal) -> Self {
        Self {
            utterance: p.utterance.clone(),
            steps: p.steps.clone(),
            summary_fr: p.summary_fr.clone(),
            summary_en: p.summary_en.clone(),
            ghost_count: p.ghosts.len(),
            has_mutations: p.has_mutations(),
        }
    }
}

/// KELI agent bound to a document session.
pub struct KeliAgent<P: LlmProvider = DeterministicProvider, I: IdGen = UuidV7Gen> {
    /// Planner / LLM.
    pub provider: P,
    /// Id generator for planned entities.
    pub ids: I,
    /// Propose (default) or Apply.
    pub mode: AgentMode,
    /// Pending proposal.
    pending: Option<Proposal>,
    /// AI journal.
    pub journal: AiJournal,
    /// Selection snapshot used by tools.
    pub selection: Vec<EntityId>,
    /// View center.
    pub view_center: Vec2,
    /// View height.
    pub view_height: f64,
}

impl Default for KeliAgent<DeterministicProvider, UuidV7Gen> {
    fn default() -> Self {
        Self::new(DeterministicProvider, UuidV7Gen)
    }
}

impl KeliAgent<DeterministicProvider, SequentialIdGen> {
    /// Agent with deterministic ids (evals / tests).
    pub fn for_evals() -> Self {
        Self::new(DeterministicProvider, SequentialIdGen::new(1_000))
    }
}

impl<P: LlmProvider, I: IdGen> KeliAgent<P, I> {
    /// Create agent.
    pub fn new(provider: P, ids: I) -> Self {
        Self {
            provider,
            ids,
            mode: AgentMode::Propose,
            pending: None,
            journal: AiJournal::new(),
            selection: Vec::new(),
            view_center: Vec2::new(0.0, 0.0),
            view_height: 20.0,
        }
    }

    /// Set interaction mode.
    pub fn set_mode(&mut self, mode: AgentMode) {
        self.mode = mode;
    }

    /// Pending proposal reference.
    pub fn pending(&self) -> Option<&Proposal> {
        self.pending.as_ref()
    }

    /// Ghost vertices for the pending proposal.
    pub fn ghost_vertices(&self) -> Vec<f32> {
        self.pending
            .as_ref()
            .map(|p| ghosts_to_vertices(&p.ghosts))
            .unwrap_or_default()
    }

    /// Ask KELI to plan (and maybe apply) for `utterance`.
    pub fn ask(&mut self, session: &mut DocSession, utterance: &str) -> Result<AskResult, AiError> {
        let summary = document_summary(session);
        let calls = self.provider.plan(utterance, &summary)?;
        let proposal = self.build_proposal(session, utterance, &calls)?;

        let dto = ProposalDto::from(&proposal);
        let mut echoes = vec![proposal.summary_fr.clone()];

        if !proposal.has_mutations() {
            self.journal.push(
                utterance.to_string(),
                proposal.steps.clone(),
                JournalOutcome::Answered,
                "read-only",
            );
            self.pending = None;
            return Ok(AskResult {
                proposal: dto,
                applied: false,
                echoes,
            });
        }

        match self.mode {
            AgentMode::Propose => {
                self.journal.push(
                    utterance.to_string(),
                    proposal.steps.clone(),
                    JournalOutcome::Proposed,
                    "awaiting accept",
                );
                self.pending = Some(proposal);
                echoes.push("Mode Proposer — Valider pour appliquer.".into());
                Ok(AskResult {
                    proposal: dto,
                    applied: false,
                    echoes,
                })
            }
            AgentMode::Apply => {
                self.pending = Some(proposal);
                self.accept(session)?;
                echoes.push("Mode Appliquer — plan exécuté.".into());
                Ok(AskResult {
                    proposal: dto,
                    applied: true,
                    echoes,
                })
            }
        }
    }

    /// Accept pending proposal as one undoable batch.
    pub fn accept(&mut self, session: &mut DocSession) -> Result<(), AiError> {
        let proposal = self.pending.take().ok_or(AiError::NoProposal)?;
        if proposal.ops.is_empty() {
            self.journal.push(
                proposal.utterance.clone(),
                proposal.steps.clone(),
                JournalOutcome::Answered,
                "nothing to apply",
            );
            return Ok(());
        }
        let batch = Operation::Batch {
            ops: proposal.ops.clone(),
        };
        session
            .apply(batch)
            .map_err(|e| AiError::Op(e.to_string()))?;
        self.journal.push(
            proposal.utterance,
            proposal.steps,
            JournalOutcome::Applied,
            "batch applied",
        );
        Ok(())
    }

    /// Reject pending proposal.
    pub fn reject(&mut self) -> Result<(), AiError> {
        let proposal = self.pending.take().ok_or(AiError::NoProposal)?;
        self.journal.push(
            proposal.utterance,
            proposal.steps,
            JournalOutcome::Rejected,
            "user rejected",
        );
        Ok(())
    }

    /// Clear pending without journaling rejection.
    pub fn clear_pending(&mut self) {
        self.pending = None;
    }

    fn build_proposal(
        &mut self,
        session: &DocSession,
        utterance: &str,
        calls: &[ToolCall],
    ) -> Result<Proposal, AiError> {
        // Simulate sequential writes on a scratch document so later tools
        // (e.g. move) see entities created earlier in the same plan.
        let mut scratch = session.clone();
        let mut steps = Vec::new();
        let mut all_ops = Vec::new();
        let mut ghosts: Vec<Geometry> = Vec::new();
        let mut planned_ids: Vec<EntityId> = Vec::new();

        for call in calls {
            let selection = if self.selection.is_empty() && !planned_ids.is_empty() {
                planned_ids.as_slice()
            } else if self.selection.is_empty() {
                // Prefer entities created in this plan; else all doc entities.
                &[][..]
            } else {
                self.selection.as_slice()
            };

            // For move/copy/rotate after creates in-plan, target planned ids.
            let mut call = call.clone();
            if matches!(
                call.name.as_str(),
                "move" | "copy" | "rotate" | "scale" | "set_layer" | "delete_entities"
            ) && call.args.get("entity_ids").is_none()
                && !planned_ids.is_empty()
            {
                let ids: Vec<String> = planned_ids.iter().map(|id| id.to_string()).collect();
                let mut args = call.args.as_object().cloned().unwrap_or_default();
                args.insert("entity_ids".into(), json!(ids));
                call.args = serde_json::Value::Object(args);
            }

            let ctx = ToolContext {
                session: &scratch,
                selection,
                view_center: self.view_center,
                view_height: self.view_height,
            };
            let exec = execute_tool(&call, &ctx, &mut self.ids)?;
            for id in exec.bindings.values() {
                planned_ids.push(*id);
            }
            // Apply planned ops to scratch so subsequent tools see them.
            if !exec.ops.is_empty() {
                let batch = Operation::Batch {
                    ops: exec.ops.clone(),
                };
                scratch
                    .apply(batch)
                    .map_err(|e| AiError::Op(e.to_string()))?;
            }
            all_ops.extend(exec.ops);
            ghosts.extend(exec.ghosts);
            steps.push(exec.step);
        }

        let summary_fr = steps
            .iter()
            .map(|s| s.summary_fr.as_str())
            .collect::<Vec<_>>()
            .join(" → ");
        let summary_en = steps
            .iter()
            .map(|s| s.summary_en.as_str())
            .collect::<Vec<_>>()
            .join(" → ");

        Ok(Proposal {
            utterance: utterance.to_string(),
            steps,
            ops: all_ops,
            ghosts,
            summary_fr,
            summary_en,
        })
    }
}

fn document_summary(session: &DocSession) -> String {
    format!(
        "entities={} layers={}",
        session.document.entity_count(),
        session.document.layers().count()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id_gen::SequentialIdGen;
    use crate::provider::DeterministicProvider;

    #[test]
    fn acceptance_propose_accept_undo() {
        let mut session = DocSession::new();
        let mut agent = KeliAgent::new(DeterministicProvider, SequentialIdGen::new(100));
        agent.set_mode(AgentMode::Propose);
        let u = "dessine un rectangle de 4 m × 3 m, cote-le et déplace-le de 2 m vers la droite";
        let ask = agent.ask(&mut session, u).unwrap();
        assert!(!ask.applied);
        assert!(ask.proposal.has_mutations);
        assert_eq!(session.document.entity_count(), 0);

        agent.accept(&mut session).unwrap();
        assert_eq!(session.document.entity_count(), 2); // rect + dim

        // Rectangle should be shifted by +2 in x.
        let mut found_poly = false;
        for e in session.document.entities() {
            if let Geometry::Polyline(p) = &e.geometry {
                found_poly = true;
                assert!((p.vertices[0].x - 2.0).abs() < 1e-9);
                assert!((p.vertices[1].x - 6.0).abs() < 1e-9);
            }
            if let Geometry::Dimension(d) = &e.geometry {
                assert!((d.measured_length() - 4.0).abs() < 1e-9);
                assert!((d.def1.x - 2.0).abs() < 1e-9);
            }
        }
        assert!(found_poly);

        assert!(session.undo().unwrap());
        assert_eq!(session.document.entity_count(), 0);
    }

    #[test]
    fn apply_mode_auto_commits() {
        let mut session = DocSession::new();
        let mut agent = KeliAgent::new(DeterministicProvider, SequentialIdGen::new(200));
        agent.set_mode(AgentMode::Apply);
        let ask = agent
            .ask(&mut session, "dessine un cercle de rayon 1")
            .unwrap();
        assert!(ask.applied);
        assert_eq!(session.document.entity_count(), 1);
    }
}
