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

/** Convert 2D line segments (x,y pairs) into thin XY-plane quads for the mesh renderer. */
function linesToQuads(
  verts: Float32Array | number[],
  halfW: number,
): MeshBatch | null {
  const nSeg = Math.floor(verts.length / 4);
  if (nSeg < 1) return null;
  const positions: number[] = [];
  const indices: number[] = [];
  let v = 0;
  for (let i = 0; i < nSeg; i++) {
    const x0 = verts[i * 4]!;
    const y0 = verts[i * 4 + 1]!;
    const x1 = verts[i * 4 + 2]!;
    const y1 = verts[i * 4 + 3]!;
    const dx = x1 - x0;
    const dy = y1 - y0;
    const len = Math.hypot(dx, dy);
    if (len < 1e-12) continue;
    const nx = (-dy / len) * halfW;
    const ny = (dx / len) * halfW;
    // 4 corners of a flat quad at z=0
    positions.push(x0 + nx, y0 + ny, 0);
    positions.push(x0 - nx, y0 - ny, 0);
    positions.push(x1 - nx, y1 - ny, 0);
    positions.push(x1 + nx, y1 + ny, 0);
    indices.push(v, v + 1, v + 2, v, v + 2, v + 3);
    v += 4;
  }
  if (!positions.length) return null;
  return {
    positions: new Float32Array(positions),
    indices: new Uint32Array(indices),
    r: 0.75,
    g: 0.85,
    b: 1.0,
  };
}

function bounds2d(verts: Float32Array | number[]): {
  min: [number, number, number];
  max: [number, number, number];
} | null {
  if (verts.length < 2) return null;
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
  if (!Number.isFinite(minX)) return null;
  return {
    min: [minX, minY, -0.5],
    max: [maxX, maxY, 0.5],
  };
}

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
  const fittedOnceRef = useRef(false);
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
        Math.max(cam.distance * (e.deltaY > 0 ? 1.12 : 1 / 1.12), 0.2),
        1e9,
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
        const c = PALETTE[i % PALETTE.length]!;
        return {
          positions: new Float32Array(m.positions),
          indices: new Uint32Array(m.indices),
          r: c[0],
          g: c[1],
          b: c[2],
        };
      });

      // Toujours afficher le dessin 2D sur le plan XY (sinon 3D paraît « cassée »).
      let lineVerts: Float32Array | number[] = [];
      try {
        lineVerts = cad.scene_line_vertices();
      } catch {
        lineVerts = [];
      }
      const b2 = bounds2d(lineVerts);
      const span = b2
        ? Math.max(b2.max[0] - b2.min[0], b2.max[1] - b2.min[1], 1)
        : 10;
      const lineMesh = linesToQuads(lineVerts, Math.max(span * 0.003, 0.02));
      if (lineMesh) batches.push(lineMesh);

      renderer.setMeshes(batches);
      renderer.setWireframe(wire);

      let min: [number, number, number] = [Infinity, Infinity, Infinity];
      let max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
      let hasBounds = false;
      for (const m of raw) {
        if (!m.aabb) continue;
        hasBounds = true;
        min = [
          Math.min(min[0], m.aabb.min[0]!),
          Math.min(min[1], m.aabb.min[1]!),
          Math.min(min[2], m.aabb.min[2]!),
        ];
        max = [
          Math.max(max[0], m.aabb.max[0]!),
          Math.max(max[1], m.aabb.max[1]!),
          Math.max(max[2], m.aabb.max[2]!),
        ];
      }
      if (b2) {
        hasBounds = true;
        min = [
          Math.min(min[0], b2.min[0]),
          Math.min(min[1], b2.min[1]),
          Math.min(min[2], b2.min[2]),
        ];
        max = [
          Math.max(max[0], b2.max[0]),
          Math.max(max[1], b2.max[1]),
          Math.max(max[2], b2.max[2]),
        ];
      }
      if (hasBounds && Number.isFinite(min[0])) {
        const fitted = fitCameraToAabb(min, max);
        if (!fittedOnceRef.current) {
          camRef.current = fitted;
          fittedOnceRef.current = true;
        } else if (raw.length > solidCount) {
          // Nouveau solide ajouté → recentrer légèrement.
          camRef.current.target = fitted.target;
          camRef.current.distance = Math.max(
            camRef.current.distance,
            fitted.distance * 0.6,
          );
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
        3D · {solidCount} solide(s) · dessin 2D sur XY · orbit · Shift+pan · molette
        {!ready ? " · init WebGPU…" : ""}
        {ready && solidCount === 0
          ? " · tip: onglet 3D → Boîte pour un volume"
          : ""}
      </p>
    </div>
  );
}
