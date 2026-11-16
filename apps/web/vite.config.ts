import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  // Clear screen off so Tauri `dev` logs stay visible.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: "127.0.0.1",
  },
  envPrefix: ["VITE_", "TAURI_"],
  assetsInclude: ["**/*.wasm"],
  optimizeDeps: {
    exclude: ["kelicad_wasm"],
  },
  build: {
    target: process.env.TAURI_ENV_PLATFORM ? "es2021" : "modules",
    minify: process.env.TAURI_ENV_DEBUG ? false : "esbuild",
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
