/**
 * Single source of truth for ribbon / tool-rail tools.
 * Status mirrors the engine audit (kelicad-cmd CommandId + WASM side-channels).
 *
 * - working: end-to-end handler exists
 * - partial: API exists but UX is limited (still callable)
 * - unavailable: shown disabled — not yet wired (no fake “working” buttons)
 */

export type ToolStatus = "working" | "partial" | "unavailable";

export type ToolKind =
  | { type: "cad"; cmd: string }
  | { type: "ui"; action: string }
  | { type: "solid"; action: string }
  | { type: "constraint"; action: string }
  | { type: "layer"; action: string };

export type CatalogTool = {
  id: string;
  label: string;
  glyph: string;
  tip: string;
  status: ToolStatus;
  kind: ToolKind;
  /** Tokens that light up the tool rail when active (aliases). */
  aliases?: string[];
};

function cad(
  id: string,
  cmd: string,
  label: string,
  glyph: string,
  tip: string,
  aliases: string[] = [],
  status: ToolStatus = "working",
): CatalogTool {
  return {
    id,
    label,
    glyph,
    tip: `${tip} (${cmd})`,
    status,
    kind: { type: "cad", cmd },
    aliases: [cmd, ...aliases],
  };
}

function ui(
  id: string,
  action: string,
  label: string,
  glyph: string,
  tip: string,
  status: ToolStatus = "working",
): CatalogTool {
  return { id, label, glyph, tip, status, kind: { type: "ui", action } };
}

function solid(
  id: string,
  action: string,
  label: string,
  glyph: string,
  tip: string,
  status: ToolStatus = "working",
): CatalogTool {
  return { id, label, glyph, tip, status, kind: { type: "solid", action } };
}

function constraint(
  id: string,
  action: string,
  label: string,
  glyph: string,
  tip: string,
  status: ToolStatus = "working",
): CatalogTool {
  return {
    id,
    label,
    glyph,
    tip,
    status,
    kind: { type: "constraint", action },
  };
}

function layer(
  id: string,
  action: string,
  label: string,
  glyph: string,
  tip: string,
  status: ToolStatus = "working",
): CatalogTool {
  return { id, label, glyph, tip, status, kind: { type: "layer", action } };
}

function soon(
  id: string,
  label: string,
  glyph: string,
  tip: string,
): CatalogTool {
  return {
    id,
    label,
    glyph,
    tip: `${tip} — non disponible`,
    status: "unavailable",
    kind: { type: "ui", action: "noop" },
  };
}

