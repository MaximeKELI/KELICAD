import { useEffect, useState } from "react";

import type { CadSession, CadStatus } from "../cad/types";

type Props = {
  cad: CadSession;
  revision: number;
  onChanged: () => void;
};

export function StatusBar({ cad, revision, onChanged }: Props) {
  const [status, setStatus] = useState<CadStatus | null>(null);

  useEffect(() => {
    try {
      setStatus(JSON.parse(cad.status_json()) as CadStatus);
    } catch {
      setStatus(null);
    }
  }, [cad, revision]);

  if (!status) return null;

  const toggle = (key: "ortho" | "osnap") => {
    if (key === "ortho") {
      cad.set_ortho(!status.ortho);
    } else {
      cad.set_osnap(!status.osnap);
    }
    onChanged();
  };

  return (
    <div className="flex flex-wrap items-center gap-2 border-t border-ink-700/60 bg-ink-950 px-3 py-1 font-mono text-[11px] text-ink-400">
      <span>
        Calque{" "}
        <span className="text-ink-100">{status.layer}</span>
      </span>
      <button
        type="button"
        title="F8 — Ortho"
        className={`rounded px-2 py-0.5 ${status.ortho ? "bg-accent-soft/25 text-accent-soft" : "hover:bg-ink-800"}`}
        onClick={() => toggle("ortho")}
      >
        ORTHO
      </button>
      <button
        type="button"
        title="F3 — Accrochage"
        className={`rounded px-2 py-0.5 ${status.osnap ? "bg-accent-soft/25 text-accent-soft" : "hover:bg-ink-800"}`}
        onClick={() => toggle("osnap")}
      >
        OSNAP
      </button>
      <span className="text-ink-600">|</span>
      <span>
        Sél. <span className="text-ink-100">{status.selection}</span>
      </span>
      <span>
        Ent. <span className="text-ink-100">{status.entities}</span>
      </span>
      <span>
        3D <span className="text-ink-100">{status.solids}</span>
      </span>
      <span className="ml-auto text-ink-600">F3 OSNAP · F8 ORTHO · Espace pan</span>
    </div>
  );
}
