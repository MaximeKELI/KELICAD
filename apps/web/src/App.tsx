import { useCallback, useEffect, useMemo, useState } from "react";

import { CollabClient, type CollabStatus, type PeerPresence } from "./cad/collab";
import {
  downloadBytes,
  kelicadBytesToSnapshotJson,
  snapshotToKelicadBytes,
} from "./cad/kelicadFile";
import { loadCad } from "./cad/loadCad";
import {
  listRecent,
  loadAutosave,
  pushRecent,
  saveAutosave,
} from "./cad/projectStore";
import { parseCadOutput, type CadSession } from "./cad/types";
import { BlockLibrary } from "./components/BlockLibrary";
import { CollabBar } from "./components/CollabBar";
import { CommentsPanel } from "./components/CommentsPanel";
import { CommandLine } from "./components/CommandLine";
import { DrawingViewport } from "./components/DrawingViewport";
import { KeliPanel } from "./components/KeliPanel";
import { LayerPanel } from "./components/LayerPanel";
import { PropertiesPanel } from "./components/PropertiesPanel";
import { SelectionPanel } from "./components/SelectionPanel";
import { ShapePalette } from "./components/ShapePalette";
import { StatusBar } from "./components/StatusBar";

const QUICK = [
  "M",
  "CO",
  "RO",
  "SC",
  "MI",
  "O",
  "TR",
  "EX",
  "F",
  "CHA",
  "STRETCH",
  "AR",
  "APOLAR",
  "DLI",
  "DCO",
  "DBA",
  "SEC",
  "H",
  "B",
  "I",
  "E",
  "U",
];

