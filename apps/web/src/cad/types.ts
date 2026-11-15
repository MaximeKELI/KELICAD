export type CadOutput = {
  prompt_fr?: string | null;
  prompt_en?: string | null;
  echoes: string[];
  document_changed: boolean;
  ghost_count: number;
  snap?: { x: number; y: number; kind: string } | null;
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
  scene_line_vertices(): Float32Array | number[];
  ghost_line_vertices(x: number, y: number): Float32Array | number[];
  selection_json(): string;
  prompt_fr(): string;
};

export function parseCadOutput(raw: string): CadOutput {
  return JSON.parse(raw) as CadOutput;
}
