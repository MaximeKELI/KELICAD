import { useCallback, useEffect, useState } from "react";

import type { CadLayer, CadSession } from "../cad/types";

type Props = {
  cad: CadSession;
  revision: number;
  onChanged: () => void;
  onEcho: (lines: string[]) => void;
};

const PRESETS: [string, number, number, number][] = [
  ["0", 200, 200, 200],
  ["Axe", 255, 80, 80],
  ["Cotation", 80, 220, 120],
  ["Hachures", 120, 160, 255],
  ["Texte", 255, 200, 80],
  ["Construction", 160, 160, 100],
];

export function LayerPanel({ cad, revision, onChanged, onEcho }: Props) {
  const [layers, setLayers] = useState<CadLayer[]>([]);
  const [name, setName] = useState("Nouveau");

  const refresh = useCallback(() => {
    try {
      setLayers(JSON.parse(cad.layers_json()) as CadLayer[]);
    } catch {
      setLayers([]);
    }
  }, [cad]);

  useEffect(() => {
    refresh();
  }, [refresh, revision]);

  return (
    <div className="flex flex-col gap-2 border-b border-ink-700/50 p-2">
      <p className="text-[10px] uppercase tracking-wider text-ink-500">Calques</p>
      <ul className="max-h-40 space-y-0.5 overflow-auto text-xs">
        {layers.map((l) => (
          <li key={l.id}>
            <button
              type="button"
              className={`flex w-full items-center gap-2 rounded px-1.5 py-1 text-left hover:bg-ink-800/60 ${
                l.current ? "bg-accent-soft/15 text-accent-soft" : "text-ink-200"
              }`}
              onClick={() => {
                try {
                  cad.set_current_layer(l.id);
                  onChanged();
                  onEcho([`Calque courant : ${l.name}`]);
                } catch (e) {
                  onEcho([String(e)]);
                }
              }}
            >
              <span
                className="inline-block h-2.5 w-2.5 shrink-0 rounded-sm border border-ink-600"
                style={{ background: `rgb(${l.color[0]},${l.color[1]},${l.color[2]})` }}
              />
              <span className="truncate font-mono">{l.name}</span>
              {l.current && <span className="ml-auto text-[9px] text-accent-soft">CUR</span>}
            </button>
          </li>
        ))}
      </ul>
      <div className="flex gap-1">
        <input
          className="min-w-0 flex-1 rounded border border-ink-700 bg-ink-950 px-1.5 py-1 text-xs text-ink-100"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          type="button"
          className="rounded border border-ink-700 px-2 text-xs hover:border-accent-soft"
          onClick={() => {
            const preset = PRESETS.find((p) => p[0].toLowerCase() === name.toLowerCase());
            const [r, g, b] = preset
              ? [preset[1], preset[2], preset[3]]
              : [140, 200, 255];
            try {
              const id = cad.create_layer(name.trim() || "Calque", r, g, b);
              cad.set_current_layer(id);
              onChanged();
              onEcho([`Calque créé : ${name}`]);
            } catch (e) {
              onEcho([String(e)]);
            }
          }}
        >
          +
        </button>
      </div>
      <button
        type="button"
        className="rounded border border-ink-700 px-2 py-1 text-[11px] text-ink-300 hover:border-accent-soft"
        onClick={() => {
          const cur = layers.find((l) => l.current);
          if (!cur) return;
          try {
            const n = cad.set_selection_layer(cur.id);
            onChanged();
            onEcho([`${n} entité(s) → ${cur.name}`]);
          } catch (e) {
            onEcho([String(e)]);
          }
        }}
      >
        Sélection → calque courant
      </button>
    </div>
  );
}
