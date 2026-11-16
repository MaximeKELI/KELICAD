type ShapeBtn = {
  cmd: string;
  label: string;
  hint: string;
};

const SHAPES: { title: string; items: ShapeBtn[] }[] = [
  {
    title: "Basiques",
    items: [
      { cmd: "L", label: "Ligne", hint: "2 points" },
      { cmd: "PL", label: "Polyligne", hint: "points + Entrée" },
      { cmd: "REC", label: "Rectangle", hint: "2 coins" },
      { cmd: "C", label: "Cercle", hint: "centre + rayon" },
      { cmd: "C2", label: "Ø 2 pts", hint: "diamètre" },
      { cmd: "A", label: "Arc", hint: "centre / début / fin" },
    ],
  },
  {
    title: "Figures",
    items: [
      { cmd: "POL", label: "Polygone", hint: "n → centre → sommet" },
      { cmd: "STAR", label: "Étoile", hint: "branches → centre → sommet" },
      { cmd: "DONUT", label: "Donut", hint: "centre → R → r" },
      { cmd: "CAPSULE", label: "Capsule", hint: "bout → bout → rayon" },
      { cmd: "ARROW", label: "Flèche", hint: "origine → pointe" },
      { cmd: "TRIANGLE", label: "Triangle", hint: "base → sommet" },
      { cmd: "EL", label: "Ellipse", hint: "centre + axes" },
      { cmd: "SPL", label: "Spline", hint: "points + Entrée" },
      { cmd: "GRID", label: "Grille", hint: "cols → rows → origine → pas" },
    ],
  },
  {
    title: "3D rapide",
    items: [
      { cmd: "__EXTRUDE__", label: "Extrude", hint: "profil fermé → solide" },
      { cmd: "__CYL__", label: "Cylindre", hint: "r=1 h=2.5" },
      { cmd: "__SPH__", label: "Sphère", hint: "r=1" },
      { cmd: "__TOR__", label: "Tore", hint: "R=2 r=0.4" },
      { cmd: "__PYR__", label: "Pyramide", hint: "base 4" },
      { cmd: "__PIPE__", label: "Tube", hint: "creux" },
      { cmd: "__REV__", label: "Révolution", hint: "profil sélectionné" },
    ],
  },
];

type CadLike = {
  start_command(name: string): string;
  solid_extrude_selected(height: number): string;
  solid_cylinder(cx: number, cy: number, radius: number, height: number): string;
  solid_sphere(cx: number, cy: number, cz: number, radius: number): string;
  solid_torus(cx: number, cy: number, cz: number, major: number, minor: number): string;
  solid_pyramid(cx: number, cy: number, radius: number, height: number, sides: number): string;
  solid_pipe(cx: number, cy: number, outer: number, inner: number, height: number): string;
  solid_revolve_selected(degrees: number): string;
  view_derive_apply(kind: string): number;
  solid_count(): number;
};

type Props = {
  cad: CadLike;
  onCommand: (raw: string) => void;
  onEcho: (lines: string[]) => void;
  onChanged: () => void;
};

export function ShapePalette({ cad, onCommand, onEcho, onChanged }: Props) {
  const run = (cmd: string) => {
    try {
      if (cmd === "__EXTRUDE__") {
        const summary = cad.solid_extrude_selected(2.5);
        const n = cad.view_derive_apply("plan");
        onEcho([`EXTRUDE: ${summary}`, `Vue plan: ${n}`]);
        onChanged();
        return;
      }
      if (cmd === "__CYL__") {
        onEcho([cad.solid_cylinder(0, 0, 1, 2.5)]);
        onChanged();
        return;
      }
      if (cmd === "__SPH__") {
        onEcho([cad.solid_sphere(0, 0, 0, 1)]);
        onChanged();
        return;
      }
      if (cmd === "__TOR__") {
        onEcho([cad.solid_torus(0, 0, 0, 2, 0.4)]);
        onChanged();
        return;
      }
      if (cmd === "__PYR__") {
        onEcho([cad.solid_pyramid(0, 0, 1.2, 2, 4)]);
        onChanged();
        return;
      }
      if (cmd === "__PIPE__") {
        onEcho([cad.solid_pipe(0, 0, 1, 0.6, 3)]);
        onChanged();
        return;
      }
      if (cmd === "__REV__") {
        onEcho([cad.solid_revolve_selected(360)]);
        onChanged();
        return;
      }
      onCommand(cad.start_command(cmd));
    } catch (e) {
      onEcho([`Erreur: ${String(e)}`]);
    }
  };

  return (
    <aside className="flex h-full w-52 shrink-0 flex-col border-r border-ink-700/60 bg-ink-900/90">
      <div className="border-b border-ink-700/60 px-3 py-2">
        <p className="text-xs uppercase tracking-[0.2em] text-accent-soft">Figures</p>
        <p className="text-sm text-ink-100">Dessin rapide</p>
      </div>
      <div className="min-h-0 flex-1 space-y-3 overflow-auto p-2">
        {SHAPES.map((group) => (
          <div key={group.title}>
            <p className="mb-1 px-1 text-[10px] uppercase tracking-wider text-ink-500">
              {group.title}
            </p>
            <div className="grid grid-cols-2 gap-1">
              {group.items.map((item) => (
                <button
                  key={item.cmd}
                  type="button"
                  title={item.hint}
                  className="rounded border border-ink-700/80 bg-ink-950/60 px-1.5 py-1.5 text-left hover:border-accent-soft/60 hover:bg-ink-800/40"
                  onClick={() => run(item.cmd)}
                >
                  <span className="block text-xs text-ink-100">{item.label}</span>
                  <span className="block truncate font-mono text-[9px] text-ink-500">
                    {item.hint}
                  </span>
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>
    </aside>
  );
}
