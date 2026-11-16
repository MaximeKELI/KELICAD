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

export type KeliImpulse = {
  label_fr: string;
  label_en: string;
  utterance: string;
  category: string;
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
  keli_impulses_json(): string;
  sync_set_identity(name: string, r: number, g: number, b: number): void;
  sync_actor_id(): string;
  sync_publish_local(): string;
  sync_drain_outbox(): string;
  sync_integrate(envelope_json: string): string;
  sync_catch_up(ops_json: string): number;
  sync_hello_json(): string;
  sync_set_presence(x: number, y: number, has_cursor: boolean): string;
  sync_upsert_peer(json: string): void;
  sync_peers_json(): string;
  sync_add_comment_at(x: number, y: number, body: string): string;
  sync_comments_json(): string;
  sync_resolve_comment(id: string): boolean;
  sync_upsert_comment_json(json: string): void;
  sync_save_version(name: string): string;
  sync_versions_json(): string;
  sync_diff_versions(a: string, b: string): string;
  sync_fingerprint(): string;
  sync_handle_wire(text: string): string;
  macro_record_json(): string;
  constraint_add_json(json: string): string;
  constraint_solve(): string;
  constraints_json(): string;
  vectorize_bytes(data: Uint8Array, filename_hint: string): number;
  solid_extrude_selected(height: number): string;
  solid_sphere(cx: number, cy: number, cz: number, radius: number): string;
  solid_cone(cx: number, cy: number, radius: number, height: number): string;
  solid_cylinder(cx: number, cy: number, radius: number, height: number): string;
  solid_torus(
    cx: number,
    cy: number,
    cz: number,
    major: number,
    minor: number,
  ): string;
  solid_pyramid(
    cx: number,
    cy: number,
    radius: number,
    height: number,
    sides: number,
  ): string;
  solid_prism(
    cx: number,
    cy: number,
    radius: number,
    height: number,
    sides: number,
  ): string;
  solid_pipe(
    cx: number,
    cy: number,
    outer: number,
    inner: number,
    height: number,
  ): string;
  solid_revolve_selected(degrees: number): string;
  solid_sweep_selected(thickness: number): string;
  solid_loft_selected(height: number): string;
  view_derive_apply(kind: string): number;
  bim_schedule_csv(): string;
  ifc_export(): string;
  solid_count(): number;
};

export function parseCadOutput(raw: string): CadOutput {
  return JSON.parse(raw) as CadOutput;
}
