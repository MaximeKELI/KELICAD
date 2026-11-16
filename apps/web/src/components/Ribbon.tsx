import { useMemo, useState } from "react";

import type { CadLayer, CadSession } from "../cad/types";

type TabId = "home" | "annotate" | "solid" | "view" | "output";

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

function Btn({
  label,
  sub,
  onClick,
  wide,
  active,
}: {
  label: string;
  sub?: string;
  onClick: () => void;
  wide?: boolean;
  active?: boolean;
}) {
  return (
    <button
      type="button"
      className={`cad-rbtn ${wide ? "cad-rbtn-wide" : ""} ${active ? "is-active" : ""}`}
      onClick={onClick}
      title={sub ?? label}
    >
      <span className="cad-rbtn-label">{label}</span>
      {sub && <span className="cad-rbtn-sub">{sub}</span>}
    </button>
  );
}

function Panel({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="cad-panel">
      <div className="cad-panel-body">{children}</div>
      <div className="cad-panel-title">{title}</div>
    </div>
  );
}

export function Ribbon({
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
}: Props) {
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
      {/* Quick Access + title */}
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
          <button type="button" title="PDF" onClick={onPdf}>
            ⎙
          </button>
          <span className="cad-sep" />
          <button type="button" title="Annuler (U)" onClick={onUndo}>
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

      {/* Tabs */}
      <div className="cad-tabs">
        {(
          [
            ["home", "Accueil"],
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

      {/* Panels */}
      <div className="cad-ribbon-body">
        {tab === "home" && (
          <>
            <Panel title="Dessiner">
              <Btn label="Ligne" sub="L" onClick={() => onCommand("L")} />
              <Btn label="Pline" sub="PL" onClick={() => onCommand("PL")} />
              <Btn label="Cercle" sub="C" onClick={() => onCommand("C")} />
              <Btn label="Arc" sub="A" onClick={() => onCommand("A")} />
              <Btn label="Rect" sub="REC" onClick={() => onCommand("REC")} />
            </Panel>
            <Panel title="Modifier">
              <Btn label="Déplacer" sub="M" onClick={() => onCommand("M")} />
              <Btn label="Copier" sub="CO" onClick={() => onCommand("CO")} />
              <Btn label="Rotation" sub="RO" onClick={() => onCommand("RO")} />
              <Btn label="Échelle" sub="SC" onClick={() => onCommand("SC")} />
              <Btn label="Miroir" sub="MI" onClick={() => onCommand("MI")} />
              <Btn label="Décalage" sub="O" onClick={() => onCommand("O")} />
              <Btn label="Ajuster" sub="TR" onClick={() => onCommand("TR")} />
              <Btn label="Congé" sub="F" onClick={() => onCommand("F")} />
              <Btn label="Effacer" sub="E" onClick={() => onCommand("E")} />
            </Panel>
            <Panel title="Calques">
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
              <Btn
                label="Propriétés"
                sub="calques"
                wide
                onClick={() => onEcho(["Panneau Calques → à droite"])}
              />
            </Panel>
            <Panel title="Bloc">
              <Btn label="Insérer" sub="I" wide onClick={() => onCommand("I")} />
              <Btn label="Créer" sub="B" wide onClick={() => onCommand("B")} />
              <Btn label="Exploser" sub="X" onClick={() => onCommand("X")} />
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
                  Type
                  <select className="cad-select" defaultValue="bylayer" disabled>
                    <option value="bylayer">ParCalque</option>
                  </select>
                </label>
              </div>
            </Panel>
            <Panel title="Mesure">
              <Btn label="Cote" sub="DLI" onClick={() => onCommand("DLI")} />
              <Btn label="Alignée" sub="DAL" onClick={() => onCommand("DAL")} />
              <Btn label="Repère" sub="LEAD" onClick={() => onCommand("LEAD")} />
            </Panel>
          </>
        )}

        {tab === "annotate" && (
          <>
            <Panel title="Texte">
              <Btn label="Texte" sub="DT" wide onClick={() => onCommand("DT")} />
              <Btn label="Tableau" sub="TBL" onClick={() => onCommand("TBL")} />
            </Panel>
            <Panel title="Cotes">
              <Btn label="Linéaire" sub="DLI" onClick={() => onCommand("DLI")} />
              <Btn label="Alignée" sub="DAL" onClick={() => onCommand("DAL")} />
              <Btn label="Rayon" sub="DRA" onClick={() => onCommand("DRA")} />
              <Btn label="Diamètre" sub="DDI" onClick={() => onCommand("DDI")} />
              <Btn label="Angle" sub="DAN" onClick={() => onCommand("DAN")} />
              <Btn label="Continue" sub="DCO" onClick={() => onCommand("DCO")} />
              <Btn label="Baseline" sub="DBA" onClick={() => onCommand("DBA")} />
            </Panel>
            <Panel title="Coupe">
              <Btn label="Section" sub="SEC" wide onClick={() => onCommand("SEC")} />
              <Btn label="Nuage" sub="RC" onClick={() => onCommand("RC")} />
              <Btn label="Centre" sub="CM" onClick={() => onCommand("CM")} />
            </Panel>
          </>
        )}

        {tab === "solid" && (
          <>
            <Panel title="Primitifs">
              <Btn label="Boîte" sub="BOX" onClick={() => onSolid("box")} />
              <Btn label="Cylindre" sub="CYL" onClick={() => onSolid("cyl")} />
              <Btn label="Sphère" sub="SPH" onClick={() => onSolid("sph")} />
              <Btn label="Cône" sub="CONE" onClick={() => onSolid("cone")} />
              <Btn label="Tore" sub="TOR" onClick={() => onSolid("tor")} />
            </Panel>
            <Panel title="Modélisation">
              <Btn label="Extruder" sub="EXTRUDE" wide onClick={() => onSolid("extrude")} />
              <Btn label="Révolution" sub="REV" onClick={() => onSolid("rev")} />
              <Btn label="Union" sub="BOOL+" onClick={() => onSolid("union")} />
              <Btn label="Différence" sub="BOOL−" onClick={() => onSolid("diff")} />
              <Btn label="Vider 3D" sub="CLR" onClick={() => onSolid("clear")} />
            </Panel>
            <Panel title="Vues dérivées">
              <Btn label="Plan" onClick={() => onSolid("view-plan")} />
              <Btn label="Élév. X" onClick={() => onSolid("view-elevx")} />
              <Btn label="Élév. Y" onClick={() => onSolid("view-elevy")} />
            </Panel>
          </>
        )}

        {tab === "view" && (
          <>
            <Panel title="Espace">
              <Btn
                label="2D"
                active={viewMode === "2d"}
                onClick={() => onViewMode("2d")}
              />
              <Btn
                label="Split"
                active={viewMode === "split"}
                onClick={() => onViewMode("split")}
              />
              <Btn
                label="3D"
                active={viewMode === "3d"}
                onClick={() => onViewMode("3d")}
              />
            </Panel>
            <Panel title="Gabarits">
              <Btn label="A4" onClick={() => onSolid("tpl-a4")} />
              <Btn label="A3" onClick={() => onSolid("tpl-a3")} />
              <Btn label="A1" onClick={() => onSolid("tpl-a1")} />
              <Btn label="Arch" onClick={() => onSolid("tpl-arch")} />
              <Btn label="Méca" onClick={() => onSolid("tpl-mech")} />
            </Panel>
          </>
        )}

        {tab === "output" && (
          <>
            <Panel title="Fichier">
              <Btn label="DXF ↓" onClick={onImportDxf} />
              <Btn label="DXF ↑" onClick={onExportDxf} />
              <Btn label="SVG" onClick={onExportSvg} />
              <Btn label="PDF" onClick={onPdf} />
            </Panel>
            <Panel title="BIM">
              <Btn label="IFC ↑" onClick={() => onSolid("ifc")} />
              <Btn label="SCHED" onClick={() => onSolid("sched")} />
            </Panel>
          </>
        )}
      </div>
    </div>
  );
}
