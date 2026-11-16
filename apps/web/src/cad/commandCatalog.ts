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

/** AutoCAD-like icon accent colors. */
export type ToolTone =
  | "draw"
  | "modify"
  | "annotate"
  | "layer"
  | "block"
  | "inquire"
  | "view"
  | "file"
  | "muted";

export const TONE_CSS: Record<ToolTone, string> = {
  draw: "#6cb6ff",
  modify: "#9ad1ff",
  annotate: "#f2f2f2",
  layer: "#f0c000",
  block: "#8fd19e",
  inquire: "#f5d76e",
  view: "#b8c4d0",
  file: "#d0d0d0",
  muted: "#a0a0a0",
};

export type CatalogTool = {
  id: string;
  label: string;
  glyph: string;
  tip: string;
  status: ToolStatus;
  kind: ToolKind;
  tone: ToolTone;
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
  tone: ToolTone = "draw",
): CatalogTool {
  return {
    id,
    label,
    glyph,
    tip: `${tip} (${cmd})`,
    status,
    tone,
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
  tone: ToolTone = "view",
): CatalogTool {
  return { id, label, glyph, tip, status, tone, kind: { type: "ui", action } };
}

function solid(
  id: string,
  action: string,
  label: string,
  glyph: string,
  tip: string,
  status: ToolStatus = "working",
  tone: ToolTone = "block",
): CatalogTool {
  return {
    id,
    label,
    glyph,
    tip,
    status,
    tone,
    kind: { type: "solid", action },
  };
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
    tone: "annotate",
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
  return {
    id,
    label,
    glyph,
    tip,
    status,
    tone: "layer",
    kind: { type: "layer", action },
  };
}

function soon(
  id: string,
  label: string,
  glyph: string,
  tip: string,
  tone: ToolTone = "muted",
): CatalogTool {
  return {
    id,
    label,
    glyph,
    tip: `${tip} — non disponible`,
    status: "unavailable",
    tone,
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
  ], "working", "annotate"),
  wipeout: cad("wipeout", "WO", "Wipe", "▣", "Masque wipeout", [
    "WIPEOUT",
    "WIPE",
  ]),
  gridDraw: cad("gridDraw", "GRID", "Grille", "▦", "Grille de construction", [
    "GRILLE",
  ]),

  // ——— Modify ———
  move: cad("move", "M", "Déplacer", "⇔", "Déplacer", ["MOVE"], "working", "modify"),
  copy: cad("copy", "CO", "Copier", "⧉", "Copier", ["COPY", "CP"], "working", "modify"),
  stretch: cad("stretch", "S", "Étirer", "⇄", "Étirer", ["STRETCH"], "working", "modify"),
  rotate: cad("rotate", "RO", "Rotation", "↻", "Rotation", ["ROTATE"], "working", "modify"),
  mirror: cad("mirror", "MI", "Miroir", "||-||", "Miroir", ["MIRROR"], "working", "modify"),
  scale: cad("scale", "SC", "Échelle", "⤢", "Échelle", ["SCALE"], "working", "modify"),
  offset: cad("offset", "O", "Décaler", "⇉", "Décalage", ["OFFSET"], "working", "modify"),
  trim: cad("trim", "TR", "Ajuster", "✂", "Ajuster (trim)", ["TRIM"], "working", "modify"),
  extend: cad("extend", "EX", "Prolonger", "↦", "Prolonger", ["EXTEND"], "working", "modify"),
  lengthen: cad("lengthen", "LEN", "Allonger", "⟷", "Allonger", ["LENGTHEN"], "working", "modify"),
  break: cad("break", "BR", "Coupure", "⊥", "Coupure", ["BREAK"], "working", "modify"),
  join: cad("join", "J", "Joindre", "⋈", "Joindre", ["JOIN"], "working", "modify"),
  fillet: cad("fillet", "F", "Congé", "⌒", "Congé", ["FILLET"], "working", "modify"),
  chamfer: cad("chamfer", "CHA", "Chanfrein", "⌐", "Chanfrein", ["CHAMFER"], "working", "modify"),
  arrayRect: cad("arrayRect", "AR", "Réseau", "▦", "Réseau rectangulaire", [
    "ARRAYRECT",
    "ARRAY",
  ], "working", "modify"),
  arrayPolar: cad("arrayPolar", "APOLAR", "Polaire", "◎", "Réseau polaire", [
    "ARRAYPOLAR",
  ], "working", "modify"),
  align: cad("align", "AL", "Aligner", "⇔", "Aligner", ["ALIGN"], "working", "modify"),
  erase: cad("erase", "E", "Effacer", "✕", "Effacer", ["ERASE", "DELETE"], "working", "modify"),
  explode: cad("explode", "X", "Exploser", "✸", "Exploser", ["EXPLODE"], "working", "modify"),
  undo: cad("undo", "U", "Annuler", "↶", "Annuler", ["UNDO"], "working", "file"),
  redo: cad("redo", "REDO", "Rétablir", "↷", "Rétablir", [], "working", "file"),

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
  ], "working", "annotate"),
  centermark: cad("centermark", "CM", "Centre", "⌖", "Marque de centre", [
    "CENTERMARK",
    "CMARK",
  ], "working", "annotate"),
  centerline: cad("centerline", "CL", "Axe", "┼", "Ligne d'axe", [
    "CENTERLINE",
    "CLINE",
  ], "working", "annotate"),

  // ——— Annotate ———
  text: cad("text", "DT", "Texte", "A", "Texte simple", ["TEXT"], "working", "annotate"),
  mtext: cad("mtext", "MT", "MText", "¶", "Texte multiligne", ["MTEXT"], "working", "annotate"),
  dimLinear: cad("dimLinear", "DLI", "Linéaire", "↔", "Cote linéaire", [
    "DIMLINEAR",
    "DIMLIN",
  ], "working", "annotate"),
  dimAligned: cad("dimAligned", "DAL", "Alignée", "⤢", "Cote alignée", [
    "DIMALIGNED",
    "DIMALI",
  ], "working", "annotate"),
  dimRadius: cad("dimRadius", "DRA", "Rayon", "R", "Cote rayon", [
    "DIMRADIUS",
    "DIMRAD",
  ], "working", "annotate"),
  dimDiameter: cad("dimDiameter", "DDI", "Diamètre", "⌀", "Cote diamètre", [
    "DIMDIAMETER",
    "DIMDIA",
  ], "working", "annotate"),
  dimAngular: cad("dimAngular", "DAN", "Angle", "∠", "Cote angulaire", [
    "DIMANGULAR",
    "DIMANG",
  ], "working", "annotate"),
  dimContinue: cad("dimContinue", "DCO", "Continue", "↕", "Cote continue", [
    "DIMCONTINUE",
    "DIMCONT",
  ], "working", "annotate"),
  dimBaseline: cad("dimBaseline", "DBA", "Base", "≡", "Cote baseline", [
    "DIMBASELINE",
    "DIMBASE",
  ], "working", "annotate"),
  leader: cad("leader", "LEAD", "Repère", "→", "Repère / ligne de rappel", [
    "LEADER",
    "MLD",
  ], "working", "annotate"),
  table: cad("table", "TBL", "Tableau", "▦", "Tableau", ["TABLE"], "working", "annotate"),
  textStyle: soon("textStyle", "Style txt", "Aa", "Styles de texte"),
  dimStyle: soon("dimStyle", "Style cote", "⌀+", "Styles de cotation"),

  // ——— Insert / blocks ———
  insert: cad("insert", "I", "Insérer", "⬇", "Insérer un bloc", ["INSERT"], "working", "block"),
  block: cad("block", "B", "Créer", "▢", "Définir un bloc", ["BLOCK"], "working", "block"),
  xref: cad("xref", "XR", "XREF", "⛓", "Référence externe", ["XREF"], "partial", "block"),
  pdfAttach: cad(
    "pdfAttach",
    "PDFATTACH",
    "PDF",
    "📄",
    "Sous-couche PDF",
    ["PDF"],
    "partial",
    "block",
  ),
  wblock: cad("wblock", "WB", "WBlock", "📦", "Écrire bloc", ["WBLOCK"], "working", "block"),
  image: soon("image", "Image", "🖼", "Insérer une image raster"),
  importDxf: ui(
    "importDxf",
    "import-dxf",
    "DXF ↓",
    "⇩",
    "Importer un fichier DXF",
    "working",
    "file",
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

  // ——— Inquiry (ACA « Renseignements ») ———
  measure: ui(
    "measure",
    "inquiry-dist",
    "Distance",
    "📐",
    "Distance AABB de la sélection",
    "working",
    "inquire",
  ),
  area: ui(
    "area",
    "inquiry-area",
    "Aire",
    "▭",
    "Aire / périmètre AABB de la sélection",
    "working",
    "inquire",
  ),
  list: ui(
    "list",
    "inquiry-list",
    "Liste",
    "☰",
    "Lister les objets sélectionnés",
    "working",
    "inquire",
  ),
  idEnt: ui(
    "idEnt",
    "inquiry-id",
    "ID",
    "ℹ",
    "Afficher les IDs sélectionnés",
    "working",
    "inquire",
  ),
  union2d: soon("union2d", "Union 2D", "∪", "Union de régions 2D"),
  diff2d: soon("diff2d", "Diff. 2D", "∖", "Différence de régions 2D"),
  inter2d: soon("inter2d", "Inter. 2D", "∩", "Intersection de régions 2D"),

  // ——— Drawing aids ———
  ortho: ui("ortho", "toggle-ortho", "ORTHO", "⊥", "Basculer Ortho (F8)", "working", "view"),
  osnap: ui("osnap", "toggle-osnap", "OSNAP", "⌖", "Basculer accrochage (F3)", "working", "view"),
  selectClear: ui(
    "selectClear",
    "select-clear",
    "Désél.",
    "◌",
    "Effacer la sélection",
    "working",
    "view",
  ),

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
  view2d: ui("view2d", "view-2d", "2D", "▢", "Viewport 2D", "working", "view"),
  viewSplit: ui("viewSplit", "view-split", "Split", "⊞", "Vue partagée 2D+3D", "working", "view"),
  view3d: ui("view3d", "view-3d", "3D", "▣", "Viewport 3D", "working", "view"),
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
  zoomExtents: ui(
    "zoomExtents",
    "view-extents",
    "Étendue",
    "⛶",
    "Zoom étendu sur le dessin",
    "working",
    "view",
  ),
  zoomIn: ui("zoomIn", "view-zoom-in", "Zoom +", "+", "Zoom avant", "working", "view"),
  zoomOut: ui("zoomOut", "view-zoom-out", "Zoom −", "−", "Zoom arrière", "working", "view"),
  pan: soon("pan", "Pan", "✋", "Panoramique — maintenir Espace dans le viewport"),
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
  fileNew: ui("fileNew", "new", "Nouveau", "⎚", "Nouveau dessin", "working", "file"),
  fileOpen: ui("fileOpen", "open", "Ouvrir", "📂", "Ouvrir .kelicad", "working", "file"),
  fileSave: ui("fileSave", "save", "Sauver", "💾", "Enregistrer", "working", "file"),
  exportPdf: ui("exportPdf", "pdf", "PDF", "⎙", "Exporter PDF", "working", "file"),
  exportDxf: ui("exportDxf", "export-dxf", "DXF ↑", "⇧", "Exporter DXF", "working", "file"),
  exportSvg: ui("exportSvg", "export-svg", "SVG ↑", "🖼", "Exporter SVG", "working", "file"),
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
  keli: ui("keli", "focus-keli", "KELI", "✦", "Panneau assistant KELI", "working", "file"),
  collab: soon("collab", "Collab", "👥", "Voir barre collaboration"),
  plugins: soon("plugins", "Plugins", "🧩", "Hôte de plugins"),
  macros: soon("macros", "Macros", "⏺", "Enregistrement macro (API)"),

  // ——— Lot B / sysvars ———
  setvar: cad("setvar", "SETVAR", "SETVAR", "⚙", "Variable système", ["SET"], "working", "file"),
  getvar: cad("getvar", "GETVAR", "GETVAR", "?", "Lire variable", ["GET"], "working", "file"),
  pedit: cad("pedit", "PE", "Pedit", "⌁", "Éditer polyligne", ["PEDIT"], "working", "modify"),
  matchprop: cad("matchprop", "MA", "Matchprop", "🖌", "Copier propriétés", ["MATCHPROP"], "working", "modify"),
  draworder: cad("draworder", "DR", "Ordre", "⇵", "Ordre de dessin", ["DRAWORDER"], "working", "modify"),
  group: cad("group", "GRP", "Groupe", "⧉", "Groupe", ["GROUP"], "working", "modify"),
  ungroup: cad("ungroup", "UG", "Dégroupe", "⧉̸", "Dégrouper", ["UNGROUP"], "working", "modify"),
  overkill: cad("overkill", "OV", "Overkill", "✂", "Supprimer doublons", ["OVERKILL"], "working", "modify"),
  oops: cad("oops", "OOPS", "Oops", "↩", "Restaurer effacement", [], "working", "modify"),
  divide: cad("divide", "DIV", "Diviser", "➗", "Diviser", ["DIVIDE"], "working", "modify"),
  cadMeasure: cad("cadMeasure", "ME", "MEASURE", "┊", "Points à intervalle", ["MEASURE"], "working", "modify"),
  boundary: cad("boundary", "BO", "Contour", "⬡", "Contour", ["BOUNDARY"], "working", "draw"),
  select: cad("select", "SELECT", "Sélect.", "▢", "Sélection", [], "working", "modify"),
  selectSimilar: cad("selectSimilar", "SELECTSIMILAR", "Similaire", "▣", "Sélection similaire", [], "working", "modify"),
  qselect: cad("qselect", "QSELECT", "QSelect", "⚙▢", "Sélection rapide", ["QSE"], "working", "modify"),
  selectAll: cad("selectAll", "SELECTALL", "Tout", "▦", "Tout sélectionner", [], "working", "modify"),
  selectPrev: cad("selectPrev", "SELECTPREV", "Préc.", "↶▢", "Sélection précédente", [], "working", "modify"),
  ddptype: cad("ddptype", "DDPTYPE", "Style pt", "•", "Style de point", ["PTYPE"], "working", "draw"),
  arrayPath: cad("arrayPath", "ARRAYPATH", "Réseau chemin", "〰▦", "Réseau sur chemin", ["APATH"], "working", "modify"),
  layiso: cad("layiso", "LAYISO", "Isoler", "👁", "Isoler calque", [], "working", "layer"),
  layoff: cad("layoff", "LAYOFF", "Éteindre", "○", "Éteindre calque", [], "working", "layer"),
  layfrz: cad("layfrz", "LAYFRZ", "Geler", "❄", "Geler calque", [], "working", "layer"),
  laylck: cad("laylck", "LAYLCK", "Verrouiller", "🔒", "Verrouiller calque", [], "working", "layer"),
  laycur: cad("laycur", "LAYCUR", "Courant", "★", "Calque courant", [], "working", "layer"),
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
