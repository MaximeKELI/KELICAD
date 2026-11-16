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

const OSNAP_MODES: { bit: number; label: string; tip: string }[] = [
  { bit: 1, label: "END", tip: "Extrémité" },
  { bit: 2, label: "MID", tip: "Milieu" },
  { bit: 4, label: "CEN", tip: "Centre" },
  { bit: 32, label: "INT", tip: "Intersection" },
  { bit: 128, label: "PER", tip: "Perpendiculaire" },
  { bit: 256, label: "TAN", tip: "Tangente" },
  { bit: 512, label: "NEA", tip: "Plus proche" },
  { bit: 1024, label: "EXT", tip: "Extension" },
];

export function StatusBar({
  cad,
  revision,
  cursor,
  layout,
  onLayout,
  onChanged,
}: Props) {
  const [status, setStatus] = useState<CadStatus | null>(null);
  const [osnapOpen, setOsnapOpen] = useState(false);

  useEffect(() => {
    try {
      setStatus(JSON.parse(cad.status_json()) as CadStatus);
    } catch {
      setStatus(null);
    }
  }, [cad, revision]);

  if (!status) return null;

  const toggle = (key: "ortho" | "osnap" | "grid" | "polar" | "snap") => {
    if (key === "ortho") cad.set_ortho(!status.ortho);
    else if (key === "osnap") cad.set_osnap(!status.osnap);
    else if (key === "grid") {
      cad.set_sysvar("GRIDMODE", status.grid ? "0" : "1");
    } else if (key === "polar") {
      cad.set_sysvar("POLARMODE", status.polar ? "0" : "1");
    } else {
      cad.set_sysvar("SNAPMODE", status.snap ? "0" : "1");
    }
    onChanged();
  };

  const osmode = status.osmode ?? 0;
  const toggleMode = (bit: number) => {
    const next = osmode & bit ? osmode & ~bit : osmode | bit;
    cad.set_sysvar("OSMODE", String(next || 0));
    if (next !== 0 && !status.osnap) cad.set_osnap(true);
    if (next === 0) cad.set_osnap(false);
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
          title="Espace papier (aperçu)"
          onClick={() => onLayout("layout1")}
        >
          Layout1
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
          title="F7 — Grille"
          className={status.grid ? "is-on" : ""}
          onClick={() => toggle("grid")}
        >
          GRID
        </button>
        <button
          type="button"
          title="F9 — Accrochage grille"
          className={status.snap ? "is-on" : ""}
          onClick={() => toggle("snap")}
        >
          SNAP
        </button>
        <button
          type="button"
          title="F8 — Ortho"
          className={status.ortho ? "is-on" : ""}
          onClick={() => toggle("ortho")}
        >
          ORTHO
        </button>
        <div className="cad-osnap-wrap">
          <button
            type="button"
            title="F3 — Accrochage objets (clic droit : modes)"
            className={status.osnap ? "is-on" : ""}
            onClick={() => toggle("osnap")}
            onContextMenu={(e) => {
              e.preventDefault();
              setOsnapOpen((v) => !v);
            }}
          >
            OSNAP
          </button>
          {osnapOpen && (
            <div className="cad-osnap-menu">
              {OSNAP_MODES.map((m) => (
                <button
                  key={m.bit}
                  type="button"
                  className={osmode & m.bit ? "is-on" : ""}
                  title={m.tip}
                  onClick={() => toggleMode(m.bit)}
                >
                  {m.label}
                </button>
              ))}
              <button
                type="button"
                className="cad-osnap-close"
                onClick={() => setOsnapOpen(false)}
              >
                Fermer
              </button>
            </div>
          )}
        </div>
        <button
          type="button"
          title="F10 — Polar (45°)"
          className={status.polar ? "is-on" : ""}
          onClick={() => toggle("polar")}
        >
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
        <span className="cad-tray-hint">
          F3 OSNAP · F7 GRID · F8 ORTHO · F9 SNAP · F10 POLAR
        </span>
      </div>
    </div>
  );
}
