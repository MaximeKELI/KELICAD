import { useCallback } from "react";

import { CanvasViewport } from "./components/CanvasViewport";
import { useDemoStore } from "./store/demoStore";

export function App() {
  const fps = useDemoStore((s) => s.fps);
  const gpuStatus = useDemoStore((s) => s.gpuStatus);
  const setFps = useDemoStore((s) => s.setFps);
  const setGpuStatus = useDemoStore((s) => s.setGpuStatus);

  const onFps = useCallback(
    (value: number) => {
      setFps(value);
    },
    [setFps],
  );

  const onGpuReady = useCallback(
    (_ok: boolean, message: string) => {
      setGpuStatus(message);
    },
    [setGpuStatus],
  );

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between border-b border-ink-700/60 bg-ink-900 px-4 py-3">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-accent-soft">KELICAD</p>
          <h1 className="text-lg font-medium text-ink-100">Milestone 0 — canevas WebGPU</h1>
        </div>
        <div className="flex gap-6 font-mono text-sm text-ink-100">
          <div>
            <span className="text-ink-500">FPS </span>
            <span className="text-accent-soft">{fps || "—"}</span>
          </div>
          <div>
            <span className="text-ink-500">GPU </span>
            <span>{gpuStatus}</span>
          </div>
          <div>
            <span className="text-ink-500">Lignes </span>
            <span>100,000</span>
          </div>
        </div>
      </header>
      <main className="min-h-0 flex-1">
        <CanvasViewport onFps={onFps} onGpuReady={onGpuReady} />
      </main>
    </div>
  );
}
