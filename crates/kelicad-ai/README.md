# kelicad-ai

KELI copilote core: JSON tool schemas, validated tool execution, Propose/Apply
agent, deterministic FR/EN planner, and AI journal.

KELI never mutates the document directly — write tools produce `Operation`
plans applied through `DocSession`.
