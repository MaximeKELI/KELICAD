import { useMemo } from "react";

import type { CadSession } from "../cad/types";

type SelItem = {
  id: string;
  kind: string;
  layer?: string;
  bbox?: { min: [number, number]; max: [number, number] };
};

type Props = {
  cad: CadSession;
  revision: number;
  onEcho: (lines: string[]) => void;
  onChanged: () => void;
};

export function SelectionPanel({ cad, revision, onEcho, onChanged }: Props) {
  const items = useMemo(() => {
    void revision;
    try {
      return JSON.parse(cad.selection_detail_json()) as SelItem[];
    } catch {
      return [];
    }
  }, [cad, revision]);

  return (
    <div className="border-t border-ink-700/60 p-2">
      <div className="mb-1 flex items-center justify-between">
        <p className="text-[10px] uppercase tracking-wider text-ink-500">
          Sélection
        </p>
        <button
          type="button"
          className="font-mono text-[10px] text-ink-500 hover:text-ink-200"
          onClick={() => {
            try {
              cad.select_clear();
              onChanged();
              onEcho(["Sélection vidée"]);
            } catch {
              /* ignore */
            }
          }}
        >
          CLR
        </button>
      </div>
      {items.length === 0 ? (
        <p className="font-mono text-[10px] text-ink-600">
          Clique / Shift+clic / fenêtre
        </p>
      ) : (
        <ul className="max-h-36 space-y-1 overflow-auto font-mono text-[10px]">
          {items.map((it) => (
            <li
              key={it.id}
              className="rounded border border-ink-800/80 px-1.5 py-1 text-ink-300"
            >
              <div className="flex justify-between gap-2">
                <span className="text-accent-soft">{it.kind}</span>
                <span className="truncate text-ink-600">
                  {it.layer ?? "—"}
                </span>
              </div>
              <div className="truncate text-ink-600">{it.id.slice(0, 8)}…</div>
            </li>
          ))}
        </ul>
      )}
      <p className="mt-1 text-[10px] text-ink-600">{items.length} objet(s)</p>
    </div>
  );
}
