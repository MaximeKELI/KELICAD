/**
 * Optional Tauri desktop bridge (native open/save `.kelicad`).
 * No-ops gracefully when running in the browser.
 */

export type NativeIoResult = {
  path: string;
  snapshot_json: string;
  entities: number;
  blocks: number;
};

function isTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    // Tauri 2 injects __TAURI_INTERNALS__
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

export function desktopAvailable(): boolean {
  return isTauri();
}

export async function desktopOpenKelicad(): Promise<NativeIoResult | null> {
  if (!isTauri()) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const { invoke } = await import("@tauri-apps/api/core");
  const selected = await open({
    multiple: false,
    filters: [{ name: "KELICAD", extensions: ["kelicad"] }],
  });
  if (!selected || Array.isArray(selected)) return null;
  return invoke<NativeIoResult>("open_kelicad", { path: selected });
}

export async function desktopSaveKelicad(
  snapshotJson: string,
  suggestedName = "dessin.kelicad",
): Promise<NativeIoResult | null> {
  if (!isTauri()) return null;
  const { save } = await import("@tauri-apps/plugin-dialog");
  const { invoke } = await import("@tauri-apps/api/core");
  const path = await save({
    defaultPath: suggestedName,
    filters: [{ name: "KELICAD", extensions: ["kelicad"] }],
  });
  if (!path) return null;
  return invoke<NativeIoResult>("save_kelicad", {
    path,
    snapshotJson,
  });
}

export async function desktopPing(): Promise<string | null> {
  if (!isTauri()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("core_ping");
}
