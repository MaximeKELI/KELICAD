import type { CadSession } from "./types";

type SelItem = { id: string; kind?: string };

function selection(cad: CadSession): SelItem[] {
  try {
    return JSON.parse(cad.selection_detail_json()) as SelItem[];
  } catch {
    try {
      const ids = JSON.parse(cad.selection_json()) as string[];
      return ids.map((id) => ({ id }));
    } catch {
      return [];
    }
  }
}

function need(n: number, sel: SelItem[], label: string): string | null {
  if (sel.length < n) {
    return `${label} : sélectionnez ${n} entité(s) (ligne).`;
  }
  return null;
}

/** Apply a named constraint from the current selection. Returns echo lines. */
export function applyConstraint(
  cad: CadSession,
  action: string,
): string[] {
  if (action === "solve") {
    const raw = cad.constraint_solve();
    return [`Contraintes résolues: ${raw}`];
  }

  const sel = selection(cad);

  switch (action) {
    case "horizontal": {
      const err = need(1, sel, "Horizontal");
      if (err) return [err];
      const id = cad.constraint_add_json(
        JSON.stringify({ kind: "horizontal", entity: sel[0].id }),
      );
      return [`Contrainte horizontale #${id}`];
    }
    case "vertical": {
      const err = need(1, sel, "Vertical");
      if (err) return [err];
      const id = cad.constraint_add_json(
        JSON.stringify({ kind: "vertical", entity: sel[0].id }),
      );
      return [`Contrainte verticale #${id}`];
    }
    case "parallel": {
      const err = need(2, sel, "Parallèle");
      if (err) return [err];
      const id = cad.constraint_add_json(
        JSON.stringify({
          kind: "parallel",
          a: sel[0].id,
          b: sel[1].id,
        }),
      );
      return [`Contrainte parallèle #${id}`];
    }
    case "perpendicular": {
      const err = need(2, sel, "Perpendiculaire");
      if (err) return [err];
      const id = cad.constraint_add_json(
        JSON.stringify({
          kind: "perpendicular",
          a: sel[0].id,
          b: sel[1].id,
        }),
      );
      return [`Contrainte perpendiculaire #${id}`];
    }
    case "equal": {
      const err = need(2, sel, "Égal");
      if (err) return [err];
      const id = cad.constraint_add_json(
        JSON.stringify({
          kind: "equal_length",
          a: sel[0].id,
          b: sel[1].id,
        }),
      );
      return [`Contrainte égalité #${id}`];
    }
    case "coincident": {
      const err = need(2, sel, "Coïncident");
      if (err) return [err];
      const id = cad.constraint_add_json(
        JSON.stringify({
          kind: "coincident",
          a: sel[0].id,
          a_end: "start",
          b: sel[1].id,
          b_end: "start",
        }),
      );
      return [`Contrainte coïncidence #${id}`];
    }
    case "distance": {
      const err = need(1, sel, "Distance");
      if (err) return [err];
      const raw = window.prompt("Longueur cible :", "10");
      if (raw == null) return ["Distance annulée."];
      const value = Number(raw);
      if (!Number.isFinite(value) || value <= 0) {
        return ["Distance invalide."];
      }
      const id = cad.constraint_add_json(
        JSON.stringify({
          kind: "distance",
          entity: sel[0].id,
          value,
        }),
      );
      return [`Contrainte distance #${id} = ${value}`];
    }
    case "angle": {
      const err = need(1, sel, "Angle");
      if (err) return [err];
      const raw = window.prompt("Angle (degrés vs +X) :", "0");
      if (raw == null) return ["Angle annulé."];
      const degrees = Number(raw);
      if (!Number.isFinite(degrees)) return ["Angle invalide."];
      const id = cad.constraint_add_json(
        JSON.stringify({
          kind: "angle",
          entity: sel[0].id,
          degrees,
        }),
      );
      return [`Contrainte angle #${id} = ${degrees}°`];
    }
    default:
      return [`Contrainte inconnue: ${action}`];
  }
}
