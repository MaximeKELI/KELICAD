import { useMemo, useState, type ReactNode } from "react";

import type { CadLayer, CadSession } from "../cad/types";

type TabId = "home" | "insert" | "annotate" | "solid" | "view" | "output";

type Props = {
  cad: CadSession;
  revision: number;
  projectName: string;
  viewMode: "2d" | "split" | "3d";
  entityCount: number;
  solidCount: number;
  onCommand: (cmd: string) => void;
  onNew: () => void;
  onOpen: () => void;
  onSave: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onPdf: () => void;
  onImportDxf: () => void;
  onExportDxf: () => void;
  onExportSvg: () => void;
  onViewMode: (m: "2d" | "split" | "3d") => void;
  onSolid: (action: string) => void;
  onChanged: () => void;
  onEcho: (lines: string[]) => void;
};

type ToolDef = {
  cmd: string;
  label: string;
  glyph: string;
  tip?: string;
};

function Tool({
  t,
  size = "md",
  onCommand,
}: {
  t: ToolDef;
  size?: "lg" | "md" | "sm";
  onCommand: (cmd: string) => void;
}) {
  return (
    <button
      type="button"
      className={`cad-toolbtn cad-toolbtn-${size}`}
      title={t.tip ?? `${t.label} (${t.cmd})`}
      onClick={() => onCommand(t.cmd)}
    >
      <span className="cad-toolbtn-glyph">{t.glyph}</span>
      {size !== "sm" && <span className="cad-toolbtn-text">{t.label}</span>}
    </button>
  );
}

