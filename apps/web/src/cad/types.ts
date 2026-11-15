export type CadOutput = {
  prompt_fr?: string | null;
  prompt_en?: string | null;
  echoes: string[];
  document_changed: boolean;
  ghost_count: number;
  snap?: { x: number; y: number; kind: string } | null;
};

export type KeliPlanStep = {
  tool: string;
  summary_fr: string;
  summary_en: string;
  read_only: boolean;
};

export type KeliProposal = {
  utterance: string;
  steps: KeliPlanStep[];
  summary_fr: string;
  summary_en: string;
  ghost_count: number;
  has_mutations: boolean;
};

export type KeliAskResult = {
  proposal: KeliProposal;
  applied: boolean;
  echoes: string[];
};

export type KeliJournalEntry = {
  index: number;
  utterance: string;
  outcome: string;
  note: string;
};

export type CadSession = {
  entity_count(): number;
  submit_text(text: string): string;
  start_command(name: string): string;
  pick(x: number, y: number): string;
  hover(x: number, y: number): string;
  enter(): string;
  cancel(): string;
  undo(): string;
  redo(): string;
  set_snap_aperture(aperture: number): void;
  export_snapshot_json(): string;
  import_snapshot_json(json: string): void;
  export_dxf(): string;
  import_dxf(text: string): string;
  export_svg(width: number, height: number): string;
  scene_line_vertices(): Float32Array | number[];
  ghost_line_vertices(x: number, y: number): Float32Array | number[];
  selection_json(): string;
  prompt_fr(): string;
  keli_ask(utterance: string): string;
  keli_accept(): string;
  keli_reject(): string;
  keli_set_mode(mode: string): void;
  keli_proposal_json(): string;
  keli_journal_json(): string;
  keli_ghost_vertices(): Float32Array | number[];
};

export function parseCadOutput(raw: string): CadOutput {
  return JSON.parse(raw) as CadOutput;
}
