# ADR 0007 — KELI tool calling and Propose mode

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

M3 requires the KELI copilote: natural-language CAD assistance with undoable,
auditable mutations. The charter forbids the LLM from writing the document
directly and requires Propose-by-default, measurements by the core, and an
interchangeable model provider.

## Decision

1. **Crate `kelicad-ai`:** tool registry with strict JSON schemas validated in
   Rust; read tools return JSON; write tools emit `Operation` plans only.
2. **Propose / Apply:** default mode builds a `Proposal` (steps + ghost
   geometries) without mutating. Accept commits a single `Operation::Batch`
   so one Undo reverses the whole plan. Apply mode auto-accepts after plan.
3. **Provider:** `LlmProvider` trait. M3 ships `DeterministicProvider` (FR/EN
   CAD phrase → tool calls) for CI, evals, and offline UI. Cloud/local LLMs
   plug in later; API keys never live in the browser client.
4. **Channel separation:** drawing text, layer names, and attributes are
   treated as *data* in prompts, never as instructions (documented for future
   LLM providers).
5. **Destructive guard:** deleting more than `N = 20` entities requires
   `confirm_destructive: true` or the tool call is rejected.
6. **Journal:** every ask / accept / reject is recorded in `AiJournal`
   (utterance, tools, outcome).

## Consequences

- All AI mutations remain undoable and rights-checkable like manual commands.
- Evals run without network; ≥90 % on 100 corpus cases gates CI.
- Real LLM quality depends on future providers; tool surface stays stable.
