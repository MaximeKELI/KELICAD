type Tool = {
  cmd: string;
  label: string;
  icon: string;
  title?: string;
};

const GROUPS: { title: string; tools: Tool[] }[] = [
  {
    title: "Dessin",
    tools: [
      { cmd: "L", label: "L", icon: "／", title: "Ligne" },
      { cmd: "PL", label: "PL", icon: "⌇", title: "Polyligne" },
      { cmd: "REC", label: "□", icon: "▭", title: "Rectangle" },
      { cmd: "C", label: "○", icon: "○", title: "Cercle" },
      { cmd: "A", label: "⌒", icon: "⌒", title: "Arc" },
      { cmd: "EL", label: "⬭", icon: "⬭", title: "Ellipse" },
      { cmd: "SPL", label: "~", icon: "∿", title: "Spline" },
      { cmd: "POL", label: "⬡", icon: "⬡", title: "Polygone" },
      { cmd: "PO", label: "·", icon: "·", title: "Point" },
      { cmd: "H", label: "▤", icon: "▤", title: "Hachure" },
      { cmd: "XL", label: "XL", icon: "✕", title: "Construction (XLINE)" },
      { cmd: "RAY", label: "RAY", icon: "↗", title: "Rayon" },
      { cmd: "DONUT", label: "◎", icon: "◎", title: "Anneau" },
      { cmd: "REVCLOUD", label: "☁", icon: "☁", title: "Nuage" },
    ],
  },
  {
    title: "Modifier",
    tools: [
      { cmd: "M", label: "M", icon: "⇔", title: "Déplacer" },
      { cmd: "CO", label: "CO", icon: "⧉", title: "Copier" },
      { cmd: "ST", label: "ST", icon: "⇄", title: "Étirer" },
      { cmd: "RO", label: "RO", icon: "↻", title: "Rotation" },
      { cmd: "SC", label: "SC", icon: "⤢", title: "Échelle" },
      { cmd: "MI", label: "MI", icon: "||", title: "Miroir" },
      { cmd: "O", label: "O", icon: "⇉", title: "Décalage" },
      { cmd: "TR", label: "TR", icon: "✂", title: "Ajuster" },
      { cmd: "EX", label: "EX", icon: "↦", title: "Prolonger" },
      { cmd: "LEN", label: "LEN", icon: "⟷", title: "Allonger" },
      { cmd: "BR", label: "BR", icon: "⊥", title: "Coupure" },
      { cmd: "J", label: "J", icon: "⋈", title: "Joindre" },
      { cmd: "F", label: "F", icon: "⌒", title: "Congé" },
      { cmd: "CHA", label: "CH", icon: "⌐", title: "Chanfrein" },
      { cmd: "AR", label: "AR", icon: "▦", title: "Réseau" },
      { cmd: "X", label: "X", icon: "✸", title: "Exploser" },
      { cmd: "E", label: "E", icon: "✕", title: "Effacer" },
    ],
  },
  {
    title: "Cotes",
    tools: [
      { cmd: "DLI", label: "⌀", icon: "↔", title: "Cote linéaire" },
      { cmd: "DAL", label: "∠", icon: "⤢", title: "Cote alignée" },
      { cmd: "DRA", label: "R", icon: "R", title: "Rayon" },
      { cmd: "DDI", label: "⌀", icon: "⌀", title: "Diamètre" },
      { cmd: "DAN", label: "∠", icon: "∠", title: "Angle" },
      { cmd: "DCO", label: "↕", icon: "↕", title: "Cote continue" },
      { cmd: "LEAD", label: "→", icon: "→", title: "Repère" },
      { cmd: "DT", label: "A", icon: "A", title: "Texte" },
      { cmd: "TBL", label: "▦", icon: "▦", title: "Tableau" },
    ],
  },
  {
    title: "Blocs",
    tools: [
      { cmd: "B", label: "B", icon: "▣", title: "Créer bloc" },
      { cmd: "I", label: "I", icon: "⬇", title: "Insérer" },
      { cmd: "W", label: "W", icon: "▭", title: "WBLOCK" },
      { cmd: "X", label: "X", icon: "✸", title: "Exploser" },
    ],
  },
];

type Props = {
  activeCmd?: string | null;
  onCommand: (cmd: string) => void;
};

export function ToolRail({ activeCmd, onCommand }: Props) {
  return (
    <aside className="cad-rail flex w-11 shrink-0 flex-col overflow-y-auto border-r border-[#2a2a2a] bg-[#1b1b1b]">
      {GROUPS.map((g) => (
        <div key={g.title} className="border-b border-[#2a2a2a] py-1">
          <p className="px-0.5 pb-0.5 text-center text-[8px] uppercase tracking-wide text-[#6a6a6a]">
            {g.title.slice(0, 3)}
          </p>
          {g.tools.map((t) => (
            <button
              key={`${g.title}-${t.cmd}-${t.title}`}
              type="button"
              title={t.title ?? t.label}
              className={`cad-tool ${activeCmd === t.cmd ? "is-active" : ""}`}
              onClick={() => onCommand(t.cmd)}
            >
              <span className="text-[13px] leading-none">{t.icon}</span>
            </button>
          ))}
        </div>
      ))}
    </aside>
  );
}
