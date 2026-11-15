import init, { WasmCad } from "../wasm/pkg/kelicad_wasm.js";

import type { CadSession } from "./types";

let ready: Promise<CadSession> | null = null;

/** Initialize WASM core and return a CAD session. */
export function loadCad(): Promise<CadSession> {
  if (!ready) {
    ready = (async () => {
      await init();
      return new WasmCad() as unknown as CadSession;
    })();
  }
  return ready;
}
