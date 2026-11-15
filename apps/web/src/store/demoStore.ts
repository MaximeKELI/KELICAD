import { create } from "zustand";

type DemoState = {
  fps: number;
  gpuStatus: string;
  setFps: (fps: number) => void;
  setGpuStatus: (status: string) => void;
};

export const useDemoStore = create<DemoState>((set) => ({
  fps: 0,
  gpuStatus: "…",
  setFps: (fps) => set({ fps }),
  setGpuStatus: (gpuStatus) => set({ gpuStatus }),
}));
