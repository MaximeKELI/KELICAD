import { useEffect, useRef, useState } from "react";

import type { PeerPresence } from "../cad/collab";
import type { CadSession } from "../cad/types";
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
  const rendererRef = useRef<LineRenderer | null>(null);
  const hoverRef = useRef({ x: 0, y: 0 });
  const onHoverRef = useRef(onHoverWorld);
  onHoverRef.current = onHoverWorld;
  const [camVersion, setCamVersion] = useState(0);
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
        const scene = toFloat32(cad.scene_line_vertices());
        const ghost = toFloat32(
          cad.ghost_line_vertices(hoverRef.current.x, hoverRef.current.y),
        );
        const keliGhost = toFloat32(cad.keli_ghost_vertices());
        const merged = new Float32Array(
          scene.length + ghost.length + keliGhost.length,
        );
        merged.set(scene, 0);
        merged.set(ghost, scene.length);
        merged.set(keliGhost, scene.length + ghost.length);
        rendererRef.current.setLines(merged.length ? merged : new Float32Array([0, 0, 0, 0]));
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
        onOutput(cad.pick(w.x, w.y));
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
    };
    const onUp = (e: PointerEvent) => {
      dragging.current = false;
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
      <p className="pointer-events-none absolute bottom-3 left-3 rounded bg-ink-900/80 px-3 py-1 font-mono text-xs text-ink-100">
        Clic = point · Molette = zoom · Molette clic / Espace+glisser = pan · Échap = annuler
      </p>
    </div>
  );
}

function toFloat32(v: Float32Array | number[]): Float32Array {
  return v instanceof Float32Array ? v : new Float32Array(v);
}
