import { useMemo } from "react";

import type { CadSession } from "../cad/types";

type BlockInfo = {
  id: string;
  name: string;
  entity_count: number;
};

type Props = {
  cad: CadSession;
  revision: number;
  onCommand: (raw: string) => void;
  onEcho: (lines: string[]) => void;
};

export function BlockLibrary({ cad, revision, onCommand, onEcho }: Props) {
  const blocks = useMemo(() => {
    void revision;
    try {
      return JSON.parse(cad.blocks_json()) as BlockInfo[];
    } catch {
      return [];
    }
  }, [cad, revision]);

  return (
    <div className="border-t border-ink-700/60 p-2">
      <p className="mb-1 text-[10px] uppercase tracking-wider text-ink-500">
        Blocs
      </p>
      {blocks.length === 0 ? (
        <p className="font-mono text-[10px] text-ink-600">
          Aucun — sélectionnez puis BLOCK
        </p>
      ) : (
        <ul className="max-h-28 space-y-1 overflow-auto">
          {blocks.map((b) => (
            <li key={b.id} className="flex items-center gap-1">
              <button
                type="button"
                className="min-w-0 flex-1 truncate rounded border border-ink-700 px-1.5 py-0.5 text-left font-mono text-[10px] hover:border-accent-soft"
                title={`${b.name} (${b.entity_count})`}
                onClick={() => {
                  onCommand(cad.start_command("INSERT"));
                  onCommand(cad.submit_text(b.name));
                  onEcho([`INSERT « ${b.name} » — cliquez le point`]);
                }}
              >
                {b.name}
                <span className="ml-1 text-ink-600">{b.entity_count}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="mt-1 flex gap-1">
        <button
          type="button"
          className="rounded border border-ink-700 px-1.5 py-0.5 font-mono text-[10px] hover:border-accent-soft"
          onClick={() => onCommand(cad.start_command("BLOCK"))}
        >
          BLOCK
        </button>
        <button
          type="button"
          className="rounded border border-ink-700 px-1.5 py-0.5 font-mono text-[10px] hover:border-accent-soft"
          onClick={() => onCommand(cad.start_command("EXPLODE"))}
        >
          X
        </button>
      </div>
    </div>
  );
}
