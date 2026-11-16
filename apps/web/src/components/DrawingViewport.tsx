import { useEffect, useRef, useState } from "react";

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
};

export function DrawingViewport({
  cad,
  onOutput,
  revision,
  peers = [],
  peerTick = 0,
  onHoverWorld,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const cameraRef = useRef<Camera2D>({ x: 5, y: 5, zoom: 20 });
  const spacePan = useRef(false);
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
  void peerTick;
  void camVersion;

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
          batches.push({ r: 1.0, g: 0.85, b: 0.35, verts: ghost });
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

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const cam = cameraRef.current;
      cam.zoom = Math.min(Math.max(cam.zoom * (e.deltaY > 0 ? 1.1 : 1 / 1.1), 1), 50_000);
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code === "Space") {
        spacePan.current = true;
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
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.code === "Space") {
        spacePan.current = false;
      }
    };

    const onDown = (e: PointerEvent) => {
      if (e.button === 1 || spacePan.current || e.button === 2) {
        dragging.current = true;
        lastPtr.current = { x: e.clientX, y: e.clientY };
        canvas.setPointerCapture(e.pointerId);
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
        if (dx > 4 || dy > 4) {
          const crossing = sd.clientX1 < sd.clientX0;
          onOutput(
            cad.select_rect(sd.x0, sd.y0, sd.x1, sd.y1, crossing, sd.shift),
          );
        } else if (sd.shift) {
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

  return (
    <div ref={wrapRef} className="relative h-full w-full overflow-hidden">
      <canvas
        ref={canvasRef}
        className="h-full w-full touch-none cursor-crosshair"
      />
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
      <p className="pointer-events-none absolute bottom-3 left-3 rounded bg-ink-900/80 px-3 py-1 font-mono text-xs text-ink-100">
        Clic = pick · Shift = multi · Glisser L→R fenêtre / R→L crossing · Espace+pan · Échap
      </p>
    </div>
  );
}

function toFloat32(v: Float32Array | number[]): Float32Array {
  return v instanceof Float32Array ? v : new Float32Array(v);
}
