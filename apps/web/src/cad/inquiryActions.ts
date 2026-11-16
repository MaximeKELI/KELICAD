import type { CadSession } from "./types";

type SelItem = {
  id: string;
  kind?: string;
  layer?: string;
  bbox?: { min: [number, number]; max: [number, number] };
};

function selection(cad: CadSession): SelItem[] {
  try {
    return JSON.parse(cad.selection_detail_json()) as SelItem[];
  } catch {
    return [];
  }
}

function unionBBox(items: SelItem[]): {
  min: [number, number];
  max: [number, number];
} | null {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  let any = false;
  for (const it of items) {
    if (!it.bbox) continue;
    any = true;
    minX = Math.min(minX, it.bbox.min[0]);
    minY = Math.min(minY, it.bbox.min[1]);
    maxX = Math.max(maxX, it.bbox.max[0]);
    maxY = Math.max(maxY, it.bbox.max[1]);
  }
  if (!any) return null;
  return { min: [minX, minY], max: [maxX, maxY] };
}

/** Inquiry tools (ACA « Renseignements ») — functional from selection. */
export function applyInquiry(cad: CadSession, action: string): string[] {
  const sel = selection(cad);

  switch (action) {
    case "list": {
      if (sel.length === 0) return ["LISTE : aucune sélection."];
      const lines = sel.slice(0, 12).map((it, i) => {
        const bb = it.bbox
          ? ` [${it.bbox.min[0].toFixed(2)},${it.bbox.min[1].toFixed(2)}]→[${it.bbox.max[0].toFixed(2)},${it.bbox.max[1].toFixed(2)}]`
          : "";
        return `${i + 1}. ${it.kind ?? "?"} ${it.id.slice(0, 8)}… calque=${it.layer ?? "?"}${bb}`;
      });
      return [`LISTE (${sel.length}) :`, ...lines];
    }
    case "dist": {
      const bb = unionBBox(sel);
      if (!bb) {
        return [
          "DIST : sélectionnez des objets (largeur/hauteur/diagonale de l'AABB).",
        ];
      }
      const w = bb.max[0] - bb.min[0];
      const h = bb.max[1] - bb.min[1];
      const d = Math.hypot(w, h);
      return [
        `DIST AABB : ΔX=${w.toFixed(4)}  ΔY=${h.toFixed(4)}  diag=${d.toFixed(4)}`,
      ];
    }
    case "area": {
      const bb = unionBBox(sel);
      if (!bb) return ["AIRE : sélectionnez des objets."];
      const w = bb.max[0] - bb.min[0];
      const h = bb.max[1] - bb.min[1];
      return [
        `AIRE AABB : ${ (w * h).toFixed(4) }  (périmètre ${ (2 * (w + h)).toFixed(4) })`,
      ];
    }
    case "id": {
      if (sel.length === 0) return ["ID : aucune sélection."];
      return sel.slice(0, 8).map((it) => `ID ${it.kind ?? "?"} = ${it.id}`);
    }
    default:
      return [`Renseignement inconnu: ${action}`];
  }
}

export type ViewAction = "zoom-in" | "zoom-out" | "extents" | "zoom-window";

export function dispatchViewAction(action: ViewAction) {
  window.dispatchEvent(
    new CustomEvent("kelicad-view", { detail: { action } }),
  );
}
