import { useCallback, useEffect, useState } from "react";

import {
  downloadBytes,
  kelicadBytesToSnapshotJson,
  snapshotToKelicadBytes,
} from "./cad/kelicadFile";
import { loadCad } from "./cad/loadCad";
import { parseCadOutput, type CadSession } from "./cad/types";
import { CommandLine } from "./components/CommandLine";
import { DrawingViewport } from "./components/DrawingViewport";
import { KeliPanel } from "./components/KeliPanel";

const QUICK = [
  "L",
  "PL",
  "C",
  "A",
  "REC",
  "DT",
  "DLI",
  "H",
  "B",
  "I",
  "M",
  "CO",
  "RO",
  "SC",
  "MI",
  "O",
  "TR",
  "EX",
  "E",
  "U",
];

export function App() {
  const [cad, setCad] = useState<CadSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [echoes, setEchoes] = useState<string[]>([
    "KELICAD M3 — dessin + annotation + KELI (panneau à droite).",
  ]);
  const [revision, setRevision] = useState(0);
  const [entityCount, setEntityCount] = useState(0);

  useEffect(() => {
    void loadCad()
      .then((session) => setCad(session))
      .catch((e: unknown) => setError(String(e)));
  }, []);

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
      if (out.document_changed) {
        setRevision((n) => n + 1);
      }
      setEntityCount(session.entity_count());
    },
    [],
  );

  const onOutput = useCallback(
    (raw: string) => {
      if (!cad) {
        return;
      }
      applyOutput(raw, cad);
    },
    [applyOutput, cad],
  );

  const onSubmit = useCallback(
    (text: string) => {
      if (!cad) {
        return;
      }
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
    if (!cad) {
      return;
    }
    try {
      const snap = cad.export_snapshot_json();
      const bytes = await snapshotToKelicadBytes(snap);
      downloadBytes("dessin.kelicad", bytes);
      setEchoes((prev) => [...prev, "Enregistré dessin.kelicad"]);
    } catch (e) {
      setEchoes((prev) => [...prev, `Erreur sauvegarde: ${String(e)}`]);
    }
  }, [cad]);

  const openKelicad = useCallback(async () => {
    if (!cad) {
      return;
    }
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".kelicad,application/octet-stream";
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) {
        return;
      }
      void file.arrayBuffer().then(async (buf) => {
        try {
          const json = await kelicadBytesToSnapshotJson(new Uint8Array(buf));
          cad.import_snapshot_json(json);
          setRevision((n) => n + 1);
          setEntityCount(cad.entity_count());
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

  if (!cad) {
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
          <h1 className="text-base font-medium">Dessin 2D — M3 · KELI</h1>
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
        </div>
        <div className="font-mono text-xs text-ink-500">
          Entités <span className="text-ink-100">{entityCount}</span>
        </div>
      </header>
      <div className="flex min-h-0 flex-1">
        <main className="min-h-0 min-w-0 flex-1">
          <DrawingViewport cad={cad} onOutput={onOutput} revision={revision} />
        </main>
        <KeliPanel
          cad={cad}
          onChanged={() => {
            setRevision((n) => n + 1);
            setEntityCount(cad.entity_count());
          }}
          onEcho={(lines) =>
            setEchoes((prev) => [...prev, ...lines].slice(-40))
          }
        />
      </div>
      <CommandLine prompt={prompt} echoes={echoes} onSubmit={onSubmit} />
    </div>
  );
}
