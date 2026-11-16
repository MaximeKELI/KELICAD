import { useCallback, useEffect, useMemo, useState } from "react";

import { CollabClient, type CollabStatus, type PeerPresence } from "./cad/collab";
import {
  desktopAvailable,
  desktopOpenKelicad,
  desktopPing,
  desktopSaveKelicad,
} from "./cad/desktopBridge";
import {
  downloadBytes,
  kelicadBytesToSnapshotJson,
  snapshotToKelicadBytes,
} from "./cad/kelicadFile";
import { loadCad } from "./cad/loadCad";
import {
  loadAutosave,
  pushRecent,
  saveAutosave,
} from "./cad/projectStore";
import { parseCadOutput, type CadSession } from "./cad/types";
import { BlockLibrary } from "./components/BlockLibrary";
import { CollabBar } from "./components/CollabBar";
import { CommandLine } from "./components/CommandLine";
import { DrawingViewport } from "./components/DrawingViewport";
import { KeliPanel } from "./components/KeliPanel";
import { LayerPanel } from "./components/LayerPanel";
import { PropertiesPanel } from "./components/PropertiesPanel";
import { Ribbon } from "./components/Ribbon";
import { SelectionPanel } from "./components/SelectionPanel";
import { StatusBar } from "./components/StatusBar";
import { ToolRail } from "./components/ToolRail";
import { Viewport3D } from "./components/Viewport3D";

