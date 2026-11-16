import { useEffect, useMemo, useState } from "react";

import type { CadSession } from "../cad/types";

type ProjectProps = {
  units: string;
  title: string;
  scale: string;
};

type SelItem = {
  id: string;
  kind: string;
  layer?: string;
  layer_id?: string;
  bbox?: { min: [number, number]; max: [number, number] };
  geometry?: { type?: string; [k: string]: unknown };
};

type Props = {
  cad: CadSession;
  revision: number;
  onChanged: () => void;
  onEcho: (lines: string[]) => void;
};

const UNITS = ["mm", "m", "in", "unitless"] as const;

export function PropertiesPanel({ cad, revision, onChanged, onEcho }: Props) {
  const project = useMemo(() => {
    void revision;
    try {
      return JSON.parse(cad.project_props_json()) as ProjectProps;
    } catch {
      return { units: "mm", title: "KELICAD", scale: "1:100" };
    }
  }, [cad, revision]);

  const selection = useMemo(() => {
    void revision;
    try {
      return JSON.parse(cad.selection_detail_json()) as SelItem[];
    } catch {
      return [];
    }
  }, [cad, revision]);

  const [title, setTitle] = useState(project.title);
  const [scale, setScale] = useState(project.scale);

  useEffect(() => {
    setTitle(project.title);
    setScale(project.scale);
  }, [project.title, project.scale]);

  const first = selection[0];
  const bbox = first?.bbox;

  return (
    <div className="flex h-full w-52 shrink-0 flex-col border-l border-ink-700/60 bg-ink-900/90">
      <div className="border-b border-ink-700/60 p-2">
        <p className="mb-1 text-[10px] uppercase tracking-wider text-ink-500">
          Projet
        </p>
        <label className="mb-1 block font-mono text-[10px] text-ink-500">
          Titre
          <input
            className="mt-0.5 w-full rounded border border-ink-700 bg-ink-950 px-1.5 py-0.5 text-ink-100"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={() => {
              cad.set_project_title(title);
              onChanged();
            }}
          />
        </label>
        <label className="mb-1 block font-mono text-[10px] text-ink-500">
          Échelle
          <input
            className="mt-0.5 w-full rounded border border-ink-700 bg-ink-950 px-1.5 py-0.5 text-ink-100"
            value={scale}
            onChange={(e) => setScale(e.target.value)}
            onBlur={() => {
              cad.set_project_scale(scale);
              onChanged();
            }}
          />
        </label>
        <label className="block font-mono text-[10px] text-ink-500">
          Unités
          <select
            className="mt-0.5 w-full rounded border border-ink-700 bg-ink-950 px-1.5 py-0.5 text-ink-100"
            value={project.units}
            onChange={(e) => {
              try {
                cad.set_project_units(e.target.value);
                onChanged();
                onEcho([`Unités : ${e.target.value}`]);
              } catch (err) {
                onEcho([`Unités: ${String(err)}`]);
              }
            }}
          >
            {UNITS.map((u) => (
              <option key={u} value={u}>
                {u}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-2">
        <p className="mb-1 text-[10px] uppercase tracking-wider text-ink-500">
          Propriétés
        </p>
        {!first ? (
          <p className="font-mono text-[10px] text-ink-600">
            Sélectionnez un objet
          </p>
        ) : (
          <dl className="space-y-1 font-mono text-[10px] text-ink-300">
            <div className="flex justify-between gap-2">
              <dt className="text-ink-500">Type</dt>
              <dd className="text-accent-soft">{first.kind}</dd>
            </div>
            <div className="flex justify-between gap-2">
              <dt className="text-ink-500">Calque</dt>
              <dd>{first.layer ?? "—"}</dd>
            </div>
            {bbox && (
              <>
                <div className="flex justify-between gap-2">
                  <dt className="text-ink-500">Min</dt>
                  <dd>
                    {bbox.min[0].toFixed(2)}, {bbox.min[1].toFixed(2)}
                  </dd>
                </div>
                <div className="flex justify-between gap-2">
                  <dt className="text-ink-500">Max</dt>
                  <dd>
                    {bbox.max[0].toFixed(2)}, {bbox.max[1].toFixed(2)}
                  </dd>
                </div>
              </>
            )}
            {first.geometry?.type && (
              <div className="flex justify-between gap-2">
                <dt className="text-ink-500">Geom</dt>
                <dd>{String(first.geometry.type)}</dd>
              </div>
            )}
            {selection.length > 1 && (
              <p className="pt-1 text-ink-600">
                +{selection.length - 1} autre(s)
              </p>
            )}
          </dl>
        )}
      </div>

      <div className="border-t border-ink-700/60 p-2">
        <p className="mb-1 text-[10px] uppercase tracking-wider text-ink-500">
          Gabarit
        </p>
        <div className="flex flex-wrap gap-1">
          {["a4", "a3", "a2", "a1", "mech", "arch"].map((t) => (
            <button
              key={t}
              type="button"
              className="rounded border border-ink-700 px-1.5 py-0.5 font-mono text-[10px] uppercase hover:border-accent-soft"
              onClick={() => {
                try {
                  const n = cad.apply_template(t);
                  onChanged();
                  onEcho([`Gabarit ${t.toUpperCase()} : +${n}`]);
                } catch (e) {
                  onEcho([`Gabarit: ${String(e)}`]);
                }
              }}
            >
              {t}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
