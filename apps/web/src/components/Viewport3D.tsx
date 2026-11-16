import { useEffect, useRef, useState } from "react";

import type { CadSession } from "../cad/types";
import {
  type Camera3D,
  fitCameraToAabb,
  MeshRenderer,
  type MeshBatch,
} from "../gpu/meshRenderer";

type SolidMeshDto = {
  id: string;
  positions: number[];
  indices: number[];
  aabb?: { min: number[]; max: number[] };
};

type Props = {
  cad: CadSession;
  revision: number;
  onEcho?: (lines: string[]) => void;
};

const PRESETS: { id: string; label: string; apply: (c: Camera3D) => Camera3D }[] =
  [
    {
      id: "iso",
      label: "ISO",
      apply: (c) => ({
        ...c,
        yaw: Math.PI * 0.25,
        pitch: Math.PI * 0.28,
      }),
    },
    {
      id: "top",
      label: "TOP",
      apply: (c) => ({ ...c, yaw: -Math.PI / 2, pitch: Math.PI / 2 - 0.05 }),
    },
    {
      id: "front",
      label: "FRONT",
      apply: (c) => ({ ...c, yaw: -Math.PI / 2, pitch: 0 }),
    },
    {
      id: "side",
      label: "SIDE",
      apply: (c) => ({ ...c, yaw: 0, pitch: 0 }),
    },
  ];

const PALETTE: [number, number, number][] = [
  [0.35, 0.72, 0.95],
  [0.95, 0.55, 0.3],
  [0.45, 0.85, 0.55],
  [0.9, 0.75, 0.35],
  [0.75, 0.5, 0.95],
];