export function App() {
  const [cad, setCad] = useState<CadSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [echoes, setEchoes] = useState<string[]>([
    "KELICAD P1+ — blocs rendus · fenêtre/crossing · grips · gabarits A0–A4/arch · DIMCONTINUE · props.",
  ]);
  const [revision, setRevision] = useState(0);
  const [entityCount, setEntityCount] = useState(0);
  const [projectName, setProjectName] = useState("dessin.kelicad");
  const [collabStatus, setCollabStatus] = useState<CollabStatus>("disconnected");
  const [displayName, setDisplayName] = useState("Alice");
  const [peers, setPeers] = useState<PeerPresence[]>([]);
  const [peerTick, setPeerTick] = useState(0);

  useEffect(() => {
    void loadCad()
      .then((session) => {
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

  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-ink-700/60 bg-ink-900 px-4 py-2">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-accent-soft">KELICAD</p>
          <h1 className="text-base font-medium">2D · 3D · KELI</h1>
          <p className="font-mono text-[10px] text-ink-500">{projectName}</p>
        </div>
        <div className="flex flex-wrap gap-1">
          {QUICK.map((cmd) => (
            <button
              key={cmd}
              type="button"
              className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
              onClick={() => applyOutput(cad.start_command(cmd), cad)}
            >
              {cmd}
            </button>
          ))}
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => {
              cad.new_drawing();
              setProjectName("dessin.kelicad");
              setRevision((n) => n + 1);
              setEntityCount(0);
              setEchoes((prev) => [...prev, "Nouveau dessin."]);
            }}
          >
            NEW
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => void openKelicad()}
          >
            OPEN
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => void saveKelicad()}
          >
            SAVE
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => applyOutput(cad.undo(), cad)}
          >
            UNDO
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => applyOutput(cad.redo(), cad)}
          >
            REDO
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={importDxf}
          >
            DXF↓
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={exportDxf}
          >
            DXF↑
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={exportSvg}
          >
            SVG
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={exportPdf}
          >
            PDF
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => {
              const recent = listRecent();
              if (!recent.length) {
                setEchoes((prev) => [...prev, "Aucun fichier récent."]);
                return;
              }
              const pick = recent[0];
              try {
                cad.import_snapshot_json(pick.snapshotJson);
                setProjectName(pick.name);
                setRevision((n) => n + 1);
                setEntityCount(cad.entity_count());
                setEchoes((prev) => [...prev, `Récent : ${pick.name}`]);
              } catch (e) {
                setEchoes((prev) => [...prev, `Récent: ${String(e)}`]);
              }
            }}
          >
            RÉCENT
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => {
              const json = cad.macro_record_json();
              downloadText("macro.json", json, "application/json");
              setEchoes((prev) => [...prev, "Macro exportée (macro.json)"]);
            }}
          >
            MACRO
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => {
              try {
                const sel = JSON.parse(cad.selection_json()) as string[];
                const id = sel[0];
                if (!id) {
                  setEchoes((prev) => [
                    ...prev,
                    "CONSTR: sélectionnez une ligne (Horizontal+Distance)",
                  ]);
                  return;
                }
                cad.constraint_add_json(
                  JSON.stringify({ kind: "horizontal", entity: id }),
                );
                cad.constraint_add_json(
                  JSON.stringify({
                    kind: "distance",
                    entity: id,
                    value: 5,
                  }),
                );
                const result = cad.constraint_solve();
                setRevision((n) => n + 1);
                setEntityCount(cad.entity_count());
                setEchoes((prev) => [...prev, `CONSTR solve: ${result}`]);
              } catch (e) {
                setEchoes((prev) => [...prev, `CONSTR erreur: ${String(e)}`]);
              }
            }}
          >
            CONSTR
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => {
              const input = document.createElement("input");
              input.type = "file";
              input.accept = ".svg,.png,image/svg+xml,image/png";
              input.onchange = () => {
                const file = input.files?.[0];
                if (!file) return;
                void file.arrayBuffer().then((buf) => {
                  try {
                    const n = cad.vectorize_bytes(
                      new Uint8Array(buf),
                      file.name,
                    );
                    setRevision((r) => r + 1);
                    setEntityCount(cad.entity_count());
                    setEchoes((prev) => [
                      ...prev,
                      `VECTORIZE: ${n} entité(s) depuis ${file.name}`,
                    ]);
                  } catch (e) {
                    setEchoes((prev) => [
                      ...prev,
                      `VECTORIZE erreur: ${String(e)}`,
                    ]);
                  }
                });
              };
              input.click();
            }}
          >
            VECTORIZE
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => {
              try {
                const summary = cad.solid_extrude_selected(2.5);
                const n = cad.view_derive_apply("plan");
                setRevision((r) => r + 1);
                setEntityCount(cad.entity_count());
                setEchoes((prev) => [
                  ...prev,
                  `EXTRUDE: ${summary}`,
                  `Vue plan: ${n} géométrie(s); solids=${cad.solid_count()}`,
                ]);
              } catch (e) {
                setEchoes((prev) => [...prev, `EXTRUDE erreur: ${String(e)}`]);
              }
            }}
          >
            EXTRUDE
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => {
              const csv = cad.bim_schedule_csv();
              downloadText("schedule.csv", csv, "text/csv");
              setEchoes((prev) => [...prev, "SCHED exportée (schedule.csv)"]);
            }}
          >
            SCHED
          </button>
          <button
            type="button"
            className="rounded border border-ink-700 px-2 py-1 font-mono text-xs hover:border-accent-soft"
            onClick={() => {
              try {
                const ifc = cad.ifc_export();
                downloadText("model.ifc", ifc, "application/x-step");
                setEchoes((prev) => [...prev, "IFC↑ exporté (model.ifc)"]);
              } catch (e) {
                setEchoes((prev) => [...prev, `IFC erreur: ${String(e)}`]);
              }
            }}
          >
            IFC↑
          </button>
        </div>
        <div className="font-mono text-xs text-ink-500">
          Entités <span className="text-ink-100">{entityCount}</span>
        </div>
      </header>
      <CollabBar
        collab={collab}
        status={collabStatus}
        name={displayName}
        onName={setDisplayName}
        onEcho={(line) => setEchoes((prev) => [...prev, line].slice(-40))}
      />
      <div className="flex min-h-0 flex-1">
        <aside className="flex h-full w-52 shrink-0 flex-col border-r border-ink-700/60 bg-ink-900/90">
          <LayerPanel
            cad={cad}
            revision={revision}
            onChanged={() => {
              setRevision((n) => n + 1);
              setEntityCount(cad.entity_count());
            }}
            onEcho={(lines) => setEchoes((prev) => [...prev, ...lines].slice(-40))}
          />
          <div className="min-h-0 flex-1 overflow-hidden">
            <ShapePalette
              cad={cad}
              onCommand={(raw) => applyOutput(raw, cad)}
              onEcho={(lines) =>
                setEchoes((prev) => [...prev, ...lines].slice(-40))
              }
              onChanged={() => {
                setRevision((n) => n + 1);
                setEntityCount(cad.entity_count());
                collab.flush();
              }}
            />
          </div>
          <SelectionPanel
            cad={cad}
            revision={revision}
            onEcho={(lines) => setEchoes((prev) => [...prev, ...lines].slice(-40))}
            onChanged={() => setRevision((n) => n + 1)}
          />
          <BlockLibrary
            cad={cad}
            revision={revision}
            onCommand={(raw) => applyOutput(raw, cad)}
            onEcho={(lines) => setEchoes((prev) => [...prev, ...lines].slice(-40))}
          />
        </aside>
        <main className="min-h-0 min-w-0 flex-1">
          <DrawingViewport
            cad={cad}
            onOutput={onOutput}
            revision={revision}
            peers={peers}
            peerTick={peerTick}
            onHoverWorld={(x, y) => {
              if (collabStatus === "online") {
                collab.sendPresence(x, y);
              }
            }}
          />
        </main>
        <PropertiesPanel
          cad={cad}
          revision={revision}
          onChanged={() => {
            setRevision((n) => n + 1);
            setEntityCount(cad.entity_count());
          }}
          onEcho={(lines) => setEchoes((prev) => [...prev, ...lines].slice(-40))}
        />
        <CommentsPanel
          cad={cad}
          collab={collab}
          revision={revision}
          onEcho={(line) => setEchoes((prev) => [...prev, line].slice(-40))}
        />
        <KeliPanel
          cad={cad}
          revision={revision}
          onChanged={() => {
            setRevision((n) => n + 1);
            setEntityCount(cad.entity_count());
            collab.flush();
          }}
          onEcho={(lines) => setEchoes((prev) => [...prev, ...lines].slice(-40))}
        />
      </div>
      <CommandLine prompt={prompt} echoes={echoes} onSubmit={onSubmit} />
      <StatusBar
        cad={cad}
        revision={revision}
        onChanged={() => setRevision((n) => n + 1)}
      />
    </div>
  );
}
