import { useEffect, useRef, useState, type ReactNode } from "react";

import type { PeerPresence } from "../cad/collab";
import type { CadSession } from "../cad/types";
import type { SceneBatch } from "../cad/types";
import { type Camera2D, LineRenderer } from "../gpu/lineRenderer";

type Props = {
  cad: CadSession;
  onOutput: (raw: string) => void;
  revision: number;
  peers?: PeerPresence[];
  peerTick?: number;
  onHoverWorld?: (x: number, y: number) => void;
  paperLayout?: boolean;
};

export function DrawingViewport({
  cad,
  onOutput,
  revision,
  peers = [],
  peerTick = 0,
  onHoverWorld,
  paperLayout = false,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const cameraRef = useRef<Camera2D>({ x: 5, y: 5, zoom: 20 });
  const dragging = useRef(false);
  const lastPtr = useRef({ x: 0, y: 0 });
  const selectDrag = useRef<{
    active: boolean;
    x0: number;
    y0: number;
    x1: number;
    y1: number;
    clientX0: number;
    clientY0: number;
    clientX1: number;
    clientY1: number;
    shift: boolean;
  } | null>(null);
  const rendererRef = useRef<LineRenderer | null>(null);
  const hoverRef = useRef({ x: 0, y: 0 });
  const onHoverRef = useRef(onHoverWorld);
  onHoverRef.current = onHoverWorld;
  const [camVersion, setCamVersion] = useState(0);
  const [band, setBand] = useState<{
    left: number;
    top: number;
    width: number;
    height: number;
    crossing: boolean;
  } | null>(null);
  const [gridOn, setGridOn] = useState(true);
  const [gridUnit, setGridUnit] = useState(10);
  void peerTick;

  useEffect(() => {
    const canvas = canvasRef.current;
    const wrap = wrapRef.current;
    if (!canvas || !wrap) {
      return;
    }
    let disposed = false;
    let raf = 0;

    void (async () => {
      const renderer = await LineRenderer.create(canvas);
      if (disposed) {
        renderer?.destroy();
        return;
      }
      if (!renderer) {
        return;
      }
      rendererRef.current = renderer;
      cad.set_snap_aperture(0.35);

      const tick = () => {
        if (disposed || !rendererRef.current) {
          return;
        }
        const rect = wrap.getBoundingClientRect();
        rendererRef.current.resize(rect.width, rect.height);
        let batches: { r: number; g: number; b: number; verts: Float32Array }[] =
          [];
        try {
          const raw = JSON.parse(cad.scene_batches_json()) as SceneBatch[];
          batches = raw
            .filter((b) => b.verts && (b.verts as number[]).length > 0)
            .map((b) => ({
              r: b.r,
              g: b.g,
              b: b.b,
              verts: toFloat32(b.verts),
            }));
        } catch {
          const scene = toFloat32(cad.scene_line_vertices());
          if (scene.length) {
            batches = [{ r: 0.55, g: 0.78, b: 1.0, verts: scene }];
          }
        }
        const ghost = toFloat32(
          cad.ghost_line_vertices(hoverRef.current.x, hoverRef.current.y),
        );
        const keliGhost = toFloat32(cad.keli_ghost_vertices());
        if (ghost.length) {
          // Rubber-band preview (bright cyan) — always on top of scene.
          batches.push({ r: 0.25, g: 1.0, b: 0.95, verts: ghost });
        }
        if (keliGhost.length) {
          batches.push({ r: 0.45, g: 1.0, b: 0.7, verts: keliGhost });
        }
        if (!batches.length) {
          batches = [{ r: 0.55, g: 0.78, b: 1.0, verts: new Float32Array([0, 0, 0, 0]) }];
        }
        rendererRef.current.setBatches(batches);
        rendererRef.current.draw(cameraRef.current);
        raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    })();

    const worldFromClient = (clientX: number, clientY: number) => {
      const rect = wrap.getBoundingClientRect();
      const cam = cameraRef.current;
      const aspect = rect.width / Math.max(rect.height, 1);
      const halfH = cam.zoom * 0.5;
      const halfW = halfH * aspect;
      const nx = (clientX - rect.left) / Math.max(rect.width, 1);
      const ny = (clientY - rect.top) / Math.max(rect.height, 1);
      return {
        x: cam.x - halfW + nx * halfW * 2,
        y: cam.y + halfH - ny * halfH * 2,
      };
    };

    let wheelRaf = 0;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const cam = cameraRef.current;
      const before = worldFromClient(e.clientX, e.clientY);
      const factor = e.deltaY > 0 ? 1.12 : 1 / 1.12;
      // zoom = hauteur monde visible ; ↑ = dézoom. Pas de plafond artificiel bas.
      cam.zoom = Math.min(Math.max(cam.zoom * factor, 0.05), 1e12);
      const after = worldFromClient(e.clientX, e.clientY);
      cam.x += before.x - after.x;
      cam.y += before.y - after.y;
      // Batch overlay refresh — avoid React thrash on every wheel tick.
      if (!wheelRaf) {
        wheelRaf = requestAnimationFrame(() => {
          wheelRaf = 0;
          setCamVersion((n) => n + 1);
        });
      }
    };

    const isTypingTarget = (t: EventTarget | null) => {
      if (!(t instanceof HTMLElement)) return false;
      const tag = t.tagName;
      return (
        tag === "INPUT" ||
        tag === "TEXTAREA" ||
        tag === "SELECT" ||
        t.isContentEditable
      );
    };

    const onKeyDown = (e: KeyboardEvent) => {
      // Middle-mouse pans; Space/Enter finish the active command (AutoCAD-like).
      if (
        (e.key === "Enter" || e.code === "Space") &&
        !isTypingTarget(e.target)
      ) {
        if (e.repeat) return;
        e.preventDefault();
        onOutput(cad.enter());
        return;
      }
      if (e.key === "Escape") {
        onOutput(cad.cancel());
      }
      if (e.key === "F8") {
        e.preventDefault();
        cad.set_ortho(!cad.ortho());
        onOutput(
          JSON.stringify({
            echoes: [`ORTHO ${cad.ortho() ? "ON" : "OFF"}`],
            document_changed: true,
            ghost_count: 0,
          }),
        );
      }
      if (e.key === "F3") {
        e.preventDefault();
        cad.set_osnap(!cad.osnap());
        onOutput(
          JSON.stringify({
            echoes: [`OSNAP ${cad.osnap() ? "ON" : "OFF"}`],
            document_changed: true,
            ghost_count: 0,
          }),
        );
      }
      if (e.key === "F7") {
        e.preventDefault();
        const on = (() => {
          try {
            return JSON.parse(cad.status_json()).grid as boolean;
          } catch {
            return false;
          }
        })();
        cad.set_sysvar("GRIDMODE", on ? "0" : "1");
        onOutput(
          JSON.stringify({
            echoes: [`GRID ${on ? "OFF" : "ON"}`],
            document_changed: true,
            ghost_count: 0,
          }),
        );
      }
      if (e.key === "F9") {
        e.preventDefault();
        const on = (() => {
          try {
            return JSON.parse(cad.status_json()).snap as boolean;
          } catch {
            return false;
          }
        })();
        cad.set_sysvar("SNAPMODE", on ? "0" : "1");
        onOutput(
          JSON.stringify({
            echoes: [`SNAP ${on ? "OFF" : "ON"}`],
            document_changed: true,
            ghost_count: 0,
          }),
        );
      }
      if (e.key === "F10") {
        e.preventDefault();
        const on = (() => {
          try {
            return JSON.parse(cad.status_json()).polar as boolean;
          } catch {
            return false;
          }
        })();
        cad.set_sysvar("POLARMODE", on ? "0" : "1");
        onOutput(
          JSON.stringify({
            echoes: [`POLAR ${on ? "OFF" : "ON"}`],
            document_changed: true,
            ghost_count: 0,
          }),
        );
      }
    };
    const onKeyUp = (_e: KeyboardEvent) => {
      /* Space no longer toggles pan — use molette/clic milieu */
    };

    const onDown = (e: PointerEvent) => {
      // Pan: middle button (or Alt+left). Right-click finishes command (AutoCAD).
      if (e.button === 1 || (e.button === 0 && e.altKey)) {
        dragging.current = true;
        lastPtr.current = { x: e.clientX, y: e.clientY };
        canvas.setPointerCapture(e.pointerId);
        return;
      }
      if (e.button === 2) {
        e.preventDefault();
        onOutput(cad.enter());
        return;
      }
      if (e.button === 0) {
        const w = worldFromClient(e.clientX, e.clientY);
        selectDrag.current = {
          active: true,
          x0: w.x,
          y0: w.y,
          x1: w.x,
          y1: w.y,
          clientX0: e.clientX,
          clientY0: e.clientY,
          clientX1: e.clientX,
          clientY1: e.clientY,
          shift: e.shiftKey,
        };
        canvas.setPointerCapture(e.pointerId);
      }
    };
    const onMove = (e: PointerEvent) => {
      const w = worldFromClient(e.clientX, e.clientY);
      hoverRef.current = w;
      onHoverRef.current?.(w.x, w.y);
      if (dragging.current) {
        const rect = wrap.getBoundingClientRect();
        const cam = cameraRef.current;
        const aspect = rect.width / Math.max(rect.height, 1);
        const worldPerPxY = cam.zoom / Math.max(rect.height, 1);
        const worldPerPxX = (cam.zoom * aspect) / Math.max(rect.width, 1);
        cam.x -= (e.clientX - lastPtr.current.x) * worldPerPxX;
        cam.y += (e.clientY - lastPtr.current.y) * worldPerPxY;
        lastPtr.current = { x: e.clientX, y: e.clientY };
        setCamVersion((n) => n + 1);
      }
      const sd = selectDrag.current;
      if (sd?.active) {
        sd.x1 = w.x;
        sd.y1 = w.y;
        sd.clientX1 = e.clientX;
        sd.clientY1 = e.clientY;
        const rect = wrap.getBoundingClientRect();
        const left = Math.min(sd.clientX0, sd.clientX1) - rect.left;
        const top = Math.min(sd.clientY0, sd.clientY1) - rect.top;
        const width = Math.abs(sd.clientX1 - sd.clientX0);
        const height = Math.abs(sd.clientY1 - sd.clientY0);
        // AutoCAD: left→right = window (contain), right→left = crossing
        const crossing = sd.clientX1 < sd.clientX0;
        if (width > 4 || height > 4) {
          setBand({ left, top, width, height, crossing });
        } else {
          setBand(null);
        }
      }
    };
    const onUp = (e: PointerEvent) => {
      dragging.current = false;
      const sd = selectDrag.current;
      selectDrag.current = null;
      setBand(null);
      if (sd?.active && e.button === 0) {
        const dx = Math.abs(sd.clientX1 - sd.clientX0);
        const dy = Math.abs(sd.clientY1 - sd.clientY0);
        const prompt = cad.prompt_fr()?.trim() ?? "";
        const gathering =
          /sélectionnez/i.test(prompt) || /select objects/i.test(prompt);
        // Pendant dessin (LINE…), toujours pick. Pendant sélection (MOVE/ERASE…),
        // autoriser la fenêtre.
        if (gathering && (dx > 4 || dy > 4)) {
          const crossing = sd.clientX1 < sd.clientX0;
          onOutput(
            cad.select_rect(sd.x0, sd.y0, sd.x1, sd.y1, crossing, true),
          );
        } else if (!prompt && (dx > 4 || dy > 4)) {
          const crossing = sd.clientX1 < sd.clientX0;
          onOutput(
            cad.select_rect(sd.x0, sd.y0, sd.x1, sd.y1, crossing, sd.shift),
          );
        } else if (sd.shift && !prompt) {
          onOutput(cad.pick_toggle(sd.x0, sd.y0));
        } else {
          onOutput(cad.pick(sd.x0, sd.y0));
        }
      }
      try {
        canvas.releasePointerCapture(e.pointerId);
      } catch {
        /* ignore */
      }
    };
    const onContext = (e: Event) => e.preventDefault();

    canvas.addEventListener("wheel", onWheel, { passive: false });
    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointermove", onMove);
    canvas.addEventListener("pointerup", onUp);
    canvas.addEventListener("contextmenu", onContext);
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      if (wheelRaf) cancelAnimationFrame(wheelRaf);
      canvas.removeEventListener("wheel", onWheel);
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerup", onUp);
      canvas.removeEventListener("contextmenu", onContext);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      rendererRef.current?.destroy();
      rendererRef.current = null;
    };
  }, [cad, onOutput]);

  useEffect(() => {
    // revision bump forces scene refresh on next RAF via cad.scene_line_vertices()
  }, [revision]);

  useEffect(() => {
    const onView = (ev: Event) => {
      const action = (ev as CustomEvent<{ action?: string }>).detail?.action;
      if (!action) return;
      const cam = cameraRef.current;
      const wrap = wrapRef.current;
      if (action === "zoom-in") {
        cam.zoom = Math.max(cam.zoom / 1.25, 0.05);
        setCamVersion((n) => n + 1);
        return;
      }
      if (action === "zoom-out") {
        cam.zoom = Math.min(cam.zoom * 1.25, 1e12);
        setCamVersion((n) => n + 1);
        return;
      }
      if (action === "extents") {
        const verts = toFloat32(cad.scene_line_vertices());
        if (verts.length < 4) {
          cam.x = 5;
          cam.y = 5;
          cam.zoom = 20;
          setCamVersion((n) => n + 1);
          return;
        }
        let minX = Infinity;
        let minY = Infinity;
        let maxX = -Infinity;
        let maxY = -Infinity;
        for (let i = 0; i < verts.length; i += 2) {
          minX = Math.min(minX, verts[i]!);
          maxX = Math.max(maxX, verts[i]!);
          minY = Math.min(minY, verts[i + 1]!);
          maxY = Math.max(maxY, verts[i + 1]!);
        }
        const cx = (minX + maxX) * 0.5;
        const cy = (minY + maxY) * 0.5;
        const w = Math.max(maxX - minX, 1e-6);
        const h = Math.max(maxY - minY, 1e-6);
        const aspect =
          wrap && wrap.clientHeight > 0
            ? wrap.clientWidth / wrap.clientHeight
            : 1.5;
        const pad = 1.15;
        const zoomH = h * pad;
        const zoomW = (w * pad) / aspect;
        cam.x = cx;
        cam.y = cy;
        cam.zoom = Math.min(Math.max(Math.max(zoomH, zoomW), 0.05), 1e12);
        setCamVersion((n) => n + 1);
      }
    };
    window.addEventListener("kelicad-view", onView);
    return () => window.removeEventListener("kelicad-view", onView);
  }, [cad]);

  useEffect(() => {
    try {
      const st = JSON.parse(cad.status_json()) as {
        grid?: boolean;
        grid_unit?: number;
      };
      setGridOn(!!st.grid);
      setGridUnit(Math.max(st.grid_unit ?? 10, 0.1));
    } catch {
      setGridOn(false);
    }
  }, [cad, revision, camVersion]);

  const clientFromWorld = (wx: number, wy: number) => {
    const wrap = wrapRef.current;
    if (!wrap) return { x: 0, y: 0 };
    const rect = wrap.getBoundingClientRect();
    const cam = cameraRef.current;
    const aspect = rect.width / Math.max(rect.height, 1);
    const halfH = cam.zoom * 0.5;
    const halfW = halfH * aspect;
    const nx = (wx - (cam.x - halfW)) / (halfW * 2);
    const ny = ((cam.y + halfH) - wy) / (halfH * 2);
    return { x: nx * rect.width, y: ny * rect.height };
  };

  const gridLines = (() => {
    if (!gridOn) return null;
    const wrap = wrapRef.current;
    if (!wrap) return null;
    const rect = wrap.getBoundingClientRect();
    const cam = cameraRef.current;
    const aspect = rect.width / Math.max(rect.height, 1);
    const halfH = cam.zoom * 0.5;
    const halfW = halfH * aspect;
    const minX = cam.x - halfW;
    const maxX = cam.x + halfW;
    const minY = cam.y - halfH;
    const maxY = cam.y + halfH;
    let step = gridUnit;
    const worldPerPx = (halfH * 2) / Math.max(rect.height, 1);
    while (step / worldPerPx < 8) step *= 2;
    while (step / worldPerPx > 40) step *= 0.5;
    step = Math.max(step, 1e-6);
    const majorEvery = 5;
    const lines: ReactNode[] = [];
    const x0 = Math.floor(minX / step) * step;
    const y0 = Math.floor(minY / step) * step;
    let i = 0;
    const maxLines = 120;
    const isMajor = (v: number) => Math.round(v / step) % majorEvery === 0;
    for (let x = x0; x <= maxX + step * 0.5 && i < maxLines; x += step) {
      const a = clientFromWorld(x, minY);
      const b = clientFromWorld(x, maxY);
      const maj = isMajor(x);
      lines.push(
        <line
          key={`vx${i++}`}
          x1={a.x}
          y1={a.y}
          x2={b.x}
          y2={b.y}
          stroke={maj ? "rgba(150,165,180,0.42)" : "rgba(100,110,125,0.22)"}
          strokeWidth={maj ? 1.25 : 1}
        />,
      );
    }
    for (let y = y0; y <= maxY + step * 0.5 && i < maxLines * 2; y += step) {
      const a = clientFromWorld(minX, y);
      const b = clientFromWorld(maxX, y);
      const maj = isMajor(y);
      lines.push(
        <line
          key={`hy${i++}`}
          x1={a.x}
          y1={a.y}
          x2={b.x}
          y2={b.y}
          stroke={maj ? "rgba(150,165,180,0.42)" : "rgba(100,110,125,0.22)"}
          strokeWidth={maj ? 1.25 : 1}
        />,
      );
    }
    return lines;
  })();

  const goTopView = () => {
    window.dispatchEvent(
      new CustomEvent("kelicad-view", { detail: { action: "extents" } }),
    );
  };

  return (
    <div
      ref={wrapRef}
      className={`cad-viewport relative h-full w-full overflow-hidden ${gridOn ? "has-grid" : ""}`}
    >
      <canvas
        ref={canvasRef}
        className="h-full w-full touch-none cursor-crosshair"
      />
      {gridOn && (
        <svg className="pointer-events-none absolute inset-0 z-[1] h-full w-full">
          {gridLines}
        </svg>
      )}

      {/* Viewport chrome — AutoCAD [-][Haut][Filaire 2D] */}
      <div className="cad-vp-chrome pointer-events-none absolute left-2 top-2 z-[15]">
        <span>[-]</span>
        <span>[Haut]</span>
        <span>[Filaire 2D]</span>
        {gridOn && <span className="cad-vp-chrome-on">[Grille]</span>}
      </div>

      {/* Cercle d'orientation (ViewCube) */}
      <div className="cad-orient" title="Orientation — HAUT = zoom étendu">
        <svg viewBox="0 0 100 100" className="cad-orient-ring" aria-hidden>
          <circle cx="50" cy="50" r="46" className="cad-orient-outer" />
          <circle cx="50" cy="50" r="28" className="cad-orient-inner" />
        </svg>
        <button type="button" className="cad-orient-n" onClick={goTopView}>
          N
        </button>
        <button type="button" className="cad-orient-e" onClick={goTopView}>
          E
        </button>
        <button type="button" className="cad-orient-s" onClick={goTopView}>
          S
        </button>
        <button type="button" className="cad-orient-o" onClick={goTopView}>
          O
        </button>
        <button type="button" className="cad-orient-core" onClick={goTopView}>
          HAUT
        </button>
        <button type="button" className="cad-orient-scg" onClick={goTopView}>
          SCG
        </button>
      </div>

      {/* UCS fixe — toujours coin bas-gauche du viewport */}
      <svg
        className="cad-ucs pointer-events-none absolute z-[12]"
        width="70"
        height="70"
        aria-hidden
      >
        <line x1="10" y1="60" x2="52" y2="60" stroke="#e8e8e8" strokeWidth="2" />
        <polygon points="52,55 64,60 52,65" fill="#e8e8e8" />
        <text x="50" y="52" fill="#e8e8e8" fontSize="12" fontFamily="IBM Plex Mono, monospace">
          X
        </text>
        <line x1="10" y1="60" x2="10" y2="18" stroke="#e8e8e8" strokeWidth="2" />
        <polygon points="5,18 10,6 15,18" fill="#e8e8e8" />
        <text x="16" y="16" fill="#e8e8e8" fontSize="12" fontFamily="IBM Plex Mono, monospace">
          Y
        </text>
        <circle cx="10" cy="60" r="3" fill="#e8e8e8" />
      </svg>

      {paperLayout && (
        <div className="pointer-events-none absolute inset-6 z-[2] rounded-sm border border-[#8a7a4a]/60 bg-[#1a1812]/35 shadow-[inset_0_0_0_1px_rgba(200,180,100,0.15)]">
          <span className="absolute left-2 top-1 font-mono text-[10px] text-[#cbb56a]">
            Layout1 · espace papier
          </span>
        </div>
      )}
      {peers.map((p) => {
        if (!p.cursor) return null;
        const pos = clientFromWorld(p.cursor.x, p.cursor.y);
        const color = `rgb(${p.color[0]},${p.color[1]},${p.color[2]})`;
        return (
          <div
            key={p.peer_id}
            className="pointer-events-none absolute z-10 -translate-x-1/2 -translate-y-1/2"
            style={{ left: pos.x, top: pos.y }}
          >
            <div className="h-2 w-2 rounded-full" style={{ background: color }} />
            <span
              className="ml-1 whitespace-nowrap rounded px-1 text-[10px] text-ink-950"
              style={{ background: color }}
            >
              {p.name}
            </span>
          </div>
        );
      })}
      {band && (
        <div
          className="pointer-events-none absolute z-20 border"
          style={{
            left: band.left,
            top: band.top,
            width: band.width,
            height: band.height,
            borderColor: band.crossing ? "rgb(80,200,120)" : "rgb(80,160,255)",
            background: band.crossing
              ? "rgba(80,200,120,0.12)"
              : "rgba(80,160,255,0.10)",
            borderStyle: band.crossing ? "dashed" : "solid",
          }}
        />
      )}
    </div>
  );
}

function toFloat32(v: Float32Array | number[]): Float32Array {
  return v instanceof Float32Array ? v : new Float32Array(v);
}