/** All catalog entries keyed by id. */
export const CATALOG: Record<string, CatalogTool> = {
  // ——— Draw ———
  line: cad("line", "L", "Ligne", "╱", "Ligne", ["LINE"]),
  pline: cad("pline", "PL", "Pline", "⌇", "Polyligne", ["PLINE"]),
  circle: cad("circle", "C", "Cercle", "○", "Cercle centre-rayon", ["CIRCLE"]),
  circle2p: cad("circle2p", "C2", "2 pts", "◯", "Cercle 2 points (Ø)", [
    "CIRCLE2P",
    "2P",
  ]),
  circle3p: cad("circle3p", "C3", "3 pts", "◎", "Cercle 3 points", [
    "CIRCLE3P",
    "3P",
  ]),
  arc: cad("arc", "A", "Arc", "⌒", "Arc", ["ARC"]),
  rect: cad("rect", "REC", "Rect", "▭", "Rectangle", ["RECTANG", "RECTANGLE"]),
  polygon: cad("polygon", "POL", "Poly", "⬡", "Polygone régulier", [
    "POLYGON",
    "PP",
  ]),
  ellipse: cad("ellipse", "EL", "Ellip", "⬭", "Ellipse", ["ELLIPSE"]),
  spline: cad("spline", "SPL", "Spl", "∿", "Spline", ["SPLINE"]),
  hatch: cad("hatch", "H", "Hach", "▤", "Hachure", ["HATCH"]),
  point: cad("point", "PO", "Pt", "·", "Point", ["POINT"]),
  ray: cad("ray", "RAY", "Ray", "↗", "Rayon", []),
  xline: cad("xline", "XL", "Xln", "↔", "Ligne de construction", ["XLINE"]),
  star: cad("star", "STAR", "★", "★", "Étoile", ["ETOILE"]),
  donut: cad("donut", "DONUT", "Anneau", "◍", "Anneau / donut", [
    "RING",
    "ANNEAU",
  ]),
  capsule: cad("capsule", "CAPSULE", "Capsule", "⬭", "Capsule / slot", [
    "SLOT",
    "STADIUM",
  ]),
  arrow: cad("arrow", "ARROW", "Flèche", "➤", "Flèche", ["FLECHE"]),
  triangle: cad("triangle", "TRI", "△", "△", "Triangle", ["TRIANGLE"]),
  revcloud: cad("revcloud", "RC", "Nuage", "☁", "Nuage de révision", [
    "REVCLOUD",
    "CLOUD",
  ]),
  wipeout: cad("wipeout", "WO", "Wipe", "▣", "Masque wipeout", [
    "WIPEOUT",
    "WIPE",
  ]),
  gridDraw: cad("gridDraw", "GRID", "Grille", "▦", "Grille de construction", [
    "GRILLE",
  ]),

  // ——— Modify ———
  move: cad("move", "M", "Déplacer", "⇔", "Déplacer", ["MOVE"]),
  copy: cad("copy", "CO", "Copier", "⧉", "Copier", ["COPY", "CP"]),
  stretch: cad("stretch", "S", "Étirer", "⇄", "Étirer", ["STRETCH"]),
  rotate: cad("rotate", "RO", "Rotation", "↻", "Rotation", ["ROTATE"]),
  mirror: cad("mirror", "MI", "Miroir", "||-||", "Miroir", ["MIRROR"]),
  scale: cad("scale", "SC", "Échelle", "⤢", "Échelle", ["SCALE"]),
  offset: cad("offset", "O", "Décaler", "⇉", "Décalage", ["OFFSET"]),
  trim: cad("trim", "TR", "Ajuster", "✂", "Ajuster (trim)", ["TRIM"]),
  extend: cad("extend", "EX", "Prolonger", "↦", "Prolonger", ["EXTEND"]),
  lengthen: cad("lengthen", "LEN", "Allonger", "⟷", "Allonger", ["LENGTHEN"]),
  break: cad("break", "BR", "Coupure", "⊥", "Coupure", ["BREAK"]),
  join: cad("join", "J", "Joindre", "⋈", "Joindre", ["JOIN"]),
  fillet: cad("fillet", "F", "Congé", "⌒", "Congé", ["FILLET"]),
  chamfer: cad("chamfer", "CHA", "Chanfrein", "⌐", "Chanfrein", ["CHAMFER"]),
  arrayRect: cad("arrayRect", "AR", "Réseau", "▦", "Réseau rectangulaire", [
    "ARRAYRECT",
    "ARRAY",
  ]),
  arrayPolar: cad("arrayPolar", "APOLAR", "Polaire", "◎", "Réseau polaire", [
    "ARRAYPOLAR",
  ]),
  align: cad("align", "AL", "Aligner", "⇔", "Aligner", ["ALIGN"]),
  erase: cad("erase", "E", "Effacer", "✕", "Effacer", ["ERASE", "DELETE"]),
  explode: cad("explode", "X", "Exploser", "✸", "Exploser", ["EXPLODE"]),
  undo: cad("undo", "U", "Annuler", "↶", "Annuler", ["UNDO"]),
  redo: cad("redo", "REDO", "Rétablir", "↷", "Rétablir", []),

  // ——— Construction / geometry helpers ———
  parallel: cad("parallel", "PAR", "Parallèle", "∥", "Parallèle par un point", [
    "PARALLEL",
  ]),
  midline: cad("midline", "ML", "Milieu", "⌖", "Ligne médiane", ["MIDLINE"]),
  tanPc: cad("tanPc", "LT", "Tangente", "⊙", "Tangente point→cercle", [
    "TANGENTPC",
    "TANPC",
  ]),
  tanCc: cad("tanCc", "LC", "T2C", "⊚", "Tangentes cercle→cercle", [
    "TANGENTCC",
    "TANCC",
  ]),
  section: cad("section", "SEC", "Coupe", "⌖", "Marque de coupe", [
    "SECTION",
    "COUPE",
  ]),
  centermark: cad("centermark", "CM", "Centre", "⌖", "Marque de centre", [
    "CENTERMARK",
    "CMARK",
  ]),
  centerline: cad("centerline", "CL", "Axe", "┼", "Ligne d'axe", [
    "CENTERLINE",
    "CLINE",
  ]),

  // ——— Annotate ———
  text: cad("text", "DT", "Texte", "A", "Texte simple", ["TEXT"]),
  mtext: soon("mtext", "MTexte", "T", "Texte multiligne (modèle prêt)"),
  dimLinear: cad("dimLinear", "DLI", "Linéaire", "↔", "Cote linéaire", [
    "DIMLINEAR",
    "DIMLIN",
  ]),
  dimAligned: cad("dimAligned", "DAL", "Alignée", "⤢", "Cote alignée", [
    "DIMALIGNED",
    "DIMALI",
  ]),
  dimRadius: cad("dimRadius", "DRA", "Rayon", "R", "Cote rayon", [
    "DIMRADIUS",
    "DIMRAD",
  ]),
  dimDiameter: cad("dimDiameter", "DDI", "Diamètre", "⌀", "Cote diamètre", [
    "DIMDIAMETER",
    "DIMDIA",
  ]),
  dimAngular: cad("dimAngular", "DAN", "Angle", "∠", "Cote angulaire", [
    "DIMANGULAR",
    "DIMANG",
  ]),
  dimContinue: cad("dimContinue", "DCO", "Continue", "↕", "Cote continue", [
    "DIMCONTINUE",
    "DIMCONT",
  ]),
  dimBaseline: cad("dimBaseline", "DBA", "Base", "≡", "Cote baseline", [
    "DIMBASELINE",
    "DIMBASE",
  ]),
  leader: cad("leader", "LEAD", "Repère", "→", "Repère / ligne de rappel", [
    "LEADER",
    "MLD",
  ]),
  table: cad("table", "TBL", "Tableau", "▦", "Tableau", ["TABLE"]),
  textStyle: soon("textStyle", "Style txt", "Aa", "Styles de texte"),
  dimStyle: soon("dimStyle", "Style cote", "⌀+", "Styles de cotation"),

  // ——— Insert / blocks ———
  insert: cad("insert", "I", "Insérer", "⬇", "Insérer un bloc", ["INSERT"]),
  block: cad("block", "B", "Créer", "▢", "Définir un bloc", ["BLOCK"]),
  xref: cad("xref", "XR", "XREF", "⛓", "Référence externe", ["XREF"], "partial"),
  pdfAttach: cad(
    "pdfAttach",
    "PDFATTACH",
    "PDF",
    "📄",
    "Sous-couche PDF",
    ["PDF"],
    "partial",
  ),
  wblock: soon("wblock", "WBLOCK", "▭", "Écrire bloc sur disque"),
  image: soon("image", "Image", "🖼", "Insérer une image raster"),
  importDxf: ui(
    "importDxf",
    "import-dxf",
    "DXF ↓",
    "⇩",
    "Importer un fichier DXF",
  ),
  importDwg: soon("importDwg", "DWG ↓", "⇩", "Importer DWG"),
  importSvg: soon("importSvg", "SVG ↓", "⇩", "Importer SVG"),
  vectorize: soon("vectorize", "Vectoriser", "✎", "Vectoriser un raster (API WASM)"),

  // ——— Layers ———
  layerProps: layer(
    "layerProps",
    "focus-panel",
    "Propriétés",
    "☰",
    "Ouvrir le panneau Calques",
  ),
  layerNew: layer("layerNew", "create", "Nouveau", "+", "Créer un calque"),
  layerSetSel: layer(
    "layerSetSel",
    "set-selection",
    "→ Sél.",
    "⤵",
    "Appliquer le calque courant à la sélection",
  ),
  layerFreeze: soon("layerFreeze", "Geler", "❄", "Geler / dégeler (pas d'API set)"),
  layerLock: soon("layerLock", "Verrouiller", "🔒", "Verrouiller (pas d'API set)"),
  layerDelete: soon("layerDelete", "Supprimer", "✕", "Supprimer un calque"),
  layerColor: soon("layerColor", "Couleur", "🎨", "Couleur de calque"),
  layerLtype: soon("layerLtype", "Type", "═", "Type de ligne"),
  layerWeight: soon("layerWeight", "Épaisseur", "━", "Épaisseur de ligne"),

  // ——— Parametric (WASM constraints) ———
  cHorizontal: constraint(
    "cHorizontal",
    "horizontal",
    "Horizontal",
    "━",
    "Contrainte horizontale (1 ligne sélectionnée)",
  ),
  cVertical: constraint(
    "cVertical",
    "vertical",
    "Vertical",
    "┃",
    "Contrainte verticale (1 ligne sélectionnée)",
  ),
  cParallel: constraint(
    "cParallel",
    "parallel",
    "Parallèle",
    "∥",
    "Contrainte parallèle (2 lignes)",
  ),
  cPerp: constraint(
    "cPerp",
    "perpendicular",
    "Perpend.",
    "⊥",
    "Contrainte perpendiculaire (2 lignes)",
  ),
  cCoincident: constraint(
    "cCoincident",
    "coincident",
    "Coïncident",
    "⊙",
    "Coïncidence d'extrémités (2 lignes)",
  ),
  cEqual: constraint(
    "cEqual",
    "equal",
    "Égal",
    "=",
    "Longueurs égales (2 lignes)",
  ),
  cDistance: constraint(
    "cDistance",
    "distance",
    "Distance",
    "↔",
    "Distance dimensionnelle (1 ligne, valeur demandée)",
  ),
  cAngle: constraint(
    "cAngle",
    "angle",
    "Angle",
    "∠",
    "Angle vs +X (1 ligne, degrés demandés)",
  ),
  cSolve: constraint(
    "cSolve",
    "solve",
    "Résoudre",
    "▶",
    "Résoudre le système de contraintes",
  ),
  cTangent: soon(
    "cTangent",
    "Tangence",
    "◠",
    "Contrainte de tangence (pas encore dans le solveur)",
  ),

  // ——— Geometry measure / 2D bool (mostly missing) ———
  measure: soon("measure", "Mesurer", "📐", "Outil de mesure"),
  area: soon("area", "Surface", "▭", "Calcul de surface / périmètre"),
  union2d: soon("union2d", "Union 2D", "∪", "Union de régions 2D"),
  diff2d: soon("diff2d", "Diff. 2D", "∖", "Différence de régions 2D"),
  inter2d: soon("inter2d", "Inter. 2D", "∩", "Intersection de régions 2D"),

  // ——— 3D solids ———
  solidBox: solid("solidBox", "box", "Boîte", "▢", "Primitive box"),
  solidCyl: solid("solidCyl", "cyl", "Cylindre", "⌭", "Primitive cylindre"),
  solidSph: solid("solidSph", "sph", "Sphère", "●", "Primitive sphère"),
  solidCone: solid("solidCone", "cone", "Cône", "△", "Primitive cône"),
  solidTor: solid("solidTor", "tor", "Tore", "◎", "Primitive tore"),
  solidPyr: solid("solidPyr", "pyramid", "Pyramide", "▲", "Primitive pyramide"),
  solidPrism: solid("solidPrism", "prism", "Prisme", "⬡", "Primitive prisme"),
  solidPipe: solid("solidPipe", "pipe", "Tube", "◎", "Primitive tube"),
  solidExtrude: solid(
    "solidExtrude",
    "extrude",
    "Extruder",
    "⬆",
    "Extrusion de la sélection",
  ),
  solidRev: solid(
    "solidRev",
    "rev",
    "Révol.",
    "↻",
    "Révolution de la sélection",
  ),
  solidSweep: solid(
    "solidSweep",
    "sweep",
    "Balayage",
    "⤴",
    "Balayage de la sélection",
  ),
  solidLoft: solid(
    "solidLoft",
    "loft",
    "Lissage",
    "≋",
    "Lissage de la sélection",
  ),
  solidUnion: solid("solidUnion", "union", "Union", "∪", "Union de solides"),
  solidDiff: solid(
    "solidDiff",
    "diff",
    "Soustraire",
    "∖",
    "Différence de solides",
  ),
  solidInter: solid(
    "solidInter",
    "intersect",
    "Intersect.",
    "∩",
    "Intersection de solides",
  ),
  solidClear: solid("solidClear", "clear", "Vider", "🗑", "Effacer les solides"),

  // ——— View ———
  view2d: ui("view2d", "view-2d", "2D", "▢", "Viewport 2D"),
  viewSplit: ui("viewSplit", "view-split", "Split", "⊞", "Vue partagée 2D+3D"),
  view3d: ui("view3d", "view-3d", "3D", "▣", "Viewport 3D"),
  viewPlan: solid("viewPlan", "view-plan", "Plan", "⊤", "Dériver vue plan"),
  viewElevX: solid(
    "viewElevX",
    "view-elevx",
    "Élév. X",
    "⊢",
    "Dériver élévation X",
  ),
  viewElevY: solid(
    "viewElevY",
    "view-elevy",
    "Élév. Y",
    "⊣",
    "Dériver élévation Y",
  ),
  zoomExtents: soon("zoomExtents", "Étendue", "⛶", "Zoom étendu"),
  zoomIn: soon("zoomIn", "Zoom +", "+", "Zoom avant (molette viewport)"),
  zoomOut: soon("zoomOut", "Zoom −", "−", "Zoom arrière (molette viewport)"),
  pan: soon("pan", "Pan", "✋", "Panoramique (molette / espace)"),
  iso: soon("iso", "Isométrique", "◇", "Vue isométrique"),
  shade: soon("shade", "Ombré", "▓", "Mode d'affichage ombré"),
  wire: soon("wire", "Filaire", "☰", "Mode filaire"),
  tplA4: solid("tplA4", "tpl-a4", "A4", "▭", "Gabarit A4"),
  tplA3: solid("tplA3", "tpl-a3", "A3", "▭", "Gabarit A3"),
  tplA2: solid("tplA2", "tpl-a2", "A2", "▭", "Gabarit A2"),
  tplA1: solid("tplA1", "tpl-a1", "A1", "▭", "Gabarit A1"),
  tplArch: solid("tplArch", "tpl-arch", "ARCH", "▭", "Gabarit architecture"),
  tplMech: solid("tplMech", "tpl-mech", "MECH", "▭", "Gabarit mécanique"),

  // ——— Output ———
  fileNew: ui("fileNew", "new", "Nouveau", "⎚", "Nouveau dessin"),
  fileOpen: ui("fileOpen", "open", "Ouvrir", "📂", "Ouvrir .kelicad"),
  fileSave: ui("fileSave", "save", "Sauver", "💾", "Enregistrer"),
  exportPdf: ui("exportPdf", "pdf", "PDF", "⎙", "Exporter PDF"),
  exportDxf: ui("exportDxf", "export-dxf", "DXF ↑", "⇧", "Exporter DXF"),
  exportSvg: ui("exportSvg", "export-svg", "SVG ↑", "🖼", "Exporter SVG"),
  exportIfc: solid("exportIfc", "ifc", "IFC", "🏗", "Exporter IFC (BIM)"),
  exportSched: solid(
    "exportSched",
    "sched",
    "SCHED",
    "📋",
    "Exporter planning CSV",
  ),
  plotLayout: soon("plotLayout", "Mise en page", "▤", "Présentations / plot"),
  print: soon("print", "Imprimer", "🖨", "Impression système"),

  // ——— Extensions ———
  keli: ui("keli", "focus-keli", "KELI", "✦", "Panneau assistant KELI"),
  collab: soon("collab", "Collab", "👥", "Voir barre collaboration"),
  plugins: soon("plugins", "Plugins", "🧩", "Hôte de plugins"),
  macros: soon("macros", "Macros", "⏺", "Enregistrement macro (API)"),
};

export function getTool(id: string): CatalogTool | undefined {
  return CATALOG[id];
}

export function requireTool(id: string): CatalogTool {
  const t = CATALOG[id];
  if (!t) throw new Error(`Unknown catalog tool: ${id}`);
  return t;
}

/** Resolve whether an active command token matches a catalog tool. */
export function toolMatchesActive(
  tool: CatalogTool,
  activeCmd: string | null | undefined,
): boolean {
  if (!activeCmd || tool.kind.type !== "cad") return false;
  const token = activeCmd.trim().toUpperCase();
  return (tool.aliases ?? []).some((a) => a.toUpperCase() === token);
}