function Flyout({
  main,
  items,
  onCommand,
}: {
  main: ToolDef;
  items: ToolDef[];
  onCommand: (cmd: string) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div
      className="cad-flyout"
      onMouseLeave={() => setOpen(false)}
    >
      <div className="cad-flyout-main">
        <button
          type="button"
          className="cad-toolbtn cad-toolbtn-md"
          title={main.tip ?? main.label}
          onClick={() => onCommand(main.cmd)}
        >
          <span className="cad-toolbtn-glyph">{main.glyph}</span>
          <span className="cad-toolbtn-text">{main.label}</span>
        </button>
        <button
          type="button"
          className="cad-flyout-caret"
          aria-label="Plus"
          onClick={() => setOpen((v) => !v)}
        >
          ▾
        </button>
      </div>
      {open && (
        <div className="cad-flyout-menu">
          {items.map((it) => (
            <button
              key={it.cmd}
              type="button"
              onClick={() => {
                onCommand(it.cmd);
                setOpen(false);
              }}
            >
              <span>{it.glyph}</span>
              <span>
                {it.label}
                <small>{it.cmd}</small>
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function Stack({ children }: { children: ReactNode }) {
  return <div className="cad-stack">{children}</div>;
}

function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="cad-panel">
      <div className="cad-panel-body">{children}</div>
      <div className="cad-panel-title">{title}</div>
    </div>
  );
}

export function Ribbon(props: Props) {
  const {
    cad,
    revision,
    projectName,
    viewMode,
    entityCount,
    solidCount,
    onCommand,
    onNew,
    onOpen,
    onSave,
    onUndo,
    onRedo,
    onPdf,
    onImportDxf,
    onExportDxf,
    onExportSvg,
    onViewMode,
    onSolid,
    onChanged,
    onEcho,
  } = props;

  const [tab, setTab] = useState<TabId>("home");
  const layers = useMemo(() => {
    void revision;
    try {
      return JSON.parse(cad.layers_json()) as CadLayer[];
    } catch {
      return [];
    }
  }, [cad, revision]);
  const current = layers.find((l) => l.current) ?? layers[0];

  return (
    <div className="cad-ribbon">
      <div className="cad-qat">
        <span className="cad-brand">KELICAD</span>
        <div className="cad-qat-btns">
          <button type="button" title="Nouveau" onClick={onNew}>
            ⎚
          </button>
          <button type="button" title="Ouvrir" onClick={onOpen}>
            📂
          </button>
          <button type="button" title="Enregistrer" onClick={onSave}>
            💾
          </button>
          <button type="button" title="Enregistrer sous" onClick={onSave}>
            💾+
          </button>
          <button type="button" title="PDF / Imprimer" onClick={onPdf}>
            ⎙
          </button>
          <span className="cad-sep" />
          <button type="button" title="Annuler" onClick={onUndo}>
            ↶
          </button>
          <button type="button" title="Rétablir" onClick={onRedo}>
            ↷
          </button>
        </div>
        <span className="cad-docname" title={projectName}>
          {projectName}
        </span>
        <span className="cad-stats">
          {entityCount} ent · {solidCount} sol
        </span>
      </div>

      <div className="cad-tabs">
        {(
          [
            ["home", "Accueil"],
            ["insert", "Insérer"],
            ["annotate", "Annoter"],
            ["solid", "3D"],
            ["view", "Affichage"],
            ["output", "Sortie"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            className={`cad-tab ${tab === id ? "is-active" : ""}`}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="cad-ribbon-body">
        {tab === "home" && (
          <>
            <Panel title="Dessiner">
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "L", label: "Ligne", glyph: "╱" }}
              />
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "PL", label: "Pline", glyph: "⌇" }}
              />
              <Flyout
                onCommand={onCommand}
                main={{ cmd: "C", label: "Cercle", glyph: "○" }}
                items={[
                  { cmd: "C", label: "Centre, rayon", glyph: "○" },
                  { cmd: "C2", label: "2 points (Ø)", glyph: "◯" },
                  { cmd: "C3", label: "3 points", glyph: "◎" },
                  { cmd: "DONUT", label: "Donut / Anneau", glyph: "◍" },
                ]}
              />
              <Flyout
                onCommand={onCommand}
                main={{ cmd: "A", label: "Arc", glyph: "⌒" }}
                items={[
                  { cmd: "A", label: "Centre début fin", glyph: "⌒" },
                  { cmd: "REVCLOUD", label: "Nuage révision", glyph: "☁" },
                ]}
              />
              <Stack>
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "REC", label: "Rect", glyph: "▭", tip: "Rectangle" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "POL", label: "Poly", glyph: "⬡", tip: "Polygone" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "EL", label: "Ellip", glyph: "⬭" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "H", label: "Hach", glyph: "▤", tip: "Hachure" }}
                />
              </Stack>
              <Stack>
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "SPL", label: "Spl", glyph: "∿" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "PO", label: "Pt", glyph: "·" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "RAY", label: "Ray", glyph: "→" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "XL", label: "Xln", glyph: "↔" }}
                />
              </Stack>
              <Stack>
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "STAR", label: "★", glyph: "★", tip: "Étoile" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "TRI", label: "△", glyph: "△", tip: "Triangle" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "CAPSULE", label: "⬭", glyph: "⬭", tip: "Capsule" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "ARROW", label: "➤", glyph: "➤", tip: "Flèche" }}
                />
              </Stack>
            </Panel>

            <Panel title="Modifier">
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "M", label: "Déplacer", glyph: "⇔" }}
              />
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "CO", label: "Copier", glyph: "⧉" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "S", label: "Étirer", glyph: "⤡" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "RO", label: "Rotation", glyph: "↻" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "MI", label: "Miroir", glyph: "||-||" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "SC", label: "Échelle", glyph: "⤢" }}
              />
              <Flyout
                onCommand={onCommand}
                main={{ cmd: "TR", label: "Ajuster", glyph: "✂" }}
                items={[
                  { cmd: "TR", label: "Ajuster (Trim)", glyph: "✂" },
                  { cmd: "EX", label: "Prolonger", glyph: "↦" },
                  { cmd: "LEN", label: "Allonger", glyph: "⟷" },
                  { cmd: "BR", label: "Coupure", glyph: "⊥" },
                ]}
              />
              <Flyout
                onCommand={onCommand}
                main={{ cmd: "F", label: "Congé", glyph: "⌒" }}
                items={[
                  { cmd: "F", label: "Congé (Fillet)", glyph: "⌒" },
                  { cmd: "CHA", label: "Chanfrein", glyph: "⌐" },
                ]}
              />
              <Flyout
                onCommand={onCommand}
                main={{ cmd: "AR", label: "Réseau", glyph: "▦" }}
                items={[
                  { cmd: "AR", label: "Réseau rectangulaire", glyph: "▦" },
                  { cmd: "APOLAR", label: "Réseau polaire", glyph: "◎" },
                ]}
              />
              <Stack>
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "O", label: "Off", glyph: "⇉", tip: "Décalage" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "J", label: "Join", glyph: "⋃" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "X", label: "Xpl", glyph: "✸", tip: "Exploser" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "AL", label: "Algn", glyph: "⇅" }}
                />
              </Stack>
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "E", label: "Effacer", glyph: "✕" }}
              />
            </Panel>

            <Panel title="Annotation">
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "DT", label: "Texte", glyph: "A" }}
              />
              <Flyout
                onCommand={onCommand}
                main={{ cmd: "DLI", label: "Cote", glyph: "↔" }}
                items={[
                  { cmd: "DLI", label: "Linéaire", glyph: "↔" },
                  { cmd: "DAL", label: "Alignée", glyph: "⤢" },
                  { cmd: "DRA", label: "Rayon", glyph: "R" },
                  { cmd: "DDI", label: "Diamètre", glyph: "⌀" },
                  { cmd: "DAN", label: "Angle", glyph: "∠" },
                  { cmd: "DCO", label: "Continue", glyph: "↦↔" },
                  { cmd: "DBA", label: "Baseline", glyph: "≡" },
                ]}
              />
              <Stack>
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "LEAD", label: "Lead", glyph: "→" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "TBL", label: "Tab", glyph: "▦" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "CM", label: "Ctr", glyph: "⌖" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "CL", label: "Axe", glyph: "┼" }}
                />
              </Stack>
            </Panel>

            <Panel title="Calques">
              <button
                type="button"
                className="cad-toolbtn cad-toolbtn-lg"
                title="Propriétés des calques"
                onClick={() => onEcho(["Calques → panneau droit (onglet Calques)"])}
              >
                <span className="cad-toolbtn-glyph">☰</span>
                <span className="cad-toolbtn-text">Propriétés</span>
              </button>
              <div className="cad-layer-combo">
                <span
                  className="cad-swatch"
                  style={{
                    background: current
                      ? `rgb(${current.color.join(",")})`
                      : "#888",
                  }}
                />
                <select
                  className="cad-select"
                  value={current?.id ?? ""}
                  onChange={(e) => {
                    try {
                      cad.set_current_layer(e.target.value);
                      onChanged();
                      const name =
                        layers.find((l) => l.id === e.target.value)?.name ?? "";
                      onEcho([`Calque : ${name}`]);
                    } catch (err) {
                      onEcho([String(err)]);
                    }
                  }}
                >
                  {layers.map((l) => (
                    <option key={l.id} value={l.id}>
                      {l.name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="cad-mini">
                <button
                  type="button"
                  title="Calque courant → sélection"
                  onClick={() => {
                    if (!current) return;
                    const n = cad.set_selection_layer(current.id);
                    onChanged();
                    onEcho([`${n} objet(s) → ${current.name}`]);
                  }}
                >
                  ⤵
                </button>
                <button
                  type="button"
                  title="Geler / Dégeler"
                  onClick={() => onEcho(["Geler calque (à brancher)"])}
                >
                  ❄
                </button>
                <button
                  type="button"
                  title="Verrouiller"
                  onClick={() => onEcho(["Verrouiller calque (à brancher)"])}
                >
                  🔒
                </button>
                <button
                  type="button"
                  title="Nouveau calque"
                  onClick={() => {
                    const name = `Calque${layers.length}`;
                    const id = cad.create_layer(name, 140, 200, 255);
                    cad.set_current_layer(id);
                    onChanged();
                    onEcho([`Calque créé : ${name}`]);
                  }}
                >
                  +
                </button>
              </div>
            </Panel>

            <Panel title="Bloc">
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "I", label: "Insérer", glyph: "⧉" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "B", label: "Créer", glyph: "▢" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "X", label: "Exploser", glyph: "✸" }}
              />
            </Panel>

            <Panel title="Propriétés">
              <div className="cad-prop-stack">
                <label>
                  Couleur
                  <select className="cad-select" defaultValue="bylayer" disabled>
                    <option value="bylayer">ParCalque</option>
                  </select>
                </label>
                <label>
                  Épaisseur
                  <select className="cad-select" defaultValue="bylayer" disabled>
                    <option value="bylayer">ParCalque</option>
                  </select>
                </label>
                <label>
                  Type ligne
                  <select className="cad-select" defaultValue="bylayer" disabled>
                    <option value="bylayer">ParCalque</option>
                  </select>
                </label>
              </div>
            </Panel>

            <Panel title="Construction">
              <Stack>
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "PAR", label: "//", glyph: "∥", tip: "Parallèle" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "ML", label: "Mid", glyph: "⟂", tip: "Médiane" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "LT", label: "Tan", glyph: "Tangent", tip: "Tangente pt-cercle" }}
                />
                <Tool
                  size="sm"
                  onCommand={onCommand}
                  t={{ cmd: "LC", label: "T2C", glyph: "⊂", tip: "Tangentes 2 cercles" }}
                />
              </Stack>
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "GRID", label: "Grille", glyph: "▦" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "SEC", label: "Coupe", glyph: "┼" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "WO", label: "Wipe", glyph: "▣" }}
              />
            </Panel>
          </>
        )}

        {tab === "insert" && (
          <>
            <Panel title="Bloc">
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "I", label: "Insérer", glyph: "⧉" }}
              />
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "B", label: "Définir", glyph: "▢" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "X", label: "Exploser", glyph: "✸" }}
              />
            </Panel>
            <Panel title="Référence">
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "XR", label: "XREF", glyph: "⛓" }}
              />
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "PDFATTACH", label: "PDF", glyph: "📄" }}
              />
            </Panel>
            <Panel title="Import">
              <Tool
                size="md"
                onCommand={() => onImportDxf()}
                t={{ cmd: "_DXFI", label: "DXF ↓", glyph: "⇩" }}
              />
            </Panel>
          </>
        )}

        {tab === "annotate" && (
          <>
            <Panel title="Texte">
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "DT", label: "Texte", glyph: "A" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "TBL", label: "Tableau", glyph: "▦" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "LEAD", label: "Repère", glyph: "→" }}
              />
            </Panel>
            <Panel title="Cotes">
              <Tool
                size="lg"
                onCommand={onCommand}
                t={{ cmd: "DLI", label: "Linéaire", glyph: "↔" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "DAL", label: "Alignée", glyph: "⤢" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "DRA", label: "Rayon", glyph: "R" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "DDI", label: "Diamètre", glyph: "⌀" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "DAN", label: "Angle", glyph: "∠" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "DCO", label: "Continue", glyph: "↦" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "DBA", label: "Baseline", glyph: "≡" }}
              />
            </Panel>
            <Panel title="Repères">
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "CM", label: "Centre", glyph: "⌖" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "CL", label: "Axe", glyph: "┼" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "RC", label: "Nuage", glyph: "☁" }}
              />
              <Tool
                size="md"
                onCommand={onCommand}
                t={{ cmd: "SEC", label: "Section", glyph: "╪" }}
              />
            </Panel>
          </>
        )}

        {tab === "solid" && (
          <>
            <Panel title="Primitifs">
              <Tool
                size="lg"
                onCommand={() => onSolid("box")}
                t={{ cmd: "BOX", label: "Boîte", glyph: "cub" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("cyl")}
                t={{ cmd: "CYL", label: "Cylindre", glyph: "⌭" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("sph")}
                t={{ cmd: "SPH", label: "Sphère", glyph: "●" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("cone")}
                t={{ cmd: "CONE", label: "Cône", glyph: "▲" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("tor")}
                t={{ cmd: "TOR", label: "Tore", glyph: "◎" }}
              />
            </Panel>
            <Panel title="Modélisation">
              <Tool
                size="lg"
                onCommand={() => onSolid("extrude")}
                t={{ cmd: "EXT", label: "Extruder", glyph: "⇑" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("rev")}
                t={{ cmd: "REV", label: "Révol.", glyph: "↻" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("union")}
                t={{ cmd: "U+", label: "Union", glyph: "∪" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("diff")}
                t={{ cmd: "U−", label: "Diff.", glyph: "∖" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("clear")}
                t={{ cmd: "CLR", label: "Vider", glyph: "⌫" }}
              />
            </Panel>
            <Panel title="Vues 2D">
              <Tool
                size="md"
                onCommand={() => onSolid("view-plan")}
                t={{ cmd: "PLN", label: "Plan", glyph: "⊤" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("view-elevx")}
                t={{ cmd: "EX", label: "Élév X", glyph: "⌟" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("view-elevy")}
                t={{ cmd: "EY", label: "Élév Y", glyph: "⌜" }}
              />
            </Panel>
          </>
        )}

        {tab === "view" && (
          <>
            <Panel title="Espace">
              <Tool
                size="md"
                onCommand={() => onViewMode("2d")}
                t={{
                  cmd: "2D",
                  label: "2D",
                  glyph: "▢",
                  tip: viewMode === "2d" ? "Actif" : "Vue 2D",
                }}
              />
              <Tool
                size="md"
                onCommand={() => onViewMode("split")}
                t={{ cmd: "SPL", label: "Split", glyph: "⊞" }}
              />
              <Tool
                size="md"
                onCommand={() => onViewMode("3d")}
                t={{ cmd: "3D", label: "3D", glyph: "▣" }}
              />
            </Panel>
            <Panel title="Gabarits">
              {(["a4", "a3", "a2", "a1", "arch", "mech"] as const).map((t) => (
                <Tool
                  key={t}
                  size="sm"
                  onCommand={() => onSolid(`tpl-${t}`)}
                  t={{ cmd: t.toUpperCase(), label: t.toUpperCase(), glyph: "▭" }}
                />
              ))}
            </Panel>
          </>
        )}

        {tab === "output" && (
          <>
            <Panel title="Fichier">
              <Tool
                size="md"
                onCommand={onImportDxf}
                t={{ cmd: "DXF↓", label: "DXF ↓", glyph: "⇩" }}
              />
              <Tool
                size="md"
                onCommand={onExportDxf}
                t={{ cmd: "DXF↑", label: "DXF ↑", glyph: "⇧" }}
              />
              <Tool
                size="md"
                onCommand={onExportSvg}
                t={{ cmd: "SVG", label: "SVG", glyph: "🖼" }}
              />
              <Tool
                size="md"
                onCommand={onPdf}
                t={{ cmd: "PDF", label: "PDF", glyph: "⎙" }}
              />
            </Panel>
            <Panel title="BIM">
              <Tool
                size="md"
                onCommand={() => onSolid("ifc")}
                t={{ cmd: "IFC", label: "IFC ↑", glyph: "🏗" }}
              />
              <Tool
                size="md"
                onCommand={() => onSolid("sched")}
                t={{ cmd: "SCH", label: "SCHED", glyph: "☰" }}
              />
            </Panel>
          </>
        )}
      </div>
    </div>
  );
}
