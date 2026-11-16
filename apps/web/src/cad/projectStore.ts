/** Local project helpers: autosave + recent files (browser localStorage). */

const AUTOSAVE_KEY = "kelicad.autosave.v1";
const RECENT_KEY = "kelicad.recent.v1";
const MAX_RECENT = 8;

export type RecentEntry = {
  name: string;
  savedAt: number;
  snapshotJson: string;
};

export function saveAutosave(snapshotJson: string): void {
  try {
    localStorage.setItem(
      AUTOSAVE_KEY,
      JSON.stringify({ savedAt: Date.now(), snapshotJson }),
    );
  } catch {
    /* quota */
  }
}

export function loadAutosave(): string | null {
  try {
    const raw = localStorage.getItem(AUTOSAVE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { snapshotJson?: string };
    return parsed.snapshotJson ?? null;
  } catch {
    return null;
  }
}

export function pushRecent(name: string, snapshotJson: string): void {
  try {
    const prev = listRecent().filter((e) => e.name !== name);
    const next: RecentEntry[] = [
      { name, savedAt: Date.now(), snapshotJson },
      ...prev,
    ].slice(0, MAX_RECENT);
    localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  } catch {
    /* quota */
  }
}

export function listRecent(): RecentEntry[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as RecentEntry[];
  } catch {
    return [];
  }
}
