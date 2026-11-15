import { useEffect, useRef, useState } from "react";

import { demoBounds, generateDemoLineVertices } from "../demoLines";
import { type Camera2D, LineRenderer } from "../gpu/lineRenderer";

const LINE_COUNT = 100_000;

type Props = {
  onFps: (fps: number) => void;
  onGpuReady: (ok: boolean, message: string) => void;
};

export function CanvasViewport({ onFps, onGpuReady }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const cameraRef = useRef<Camera2D>({ x: 0, y: 0, zoom: 80 });
  const dragging = useRef(false);
  const lastPtr = useRef({ x: 0, y: 0 });
  const [hint, setHint] = useState("Initialisation WebGPU…");

  useEffect(() => {
    const canvas = canvasRef.current;
    const wrap = wrapRef.current;
    if (!canvas || !wrap) {
      return;
    }

    let disposed = false;
    let renderer: LineRenderer | null = null;
    let raf = 0;
    let frames = 0;
    let lastFpsTs = performance.now();

    const bounds = demoBounds(LINE_COUNT);
    cameraRef.current = {
      x: bounds.width * 0.5,
      y: bounds.height * 0.5,
      zoom: Math.max(bounds.width, bounds.height) * 0.35,
    };

    void (async () => {
      renderer = await LineRenderer.create(canvas);
      if (disposed) {
        renderer?.destroy();
        return;
      }
      if (!renderer) {
        setHint("WebGPU indisponible — utilisez Chrome/Edge récents.");
        onGpuReady(false, "WebGPU unavailable");
        return;
      }
      const vertices = generateDemoLineVertices(LINE_COUNT);
      renderer.setLines(vertices);
      setHint(`${LINE_COUNT.toLocaleString("fr-FR")} lignes — molette = zoom, glisser = pan`);
      onGpuReady(true, "WebGPU ready");

      const tick = (now: number) => {
        if (disposed || !renderer) {
          return;
        }
        const rect = wrap.getBoundingClientRect();
        renderer.resize(rect.width, rect.height);
        renderer.draw(cameraRef.current);
        frames += 1;
        if (now - lastFpsTs >= 500) {
          onFps(Math.round((frames * 1000) / (now - lastFpsTs)));
          frames = 0;
          lastFpsTs = now;
        }
        raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    })();

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const cam = cameraRef.current;
      const factor = e.deltaY > 0 ? 1.1 : 1 / 1.1;
      cam.zoom = Math.min(Math.max(cam.zoom * factor, 5), 50_000);
    };

    const onDown = (e: PointerEvent) => {
      dragging.current = true;
      lastPtr.current = { x: e.clientX, y: e.clientY };
      canvas.setPointerCapture(e.pointerId);
    };
    const onMove = (e: PointerEvent) => {
      if (!dragging.current) {
        return;
      }
      const rect = wrap.getBoundingClientRect();
      const cam = cameraRef.current;
      const aspect = rect.width / Math.max(rect.height, 1);
      const worldPerPxY = cam.zoom / Math.max(rect.height, 1);
      const worldPerPxX = (cam.zoom * aspect) / Math.max(rect.width, 1);
      const dx = e.clientX - lastPtr.current.x;
      const dy = e.clientY - lastPtr.current.y;
      cam.x -= dx * worldPerPxX;
      cam.y += dy * worldPerPxY;
      lastPtr.current = { x: e.clientX, y: e.clientY };
    };
    const onUp = (e: PointerEvent) => {
      dragging.current = false;
      canvas.releasePointerCapture(e.pointerId);
    };

    canvas.addEventListener("wheel", onWheel, { passive: false });
    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointermove", onMove);
    canvas.addEventListener("pointerup", onUp);

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      canvas.removeEventListener("wheel", onWheel);
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerup", onUp);
      renderer?.destroy();
    };
  }, [onFps, onGpuReady]);

  return (
    <div ref={wrapRef} className="relative h-full w-full overflow-hidden">
      <canvas ref={canvasRef} className="h-full w-full touch-none cursor-grab active:cursor-grabbing" />
      <p className="pointer-events-none absolute bottom-3 left-3 rounded bg-ink-900/80 px-3 py-1 font-mono text-xs text-ink-100">
        {hint}
      </p>
    </div>
  );
}
