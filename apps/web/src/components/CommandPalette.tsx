import { useEffect, useMemo, useState } from "react";

import { CATALOG, type CatalogTool } from "../cad/commandCatalog";

type Props = {
  open: boolean;
  onClose: () => void;
  onPick: (cmd: string) => void;
};

export function CommandPalette({ open, onClose, onPick }: Props) {
  const [q, setQ] = useState("");
  const tools = useMemo(() => {
    const all = Object.values(CATALOG).filter(
      (t) => t.status !== "unavailable" && t.kind.type === "cad",
    );
    const needle = q.trim().toLowerCase();
    if (!needle) return all.slice(0, 40);
    return all
      .filter((t) => {
        const hay = `${t.label} ${t.id} ${(t.aliases ?? []).join(" ")}`.toLowerCase();
        return hay.includes(needle);
      })
      .slice(0, 40);
  }, [q]);

  useEffect(() => {
    if (!open) setQ("");
  }, [open]);

  if (!open) return null;

  const invoke = (t: CatalogTool) => {
    if (t.kind.type === "cad") onPick(t.kind.cmd);
    onClose();
  };

  return (
    <div className="cad-cmdpal-backdrop" onMouseDown={onClose}>
      <div
        className="cad-cmdpal"
        role="dialog"
        aria-label="Palette de commandes"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <input
          autoFocus
          className="cad-cmdpal-input"
          placeholder="Commande… (Ctrl+K)"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
            if (e.key === "Enter" && tools[0]) invoke(tools[0]);
          }}
        />
        <ul className="cad-cmdpal-list">
          {tools.map((t) => (
            <li key={t.id}>
              <button type="button" onClick={() => invoke(t)}>
                <span className="cad-cmdpal-glyph">{t.glyph}</span>
                <span>
                  {t.label}
                  <small>{t.kind.type === "cad" ? t.kind.cmd : ""}</small>
                </span>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