export function App() {
  const [cad, setCad] = useState<CadSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [echoes, setEchoes] = useState<string[]>([
    "KELICAD prêt — tapez L / C / REC ou utilisez le ruban.",
  ]);
  const [revision, setRevision] = useState(0);
  const [entityCount, setEntityCount] = useState(0);
  const [projectName, setProjectName] = useState("dessin.kelicad");
  const [collabStatus, setCollabStatus] = useState<CollabStatus>("disconnected");
  const [displayName, setDisplayName] = useState("Alice");
  const [peers, setPeers] = useState<PeerPresence[]>([]);
  const [peerTick, setPeerTick] = useState(0);
  const [viewMode, setViewMode] = useState<"2d" | "split" | "3d">("2d");
  const [solidCount, setSolidCount] = useState(0);
  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null);
  const [layout, setLayout] = useState<"model" | "layout1">("model");
  const [dockTab, setDockTab] = useState<"layers" | "blocks" | "props" | "keli">(
    "layers",
  );
  const [activeCmd, setActiveCmd] = useState<string | null>(null);

  useEffect(() => {
    void loadCad()
      .then(async (session) => {
        const auto = loadAutosave();
        if (auto) {
          try {
            session.import_snapshot_json(auto);
            setEchoes((prev) => [...prev, "Autosave restauré."]);
          } catch {
            /* ignore corrupt */
          }
        }
        setCad(session);
        setEntityCount(session.entity_count());
        setSolidCount(session.solid_count());
        if (desktopAvailable()) {
          try {
            const ping = await desktopPing();
            setEchoes((prev) => [
              ...prev,
              `Shell Tauri: ${ping ?? "ok"}`,
            ]);
          } catch {
            setEchoes((prev) => [...prev, "Shell Tauri détecté."]);
          }
        }
      })
      .catch((e: unknown) => setError(String(e)));
  }, []);

  // Autosave every ~8s when document changes.
  useEffect(() => {
    if (!cad) return;
    const t = window.setTimeout(() => {
      try {
        saveAutosave(cad.export_snapshot_json());
      } catch {
        /* ignore */
      }
    }, 800);
    return () => window.clearTimeout(t);
  }, [cad, revision]);

  const collab = useMemo(() => (cad ? new CollabClient(cad) : null), [cad]);

  useEffect(() => {
    if (!collab || !cad) return;
    collab.onStatus = setCollabStatus;
    collab.onEcho = (line) => setEchoes((prev) => [...prev, line].slice(-40));
    collab.onDoc = () => {
      setRevision((n) => n + 1);
      setEntityCount(cad.entity_count());
    };
    collab.onPeers = () => {
      try {
        setPeers(JSON.parse(cad.sync_peers_json()) as PeerPresence[]);
        setPeerTick((n) => n + 1);
      } catch {
        setPeers([]);
      }
    };
  }, [cad, collab]);

  const applyOutput = useCallback(
    (raw: string, session: CadSession) => {
      const out = parseCadOutput(raw);
      if (out.prompt_fr) {
        setPrompt(out.prompt_fr);
      } else if (!session.prompt_fr()) {
        setPrompt("");
      } else {
        setPrompt(session.prompt_fr());
      }
      if (out.echoes.length) {
        setEchoes((prev) => [...prev, ...out.echoes].slice(-40));
      }
      // Always refresh panels (selection / props) even when geometry unchanged.
      setRevision((n) => n + 1);
      if (out.document_changed) {
        collab?.flush();
      }
      setEntityCount(session.entity_count());
      try {
        setSolidCount(session.solid_count());
      } catch {
        setSolidCount(0);
      }
    },
    [collab],
  );

  const onOutput = useCallback(
    (raw: string) => {
      if (!cad) return;
      applyOutput(raw, cad);
    },
    [applyOutput, cad],
  );

  const onSubmit = useCallback(
    (text: string) => {
      if (!cad) return;
      const t = text.trim();
      if (!t) {
        applyOutput(cad.enter(), cad);
        return;
      }
      applyOutput(cad.submit_text(t), cad);
    },
    [applyOutput, cad],
  );

  const saveKelicad = useCallback(async () => {
    if (!cad) return;
    try {
      const snap = cad.export_snapshot_json();
      if (desktopAvailable()) {
        const native = await desktopSaveKelicad(
          snap,
          projectName || "dessin.kelicad",
        );
        if (native) {
          setProjectName(native.path.split(/[/\\]/).pop() || projectName);
          pushRecent(native.path, snap);
          saveAutosave(snap);
          setEchoes((prev) => [
            ...prev,
            `Enregistré (natif) ${native.path} · ${native.entities} ent. · ${native.blocks} blocs`,
          ]);
          return;
        }
      }
      const bytes = await snapshotToKelicadBytes(snap);
      downloadBytes(projectName || "dessin.kelicad", bytes);
      pushRecent(projectName || "dessin.kelicad", snap);
      saveAutosave(snap);
      setEchoes((prev) => [...prev, `Enregistré ${projectName}`]);
    } catch (e) {
      setEchoes((prev) => [...prev, `Erreur sauvegarde: ${String(e)}`]);
    }
  }, [cad, projectName]);

  const exportPdf = useCallback(() => {
    if (!cad) return;
    try {
      const raw = cad.export_pdf();
      const bytes =
        raw instanceof Uint8Array ? raw : Uint8Array.from(raw as number[]);
      downloadBytes(
        (projectName || "dessin").replace(/\.kelicad$/i, "") + ".pdf",
        bytes,
      );
      setEchoes((prev) => [...prev, "Export PDF"]);
    } catch (e) {
      setEchoes((prev) => [...prev, `Erreur PDF: ${String(e)}`]);
    }
  }, [cad, projectName]);

  const openKelicad = useCallback(async () => {
    if (!cad) return;
    if (desktopAvailable()) {
      try {
        const native = await desktopOpenKelicad();
        if (native) {
          cad.import_snapshot_json(native.snapshot_json);
          setProjectName(native.path.split(/[/\\]/).pop() || native.path);
          setRevision((n) => n + 1);
          setEntityCount(cad.entity_count());
          setSolidCount(cad.solid_count());
          pushRecent(native.path, native.snapshot_json);
          setEchoes((prev) => [
            ...prev,
            `Ouvert (natif) ${native.path} · ${native.entities} ent. · ${native.blocks} blocs`,
          ]);
          return;
        }
      } catch (e) {
        setEchoes((prev) => [...prev, `Ouverture native: ${String(e)}`]);
      }
    }
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".kelicad,application/octet-stream";
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return;
      void file.arrayBuffer().then(async (buf) => {
        try {
          const json = await kelicadBytesToSnapshotJson(new Uint8Array(buf));
          cad.import_snapshot_json(json);
          setProjectName(file.name);
          setRevision((n) => n + 1);
          setEntityCount(cad.entity_count());
          pushRecent(file.name, json);
          setEchoes((prev) => [...prev, `Ouvert ${file.name}`]);
        } catch (e) {
          setEchoes((prev) => [...prev, `Erreur ouverture: ${String(e)}`]);
        }
      });
    };
    input.click();
  }, [cad]);

  const downloadText = (filename: string, text: string, mime: string) => {
    const blob = new Blob([text], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  };

  const exportDxf = useCallback(() => {
    if (!cad) return;
    try {
      downloadText("dessin.dxf", cad.export_dxf(), "application/dxf");
      setEchoes((prev) => [...prev, "Export DXF"]);
    } catch (e) {
      setEchoes((prev) => [...prev, `Erreur DXF: ${String(e)}`]);
    }
  }, [cad]);

  const exportSvg = useCallback(() => {
    if (!cad) return;
    try {
      downloadText("dessin.svg", cad.export_svg(1200, 800), "image/svg+xml");
      setEchoes((prev) => [...prev, "Export SVG"]);
    } catch (e) {
      setEchoes((prev) => [...prev, `Erreur SVG: ${String(e)}`]);
    }
  }, [cad]);

  const importDxf = useCallback(() => {
    if (!cad) return;
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".dxf,text/plain";
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return;
      void file.text().then((text) => {
        try {
          const report = JSON.parse(cad.import_dxf(text)) as {
            imported: number;
            skipped: string[];
            warnings: string[];
          };
          setRevision((n) => n + 1);
          setEntityCount(cad.entity_count());
          setEchoes((prev) => [
            ...prev,
            `DXF import: ${report.imported} entité(s), ${report.skipped.length} ignorée(s)`,
            ...report.warnings.slice(0, 3),
          ]);
        } catch (e) {
          setEchoes((prev) => [...prev, `Erreur import DXF: ${String(e)}`]);
        }
      });
    };
    input.click();
  }, [cad]);

  if (error) {
    return (
      <div className="flex h-full items-center justify-center p-8 text-red-300">
        Échec chargement WASM : {error}
      </div>
    );
  }

  if (!cad || !collab) {
    return (
      <div className="flex h-full items-center justify-center font-mono text-ink-500">
        Chargement du cœur KELICAD…
      </div>
    );
  }

  const runCmd = useCallback(
    (cmd: string) => {
      setActiveCmd(cmd);
      applyOutput(cad.start_command(cmd), cad);
    },
    [applyOutput, cad],
  );

  const bump = useCallback(() => {
    setRevision((n) => n + 1);
    setEntityCount(cad.entity_count());
    setSolidCount(cad.solid_count());
  }, [cad]);

  const onSolidAction = useCallback(
    (action: string) => {
      try {
        switch (action) {
          case "box": {
            const s = cad.solid_box(0, 0, 0, 4, 3, 2.5);
            setEchoes((p) => [...p, `BOX: ${s}`]);
            setViewMode((m) => (m === "2d" ? "split" : m));
            break;
          }
          case "cyl":
            setEchoes((p) => [...p, `CYL: ${cad.solid_cylinder(2, 2, 1.2, 3)}`]);
            break;
          case "sph":
            setEchoes((p) => [...p, `SPH: ${cad.solid_sphere(0, 0, 1, 1)}`]);
            break;
          case "cone":
            setEchoes((p) => [...p, `CONE: ${cad.solid_cone(0, 0, 1.5, 3)}`]);
            break;
          case "tor":
            setEchoes((p) => [
              ...p,
              `TOR: ${cad.solid_torus(0, 0, 0, 2, 0.4)}`,
            ]);
            break;
          case "extrude": {
            const s = cad.solid_extrude_selected(2.5);
            const n = cad.view_derive_apply("plan");
            setEchoes((p) => [...p, `EXTRUDE: ${s}`, `Plan: +${n}`]);
            break;
          }
          case "rev":
            setEchoes((p) => [
              ...p,
              `REV: ${cad.solid_revolve_selected(360)}`,
            ]);
            break;
          case "union":
            setEchoes((p) => [...p, `BOOL+: ${cad.solid_boolean("union")}`]);
            break;
          case "diff":
            setEchoes((p) => [
              ...p,
              `BOOL−: ${cad.solid_boolean("difference")}`,
            ]);
            break;
          case "clear":
            cad.solids_clear();
            setEchoes((p) => [...p, "Solids cleared"]);
            break;
          case "view-plan":
            setEchoes((p) => [
              ...p,
              `Vue plan: +${cad.view_derive_apply("plan")}`,
            ]);
            break;
          case "view-elevx":
            setEchoes((p) => [
              ...p,
              `Élév X: +${cad.view_derive_apply("elevation_x")}`,
            ]);
            break;
          case "view-elevy":
            setEchoes((p) => [
              ...p,
              `Élév Y: +${cad.view_derive_apply("elevation_y")}`,
            ]);
            break;
          case "tpl-a4":
          case "tpl-a3":
          case "tpl-a1":
          case "tpl-arch":
          case "tpl-mech": {
            const name = action.replace("tpl-", "");
            const n = cad.apply_template(name);
            setEchoes((p) => [...p, `Gabarit ${name}: +${n}`]);
            break;
          }
          case "ifc":
            downloadText("model.ifc", cad.ifc_export(), "application/x-step");
            setEchoes((p) => [...p, "IFC exporté"]);
            break;
          case "sched":
            downloadText("schedule.csv", cad.bim_schedule_csv(), "text/csv");
            setEchoes((p) => [...p, "SCHED exporté"]);
            break;
          default:
            break;
        }
        bump();
      } catch (e) {
        setEchoes((p) => [...p, String(e)]);
      }
    },
    [bump, cad],
  );

  return (
    <div className="flex h-full flex-col bg-[#0e0e0e] text-[#e6e6e6]">
      <Ribbon
        cad={cad}
        revision={revision}
        projectName={projectName}
        viewMode={viewMode}
        entityCount={entityCount}
        solidCount={solidCount}
        onCommand={runCmd}
        onNew={() => {
          cad.new_drawing();
          setProjectName("dessin.kelicad");
          setEntityCount(0);
          setSolidCount(0);
          setRevision((n) => n + 1);
          setEchoes((p) => [...p, "Nouveau dessin."]);
        }}
        onOpen={() => void openKelicad()}
        onSave={() => void saveKelicad()}
        onUndo={() => applyOutput(cad.undo(), cad)}
        onRedo={() => applyOutput(cad.redo(), cad)}
        onPdf={exportPdf}
        onImportDxf={importDxf}
        onExportDxf={exportDxf}
        onExportSvg={exportSvg}
        onViewMode={setViewMode}
        onSolid={onSolidAction}
        onChanged={bump}
        onEcho={(lines) => setEchoes((p) => [...p, ...lines].slice(-40))}
      />

      <div className="min-h-0 flex flex-1">
        <ToolRail activeCmd={activeCmd} onCommand={runCmd} />

        <main className="cad-canvas-wrap relative flex min-h-0 min-w-0 flex-1 flex-col md:flex-row">
          {viewMode !== "3d" && (
            <div
              className={`relative min-h-0 min-w-0 ${
                viewMode === "split"
                  ? "h-1/2 md:h-full md:w-1/2"
                  : "h-full w-full"
              }`}
            >
              <DrawingViewport
                cad={cad}
                onOutput={onOutput}
                revision={revision}
                peers={peers}
                peerTick={peerTick}
                onHoverWorld={(x, y) => {
                  setCursor({ x, y });
                  if (collabStatus === "online") collab.sendPresence(x, y);
                }}
              />
              {viewMode === "2d" && (
                <div className="cad-navrail">
                  <button type="button" title="Zoom étendu">⊕</button>
                  <button type="button" title="Pan (Espace)">✋</button>
                </div>
              )}
            </div>
          )}
          {viewMode !== "2d" && (
            <div
              className={`relative min-h-0 min-w-0 ${
                viewMode === "split"
                  ? "h-1/2 border-t border-[#2e2e2e] md:h-full md:w-1/2 md:border-l md:border-t-0"
                  : "h-full w-full"
              }`}
            >
              <Viewport3D
                cad={cad}
                revision={revision}
                onEcho={(lines) =>
                  setEchoes((p) => [...p, ...lines].slice(-40))
                }
              />
              <div className="cad-viewcube">
                <button type="button" onClick={() => setViewMode("3d")}>
                  ISO
                </button>
                <button type="button" onClick={() => setViewMode("split")}>
                  SPLIT
                </button>
                <button type="button" onClick={() => setViewMode("2d")}>
                  2D
                </button>
              </div>
            </div>
          )}
        </main>

        <aside className="cad-dock">
          <div className="cad-dock-tabs">
            {(
              [
                ["layers", "Calques"],
                ["blocks", "Blocs"],
                ["props", "Props"],
                ["keli", "KELI"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                className={dockTab === id ? "is-active" : ""}
                onClick={() => setDockTab(id)}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="cad-dock-body">
            {dockTab === "layers" && (
              <>
                <LayerPanel
                  cad={cad}
                  revision={revision}
                  onChanged={bump}
                  onEcho={(lines) =>
                    setEchoes((p) => [...p, ...lines].slice(-40))
                  }
                />
                <SelectionPanel
                  cad={cad}
                  revision={revision}
                  onEcho={(lines) =>
                    setEchoes((p) => [...p, ...lines].slice(-40))
                  }
                  onChanged={() => setRevision((n) => n + 1)}
                />
              </>
            )}
            {dockTab === "blocks" && (
              <BlockLibrary
                cad={cad}
                revision={revision}
                onCommand={(raw) => applyOutput(raw, cad)}
                onEcho={(lines) =>
                  setEchoes((p) => [...p, ...lines].slice(-40))
                }
              />
            )}
            {dockTab === "props" && (
              <PropertiesPanel
                cad={cad}
                revision={revision}
                onChanged={bump}
                onEcho={(lines) =>
                  setEchoes((p) => [...p, ...lines].slice(-40))
                }
              />
            )}
            {dockTab === "keli" && (
              <>
                <CollabBar
                  collab={collab}
                  status={collabStatus}
                  name={displayName}
                  onName={setDisplayName}
                  onEcho={(line) =>
                    setEchoes((p) => [...p, line].slice(-40))
                  }
                />
                <KeliPanel
                  cad={cad}
                  revision={revision}
                  onChanged={() => {
                    bump();
                    collab.flush();
                  }}
                  onEcho={(lines) =>
                    setEchoes((p) => [...p, ...lines].slice(-40))
                  }
                />
              </>
            )}
          </div>
        </aside>
      </div>

      <CommandLine prompt={prompt} echoes={echoes} onSubmit={onSubmit} />
      <StatusBar
        cad={cad}
        revision={revision}
        cursor={cursor}
        layout={layout}
        onLayout={setLayout}
        onChanged={() => setRevision((n) => n + 1)}
      />
    </div>
  );
}
