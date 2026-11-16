/**
 * AutoCAD-like ribbon structure. Every tool id references commandCatalog.
 */

export type RibbonTabId =
  | "home"
  | "insert"
  | "annotate"
  | "parametric"
  | "layers"
  | "blocks"
  | "geometry"
  | "solid"
  | "view"
  | "output"
  | "extensions";

export type RibbonItem =
  | { type: "tool"; id: string; size?: "lg" | "md" | "sm" }
  | { type: "flyout"; main: string; items: string[]; size?: "lg" | "md" }
  | { type: "stack"; ids: string[] }
  | { type: "layer-combo" };

export type RibbonPanel = {
  title: string;
  items: RibbonItem[];
};

export type RibbonTab = {
  id: RibbonTabId;
  label: string;
  panels: RibbonPanel[];
};

export const RIBBON_TABS: RibbonTab[] = [
  {
    id: "home",
    label: "Accueil",
    panels: [
      {
        title: "Dessiner",
        items: [
          { type: "tool", id: "line", size: "lg" },
          { type: "tool", id: "pline", size: "lg" },
          {
            type: "flyout",
            main: "circle",
            items: ["circle", "circle2p", "circle3p", "donut"],
          },
          {
            type: "flyout",
            main: "arc",
            items: ["arc", "revcloud"],
          },
          {
            type: "stack",
            ids: ["rect", "polygon", "ellipse", "hatch"],
          },
          {
            type: "stack",
            ids: ["spline", "point", "ray", "xline"],
          },
          {
            type: "stack",
            ids: ["star", "triangle", "capsule", "arrow"],
          },
        ],
      },
      {
        title: "Modifier",
        items: [
          { type: "tool", id: "move", size: "md" },
          { type: "tool", id: "copy", size: "md" },
          { type: "tool", id: "stretch", size: "md" },
          { type: "tool", id: "rotate", size: "md" },
          { type: "tool", id: "mirror", size: "md" },
          { type: "tool", id: "scale", size: "md" },
          { type: "tool", id: "offset", size: "md" },
          {
            type: "flyout",
            main: "trim",
            items: ["trim", "extend", "lengthen", "break"],
          },
          {
            type: "flyout",
            main: "fillet",
            items: ["fillet", "chamfer"],
          },
          {
            type: "flyout",
            main: "arrayRect",
            items: ["arrayRect", "arrayPolar"],
          },
          { type: "tool", id: "join", size: "sm" },
          { type: "tool", id: "align", size: "sm" },
          { type: "tool", id: "explode", size: "sm" },
          { type: "tool", id: "erase", size: "md" },
        ],
      },
      {
        title: "Annoter",
        items: [
          { type: "tool", id: "text", size: "lg" },
          {
            type: "flyout",
            main: "dimLinear",
            items: [
              "dimLinear",
              "dimAligned",
              "dimRadius",
              "dimDiameter",
              "dimAngular",
              "dimContinue",
              "dimBaseline",
            ],
          },
          { type: "tool", id: "hatch", size: "md" },
          { type: "tool", id: "leader", size: "sm" },
        ],
      },
      {
        title: "Calques",
        items: [
          { type: "tool", id: "layerProps", size: "lg" },
          { type: "layer-combo" },
          {
            type: "stack",
            ids: ["layerNew", "layerSetSel", "layerFreeze", "layerLock"],
          },
        ],
      },
      {
        title: "Bloc",
        items: [
          { type: "tool", id: "insert", size: "lg" },
          { type: "tool", id: "block", size: "md" },
          { type: "tool", id: "explode", size: "md" },
        ],
      },
      {
        title: "Propriétés",
        items: [
          { type: "tool", id: "undo", size: "sm" },
          { type: "tool", id: "redo", size: "sm" },
        ],
      },
    ],
  },
  {
    id: "insert",
    label: "Insérer",
    panels: [
      {
        title: "Bloc",
        items: [
          { type: "tool", id: "insert", size: "lg" },
          { type: "tool", id: "block", size: "lg" },
          { type: "tool", id: "wblock", size: "md" },
          { type: "tool", id: "explode", size: "md" },
        ],
      },
      {
        title: "Référence",
        items: [
          { type: "tool", id: "xref", size: "lg" },
          { type: "tool", id: "pdfAttach", size: "lg" },
          { type: "tool", id: "image", size: "md" },
        ],
      },
      {
        title: "Import",
        items: [
          { type: "tool", id: "importDxf", size: "lg" },
          { type: "tool", id: "importDwg", size: "md" },
          { type: "tool", id: "importSvg", size: "md" },
          { type: "tool", id: "vectorize", size: "md" },
        ],
      },
    ],
  },
  {
    id: "annotate",
    label: "Annoter",
    panels: [
      {
        title: "Texte",
        items: [
          { type: "tool", id: "text", size: "lg" },
          { type: "tool", id: "mtext", size: "md" },
          { type: "tool", id: "table", size: "md" },
          { type: "tool", id: "leader", size: "md" },
          { type: "tool", id: "textStyle", size: "sm" },
        ],
      },
      {
        title: "Cotes",
        items: [
          { type: "tool", id: "dimLinear", size: "lg" },
          { type: "tool", id: "dimAligned", size: "md" },
          { type: "tool", id: "dimRadius", size: "md" },
          { type: "tool", id: "dimDiameter", size: "md" },
          { type: "tool", id: "dimAngular", size: "md" },
          { type: "tool", id: "dimContinue", size: "sm" },
          { type: "tool", id: "dimBaseline", size: "sm" },
          { type: "tool", id: "dimStyle", size: "sm" },
        ],
      },
      {
        title: "Repères",
        items: [
          { type: "tool", id: "centermark", size: "md" },
          { type: "tool", id: "centerline", size: "md" },
          { type: "tool", id: "revcloud", size: "md" },
          { type: "tool", id: "section", size: "md" },
        ],
      },
    ],
  },
  {
    id: "parametric",
    label: "Paramétrique",
    panels: [
      {
        title: "Géométriques",
        items: [
          { type: "tool", id: "cHorizontal", size: "md" },
          { type: "tool", id: "cVertical", size: "md" },
          { type: "tool", id: "cParallel", size: "md" },
          { type: "tool", id: "cPerp", size: "md" },
          { type: "tool", id: "cCoincident", size: "md" },
          { type: "tool", id: "cEqual", size: "md" },
          { type: "tool", id: "cTangent", size: "md" },
        ],
      },
      {
        title: "Dimensionnelles",
        items: [
          { type: "tool", id: "cDistance", size: "lg" },
          { type: "tool", id: "cAngle", size: "md" },
          { type: "tool", id: "cSolve", size: "lg" },
        ],
      },
    ],
  },
  {
    id: "layers",
    label: "Calques",
    panels: [
      {
        title: "Gestion",
        items: [
          { type: "tool", id: "layerProps", size: "lg" },
          { type: "layer-combo" },
          { type: "tool", id: "layerNew", size: "md" },
          { type: "tool", id: "layerSetSel", size: "md" },
          { type: "tool", id: "layerDelete", size: "md" },
        ],
      },
      {
        title: "État",
        items: [
          { type: "tool", id: "layerFreeze", size: "md" },
          { type: "tool", id: "layerLock", size: "md" },
        ],
      },
      {
        title: "Propriétés",
        items: [
          { type: "tool", id: "layerColor", size: "md" },
          { type: "tool", id: "layerLtype", size: "md" },
          { type: "tool", id: "layerWeight", size: "md" },
        ],
      },
    ],
  },
  {
    id: "blocks",
    label: "Blocs",
    panels: [
      {
        title: "Blocs",
        items: [
          { type: "tool", id: "insert", size: "lg" },
          { type: "tool", id: "block", size: "lg" },
          { type: "tool", id: "explode", size: "md" },
          { type: "tool", id: "wblock", size: "md" },
        ],
      },
    ],
  },
  {
    id: "geometry",
    label: "Géométrie",
    panels: [
      {
        title: "Construction",
        items: [
          { type: "tool", id: "parallel", size: "md" },
          { type: "tool", id: "midline", size: "md" },
          { type: "tool", id: "tanPc", size: "md" },
          { type: "tool", id: "tanCc", size: "md" },
          { type: "tool", id: "gridDraw", size: "md" },
          { type: "tool", id: "xline", size: "md" },
          { type: "tool", id: "ray", size: "md" },
        ],
      },
      {
        title: "Édition",
        items: [
          { type: "tool", id: "fillet", size: "md" },
          { type: "tool", id: "chamfer", size: "md" },
          { type: "tool", id: "offset", size: "md" },
          { type: "tool", id: "trim", size: "md" },
          { type: "tool", id: "extend", size: "md" },
          { type: "tool", id: "section", size: "md" },
        ],
      },
      {
        title: "Mesure / Bool 2D",
        items: [
          { type: "tool", id: "measure", size: "md" },
          { type: "tool", id: "area", size: "md" },
          { type: "tool", id: "union2d", size: "md" },
          { type: "tool", id: "diff2d", size: "md" },
          { type: "tool", id: "inter2d", size: "md" },
        ],
      },
    ],
  },
  {
    id: "solid",
    label: "3D",
    panels: [
      {
        title: "Primitives",
        items: [
          { type: "tool", id: "solidBox", size: "lg" },
          { type: "tool", id: "solidCyl", size: "md" },
          { type: "tool", id: "solidSph", size: "md" },
          { type: "tool", id: "solidCone", size: "md" },
          { type: "tool", id: "solidTor", size: "md" },
          { type: "tool", id: "solidPyr", size: "sm" },
          { type: "tool", id: "solidPrism", size: "sm" },
          { type: "tool", id: "solidPipe", size: "sm" },
        ],
      },
      {
        title: "Création",
        items: [
          { type: "tool", id: "solidExtrude", size: "lg" },
          { type: "tool", id: "solidRev", size: "md" },
          { type: "tool", id: "solidSweep", size: "md" },
          { type: "tool", id: "solidLoft", size: "md" },
        ],
      },
      {
        title: "Booléens",
        items: [
          { type: "tool", id: "solidUnion", size: "md" },
          { type: "tool", id: "solidDiff", size: "md" },
          { type: "tool", id: "solidInter", size: "md" },
          { type: "tool", id: "solidClear", size: "sm" },
        ],
      },
    ],
  },
  {
    id: "view",
    label: "Affichage",
    panels: [
      {
        title: "Viewport",
        items: [
          { type: "tool", id: "view2d", size: "lg" },
          { type: "tool", id: "viewSplit", size: "md" },
          { type: "tool", id: "view3d", size: "md" },
        ],
      },
      {
        title: "Vues dérivées",
        items: [
          { type: "tool", id: "viewPlan", size: "md" },
          { type: "tool", id: "viewElevX", size: "md" },
          { type: "tool", id: "viewElevY", size: "md" },
        ],
      },
      {
        title: "Navigation",
        items: [
          { type: "tool", id: "zoomExtents", size: "md" },
          { type: "tool", id: "zoomIn", size: "sm" },
          { type: "tool", id: "zoomOut", size: "sm" },
          { type: "tool", id: "pan", size: "sm" },
          { type: "tool", id: "iso", size: "sm" },
        ],
      },
      {
        title: "Style",
        items: [
          { type: "tool", id: "wire", size: "sm" },
          { type: "tool", id: "shade", size: "sm" },
        ],
      },
      {
        title: "Gabarits",
        items: [
          { type: "stack", ids: ["tplA4", "tplA3", "tplA2", "tplA1"] },
          { type: "stack", ids: ["tplArch", "tplMech"] },
        ],
      },
    ],
  },
  {
    id: "output",
    label: "Sortie",
    panels: [
      {
        title: "Fichier",
        items: [
          { type: "tool", id: "fileNew", size: "md" },
          { type: "tool", id: "fileOpen", size: "md" },
          { type: "tool", id: "fileSave", size: "md" },
        ],
      },
      {
        title: "Export",
        items: [
          { type: "tool", id: "exportPdf", size: "lg" },
          { type: "tool", id: "exportDxf", size: "md" },
          { type: "tool", id: "exportSvg", size: "md" },
          { type: "tool", id: "exportIfc", size: "md" },
          { type: "tool", id: "exportSched", size: "md" },
        ],
      },
      {
        title: "Présentation",
        items: [
          { type: "tool", id: "plotLayout", size: "md" },
          { type: "tool", id: "print", size: "md" },
        ],
      },
    ],
  },
  {
    id: "extensions",
    label: "Extensions",
    panels: [
      {
        title: "Assistants",
        items: [
          { type: "tool", id: "keli", size: "lg" },
          { type: "tool", id: "collab", size: "md" },
        ],
      },
      {
        title: "SDK",
        items: [
          { type: "tool", id: "plugins", size: "md" },
          { type: "tool", id: "macros", size: "md" },
        ],
      },
    ],
  },
];

/** Left tool-rail groups — only working CAD commands. */
export const TOOL_RAIL_GROUPS: { title: string; ids: string[] }[] = [
  {
    title: "Dessin",
    ids: [
      "line",
      "pline",
      "rect",
      "circle",
      "arc",
      "ellipse",
      "spline",
      "polygon",
      "point",
      "hatch",
      "xline",
      "ray",
      "donut",
      "revcloud",
    ],
  },
  {
    title: "Modifier",
    ids: [
      "move",
      "copy",
      "stretch",
      "rotate",
      "scale",
      "mirror",
      "offset",
      "trim",
      "extend",
      "lengthen",
      "break",
      "join",
      "fillet",
      "chamfer",
      "arrayRect",
      "explode",
      "erase",
    ],
  },
  {
    title: "Cotes",
    ids: [
      "dimLinear",
      "dimAligned",
      "dimRadius",
      "dimDiameter",
      "dimAngular",
      "dimContinue",
      "leader",
      "text",
      "table",
    ],
  },
  {
    title: "Blocs",
    ids: ["block", "insert", "explode"],
  },
];

export const QAT_IDS = [
  "fileNew",
  "fileOpen",
  "fileSave",
  "exportPdf",
  "undo",
  "redo",
] as const;