export function Viewport3D({ cad, revision, onEcho }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const rendererRef = useRef<MeshRenderer | null>(null);
  const camRef = useRef<Camera3D>({
    target: [0, 0, 0],
    yaw: Math.PI * 0.25,
    pitch: Math.PI * 0.28,
    distance: 12,
  });
  const dragRef = useRef<{
    mode: "orbit" | "pan" | null;
    x: number;
    y: number;
  }>({ mode: null, x: 0, y: 0 });
  const [ready, setReady] = useState(false);
  const [wire, setWire] = useState(false);
  const [solidCount, setSolidCount] = useState(0);
  const [camTick, setCamTick] = useState(0);
  void camTick;

  useEffect(() => {
    const canvas = canvasRef.current;
    const wrap = wrapRef.current;
    if (!canvas || !wrap) return;
    let disposed = false;
    let raf = 0;

    void (async () => {
      const renderer = await MeshRenderer.create(canvas);
      if (disposed) {
        renderer?.destroy();
        return;
      }
      if (!renderer) {
        onEcho?.(["WebGPU indisponible pour le viewport 3D."]);
        return;
      }
      rendererRef.current = renderer;
      setReady(true);

      const tick = () => {
        if (disposed || !rendererRef.current) return;
        const rect = wrap.getBoundingClientRect();
        rendererRef.current.resize(rect.width, rect.height);
        rendererRef.current.draw(camRef.current);
        raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    })();

    const onDown = (e: PointerEvent) => {
      if (e.button === 0 || e.button === 1) {
        dragRef.current = {
          mode: e.button === 1 || e.shiftKey ? "pan" : "orbit",
          x: e.clientX,
          y: e.clientY,
        };
        canvas.setPointerCapture(e.pointerId);
      }
    };
    const onMove = (e: PointerEvent) => {
      const d = dragRef.current;
      if (!d.mode) return;
      const dx = e.clientX - d.x;
      const dy = e.clientY - d.y;
      d.x = e.clientX;
      d.y = e.clientY;
      const cam = camRef.current;
      if (d.mode === "orbit") {
        cam.yaw -= dx * 0.008;
        cam.pitch = Math.max(
          -Math.PI / 2 + 0.05,
          Math.min(Math.PI / 2 - 0.05, cam.pitch + dy * 0.008),
        );
      } else {
        const scale = cam.distance * 0.002;
        const cy = Math.cos(cam.yaw);
        const sy = Math.sin(cam.yaw);
        // Pan in XY plane
        cam.target[0] += (-dx * cy - dy * sy) * scale;
        cam.target[1] += (-dx * sy + dy * cy) * scale;
      }
      setCamTick((n) => n + 1);
    };
    const onUp = (e: PointerEvent) => {
      dragRef.current.mode = null;
      try {
        canvas.releasePointerCapture(e.pointerId);
      } catch {
        /* ignore */
      }
    };
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const cam = camRef.current;
      cam.distance = Math.min(
        Math.max(cam.distance * (e.deltaY > 0 ? 1.12 : 1 / 1.12), 0.5),
        5000,
      );
      setCamTick((n) => n + 1);
    };

    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointermove", onMove);
    canvas.addEventListener("pointerup", onUp);
    canvas.addEventListener("wheel", onWheel, { passive: false });

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerup", onUp);
      canvas.removeEventListener("wheel", onWheel);
      rendererRef.current?.destroy();
      rendererRef.current = null;
    };
  }, [cad, onEcho]);

  useEffect(() => {
    const renderer = rendererRef.current;
    if (!renderer) return;
    try {
      const raw = JSON.parse(cad.solids_meshes_json()) as SolidMeshDto[];
      setSolidCount(raw.length);
      const batches: MeshBatch[] = raw.map((m, i) => {
        const c = PALETTE[i % PALETTE.length];
        return {
          positions: new Float32Array(m.positions),
          indices: new Uint32Array(m.indices),
          r: c[0],
          g: c[1],
          b: c[2],
        };
      });
      renderer.setMeshes(batches);
      renderer.setWireframe(wire);

      if (raw.length) {
        let min: [number, number, number] = [Infinity, Infinity, Infinity];
        let max: [number, number, number] = [
          -Infinity,
          -Infinity,
          -Infinity,
        ];
        for (const m of raw) {
          if (m.aabb) {
            min = [
              Math.min(min[0], m.aabb.min[0]),
              Math.min(min[1], m.aabb.min[1]),
              Math.min(min[2], m.aabb.min[2]),
            ];
            max = [
              Math.max(max[0], m.aabb.max[0]),
              Math.max(max[1], m.aabb.max[1]),
              Math.max(max[2], m.aabb.max[2]),
            ];
          }
        }
        if (Number.isFinite(min[0])) {
          const fitted = fitCameraToAabb(min, max);
          // Keep yaw/pitch if user already framed; only reset distance/target on first solid.
          if (solidCount === 0) {
            camRef.current = fitted;
          } else {
            camRef.current.target = fitted.target;
            camRef.current.distance = Math.max(
              camRef.current.distance,
              fitted.distance * 0.6,
            );
          }
        }
      }
    } catch {
      renderer.setMeshes([]);
      setSolidCount(0);
    }
  }, [cad, revision, wire, ready]);

  return (
    <div ref={wrapRef} className="relative h-full w-full overflow-hidden bg-ink-950">
      <canvas
        ref={canvasRef}
        className="h-full w-full touch-none cursor-grab active:cursor-grabbing"
      />
      <div className="absolute left-2 top-2 flex flex-wrap gap-1">
        {PRESETS.map((p) => (
          <button
            key={p.id}
            type="button"
            className="rounded border border-ink-700 bg-ink-900/80 px-1.5 py-0.5 font-mono text-[10px] hover:border-accent-soft"
            onClick={() => {
              camRef.current = p.apply(camRef.current);
              setCamTick((n) => n + 1);
            }}
          >
            {p.label}
          </button>
        ))}
        <button
          type="button"
          className="rounded border border-ink-700 bg-ink-900/80 px-1.5 py-0.5 font-mono text-[10px] hover:border-accent-soft"
          onClick={() => setWire((w) => !w)}
        >
          {wire ? "SHADE" : "WIRE"}
        </button>
        <button
          type="button"
          className="rounded border border-ink-700 bg-ink-900/80 px-1.5 py-0.5 font-mono text-[10px] hover:border-accent-soft"
          onClick={() => {
            try {
              const raw = JSON.parse(cad.solids_meshes_json()) as SolidMeshDto[];
              if (!raw.length) return;
              let min: [number, number, number] = [Infinity, Infinity, Infinity];
              let max: [number, number, number] = [
                -Infinity,
                -Infinity,
                -Infinity,
              ];
              for (const m of raw) {
                if (!m.aabb) continue;
                min = [
                  Math.min(min[0], m.aabb.min[0]),
                  Math.min(min[1], m.aabb.min[1]),
                  Math.min(min[2], m.aabb.min[2]),
                ];
                max = [
                  Math.max(max[0], m.aabb.max[0]),
                  Math.max(max[1], m.aabb.max[1]),
                  Math.max(max[2], m.aabb.max[2]),
                ];
              }
              camRef.current = fitCameraToAabb(min, max);
              setCamTick((n) => n + 1);
            } catch {
              /* ignore */
            }
          }}
        >
          FIT
        </button>
      </div>
      <p className="pointer-events-none absolute bottom-2 left-2 rounded bg-ink-900/80 px-2 py-1 font-mono text-[10px] text-ink-300">
        3D · {solidCount} solid(s) · orbit clic · Shift+pan · molette zoom
        {!ready ? " · init…" : ""}
      </p>
    </div>
  );
}
