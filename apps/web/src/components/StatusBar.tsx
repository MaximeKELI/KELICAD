import { useEffect, useState } from "react";

import type { CadSession, CadStatus } from "../cad/types";

type Props = {
  cad: CadSession;
  revision: number;
  cursor?: { x: number; y: number } | null;
  layout: "model" | "layout1";
  onLayout: (l: "model" | "layout1") => void;
  onChanged: () => void;
};

export function StatusBar({
  cad,
  revision,
  cursor,
  layout,
  onLayout,
  onChanged,
}: Props) {
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
    if (key === "ortho") cad.set_ortho(!status.ortho);
    else cad.set_osnap(!status.osnap);
    onChanged();
  };

  return (
    <div className="cad-statusbar">
      <div className="cad-layout-tabs">
        <button
          type="button"
          className={layout === "model" ? "is-active" : ""}
          onClick={() => onLayout("model")}
        >
          Model
        </button>
        <button
          type="button"
          className={layout === "layout1" ? "is-active" : ""}
          onClick={() => onLayout("layout1")}
        >
          Layout1
        </button>
        <button type="button" className="cad-plus" title="Nouveau layout">
          +
        </button>
      </div>

      <div className="cad-coords font-mono">
        {cursor
          ? `${cursor.x.toFixed(3)}, ${cursor.y.toFixed(3)}`
          : "—, —"}
      </div>

      <div className="cad-tray">
        <button
          type="button"
          title="Grille (affichage)"
          className="is-on"
          disabled
        >
          GRID
        </button>
        <button
          type="button"
          title="F8 — Ortho"
          className={status.ortho ? "is-on" : ""}
          onClick={() => toggle("ortho")}
        >
          ORTHO
        </button>
        <button
          type="button"
          title="F3 — Accrochage objets"
          className={status.osnap ? "is-on" : ""}
          onClick={() => toggle("osnap")}
        >
          OSNAP
        </button>
        <button type="button" title="Polar (bientôt)" disabled>
          POLAR
        </button>
        <span className="cad-tray-sep" />
        <span className="cad-tray-meta">
          Sél.&nbsp;<b>{status.selection}</b>
        </span>
        <span className="cad-tray-meta">
          Ent.&nbsp;<b>{status.entities}</b>
        </span>
        <span className="cad-tray-meta">
          3D&nbsp;<b>{status.solids}</b>
        </span>
        <span className="cad-tray-meta">
          Calque&nbsp;<b>{status.layer}</b>
        </span>
        <span className="cad-tray-hint">F3 OSNAP · F8 ORTHO · Espace=pan</span>
      </div>
    </div>
  );
}
