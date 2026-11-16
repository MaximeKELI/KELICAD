import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import {
  CATALOG,
  TONE_CSS,
  getTool,
  type CatalogTool,
} from "../cad/commandCatalog";
import {
  QAT_IDS,
  RIBBON_TABS,
  type RibbonItem,
  type RibbonTabId,
} from "../cad/ribbonLayout";
import type { CadLayer, CadSession } from "../cad/types";

export type RibbonHandlers = {
  onCommand: (cmd: string) => void;
  onUiAction: (action: string) => void;
  onSolid: (action: string) => void;
  onConstraint: (action: string) => void;
  onLayerAction: (action: string) => void;
  onChanged: () => void;
  onEcho: (lines: string[]) => void;
};

type Props = RibbonHandlers & {
  cad: CadSession;
  revision: number;
  projectName: string;
  viewMode: "2d" | "split" | "3d";
  entityCount: number;
  solidCount: number;
  collapsed?: boolean;
  onCollapsedChange?: (v: boolean) => void;
};

function invokeTool(tool: CatalogTool, h: RibbonHandlers) {
  if (tool.status === "unavailable") {
    h.onEcho([`${tool.label} : non disponible pour l'instant.`]);
    return;
  }
  switch (tool.kind.type) {
    case "cad":
      h.onCommand(tool.kind.cmd);
      break;
    case "ui":
      h.onUiAction(tool.kind.action);
      break;
    case "solid":
      h.onSolid(tool.kind.action);
      break;
    case "constraint":
      h.onConstraint(tool.kind.action);
      break;
    case "layer":
      h.onLayerAction(tool.kind.action);
      break;
  }
}

function ToolButton({
  tool,
  size = "md",
  handlers,
}: {
  tool: CatalogTool;
  size?: "lg" | "md" | "sm";
  handlers: RibbonHandlers;
}) {
  const disabled = tool.status === "unavailable";
  const tone = TONE_CSS[tool.tone] ?? TONE_CSS.muted;
  return (
    <button
      type="button"
      className={`cad-toolbtn cad-toolbtn-${size}${disabled ? " is-disabled" : ""}${
        tool.status === "partial" ? " is-partial" : ""
      }`}
      title={tool.tip}
      disabled={disabled}
      onClick={() => invokeTool(tool, handlers)}
    >
      <span className="cad-toolbtn-glyph" style={{ color: tone }}>
        {tool.glyph}
      </span>
      <span className="cad-toolbtn-text">{tool.label}</span>
    </button>
  );
}

function Flyout({
  mainId,
  itemIds,
  handlers,
  size = "lg",
}: {
  mainId: string;
  itemIds: string[];
  handlers: RibbonHandlers;
  size?: "lg" | "md";
}) {
  const [open, setOpen] = useState(false);
  const [menuPos, setMenuPos] = useState<{ top: number; left: number } | null>(
    null,
  );
  const rootRef = useRef<HTMLDivElement>(null);
  const main = getTool(mainId);

  const placeMenu = () => {
    const el = rootRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setMenuPos({ top: r.bottom + 2, left: r.left });
  };

  useEffect(() => {
    if (!open) return;
    placeMenu();
    const onDoc = (ev: MouseEvent) => {
      if (!rootRef.current?.contains(ev.target as Node)) setOpen(false);
    };
    const onReposition = () => placeMenu();
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("resize", onReposition);
    window.addEventListener("scroll", onReposition, true);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("resize", onReposition);
      window.removeEventListener("scroll", onReposition, true);
    };
  }, [open]);

  if (!main) return null;
  const items = itemIds
    .map((id) => getTool(id))
    .filter((t): t is CatalogTool => !!t);

  return (
    <div className="cad-flyout" ref={rootRef}>
      <div className="cad-flyout-main">
        <ToolButton tool={main} size={size} handlers={handlers} />
        <button
          type="button"
          className="cad-flyout-caret"
          aria-label="Plus"
          aria-expanded={open}
          disabled={main.status === "unavailable"}
          onClick={(e) => {
            e.stopPropagation();
            setOpen((v) => !v);
          }}
        >
          ▾
        </button>
      </div>
      {open && menuPos && (
        <div
          className="cad-flyout-menu is-fixed"
          role="menu"
          style={{ top: menuPos.top, left: menuPos.left }}
        >
          {items.map((it) => (
            <button
              key={it.id}
              type="button"
              role="menuitem"
              className={it.status === "unavailable" ? "is-disabled" : ""}
              disabled={it.status === "unavailable"}
              title={it.tip}
              onClick={() => {
                invokeTool(it, handlers);
                setOpen(false);
              }}
            >
              <span style={{ color: TONE_CSS[it.tone] ?? TONE_CSS.muted }}>
                {it.glyph}
              </span>
              <span>
                {it.label}
                {it.kind.type === "cad" && <small>{it.kind.cmd}</small>}
                {it.status === "unavailable" && <small>indispo.</small>}
                {it.status === "partial" && <small>partiel</small>}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function Panel({
  title,
  children,
  expandIds,
  handlers,
}: {
  title: string;
  children: ReactNode;
  expandIds?: string[];
  handlers: RibbonHandlers;
}) {
  const [open, setOpen] = useState(false);
  const [menuPos, setMenuPos] = useState<{ top: number; left: number } | null>(
    null,
  );
  const rootRef = useRef<HTMLDivElement>(null);
  const expandTools = (expandIds ?? [])
    .map((id) => getTool(id))
    .filter((t): t is CatalogTool => !!t && t.status !== "unavailable");

  const placeMenu = () => {
    const el = rootRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setMenuPos({ top: r.bottom + 2, left: r.left });
  };

  useEffect(() => {
    if (!open) return;
    placeMenu();
    const onDoc = (ev: MouseEvent) => {
      if (!rootRef.current?.contains(ev.target as Node)) setOpen(false);
    };
    const onReposition = () => placeMenu();
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("resize", onReposition);
    window.addEventListener("scroll", onReposition, true);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("resize", onReposition);
      window.removeEventListener("scroll", onReposition, true);
    };
  }, [open]);

  return (
    <div className="cad-panel" ref={rootRef}>
      <div className="cad-panel-body">{children}</div>
      <div className="cad-panel-title">
        <span>{title}</span>
        <button
          type="button"
          className="cad-panel-expand"
          title={expandTools.length ? "Plus d'outils" : title}
          aria-expanded={open}
          disabled={expandTools.length === 0}
          onClick={(e) => {
            e.stopPropagation();
            if (expandTools.length) setOpen((v) => !v);
          }}
        >
          ▾
        </button>
      </div>
      {open && expandTools.length > 0 && menuPos && (
        <div
          className="cad-panel-flyout is-fixed"
          role="menu"
          style={{ top: menuPos.top, left: menuPos.left }}
        >
          {expandTools.map((t) => (
            <button
              key={t.id}
              type="button"
              role="menuitem"
              title={t.tip}
              onClick={() => {
                invokeTool(t, handlers);
                setOpen(false);
              }}
            >
              <span style={{ color: TONE_CSS[t.tone] ?? TONE_CSS.muted }}>
                {t.glyph}
              </span>
              <span>
                {t.label}
                {t.kind.type === "cad" && <small>{t.kind.cmd}</small>}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function IconRows({
  ids,
  handlers,
}: {
  ids: string[];
  handlers: RibbonHandlers;
}) {
  const tools = ids
    .map((id) => getTool(id))
    .filter((t): t is CatalogTool => !!t && t.status !== "unavailable");
  return (
    <div className="cad-icon-rows">
      {tools.map((tool) => (
        <button
          key={tool.id}
          type="button"
          className={`cad-icon-cell${tool.status === "partial" ? " is-partial" : ""}`}
          title={tool.tip}
          onClick={() => invokeTool(tool, handlers)}
        >
          <span
            className="cad-icon-cell-glyph"
            style={{ color: TONE_CSS[tool.tone] ?? TONE_CSS.muted }}
          >
            {tool.glyph}
          </span>
          <span className="cad-icon-cell-label">{tool.label}</span>
        </button>
      ))}
    </div>
  );
}

function PropsStack({
  cad,
  revision,
  onChanged,
  onEcho,
}: {
  cad: CadSession;
  revision: number;
  onChanged: () => void;
  onEcho: (lines: string[]) => void;
}) {
  const color = (() => {
    void revision;
    try {
      return cad.get_sysvar("CECOLOR") || "BYLAYER";
    } catch {
      return "BYLAYER";
    }
  })();
  const ltype = (() => {
    void revision;
    try {
      return cad.get_sysvar("CELTYPE") || "BYLAYER";
    } catch {
      return "BYLAYER";
    }
  })();
  const lweight = (() => {
    void revision;
    try {
      return cad.get_sysvar("CELWEIGHT") || "-1";
    } catch {
      return "-1";
    }
  })();

  const applyProp = (
    kind: "color" | "linetype" | "lineweight",
    value: string,
  ) => {
    try {
      if (kind === "color") cad.set_sysvar("CECOLOR", value);
      else if (kind === "linetype") cad.set_sysvar("CELTYPE", value);
      else cad.set_sysvar("CELWEIGHT", value);

      let sel = 0;
      try {
        sel = JSON.parse(cad.selection_json() as string).length as number;
      } catch {
        sel = 0;
      }
      if (sel > 0) {
        const n = cad.set_selection_props(
          kind === "color" ? value : "",
          kind === "linetype" ? value : "",
          kind === "lineweight" ? value : "",
        );
        onEcho([`${kind} → ${n} objet(s) · courant = ${value}`]);
      } else {
        onEcho([`Courant ${kind} = ${value}`]);
      }
      onChanged();
    } catch (err) {
      onEcho([String(err)]);
    }
  };

  return (
    <div className="cad-prop-stack">
      <label>
        Couleur
        <select
          className="cad-select"
          value={color}
          onChange={(e) => applyProp("color", e.target.value)}
        >
          <option value="BYLAYER">DuCalque</option>
          <option value="BYBLOCK">DuBloc</option>
          <option value="red">Rouge</option>
          <option value="yellow">Jaune</option>
          <option value="green">Vert</option>
          <option value="cyan">Cyan</option>
          <option value="blue">Bleu</option>
          <option value="magenta">Magenta</option>
          <option value="white">Blanc</option>
        </select>
      </label>
      <label>
        Type de ligne
        <select
          className="cad-select"
          value={ltype}
          onChange={(e) => applyProp("linetype", e.target.value)}
        >
          <option value="BYLAYER">DuCalque</option>
          <option value="CONTINUOUS">Continuous</option>
          <option value="DASHED">Dashed</option>
          <option value="CENTER">Center</option>
          <option value="HIDDEN">Hidden</option>
        </select>
      </label>
      <label>
        Épaisseur
        <select
          className="cad-select"
          value={lweight}
          onChange={(e) => applyProp("lineweight", e.target.value)}
        >
          <option value="-1">DuCalque</option>
          <option value="0">0.00 mm</option>
          <option value="25">0.25 mm</option>
          <option value="50">0.50 mm</option>
          <option value="70">0.70 mm</option>
          <option value="100">1.00 mm</option>
        </select>
      </label>
    </div>
  );
}

function LayerCombo({
  cad,
  revision,
  onChanged,
  onEcho,
}: {
  cad: CadSession;
  revision: number;
  onChanged: () => void;
  onEcho: (lines: string[]) => void;
}) {
  const layers = useMemo(() => {
    void revision;
    try {
      return JSON.parse(cad.layers_json()) as CadLayer[];
    } catch {
      return [];
    }
  }, [cad, revision]);
  const current = layers.find((l) => l.current) ?? layers[0];

  return (
    <div className="cad-layer-combo">
      <div className="cad-layer-row">
        <span
          className="cad-swatch cad-swatch-lg"
          style={{
            background: current
              ? `rgb(${current.color.join(",")})`
              : "#888",
          }}
        />
        <select
          className="cad-select cad-select-layer"
          value={current?.id ?? ""}
          onChange={(e) => {
            try {
              cad.set_current_layer(e.target.value);
              onChanged();
              const name =
                layers.find((l) => l.id === e.target.value)?.name ?? "";
              onEcho([`Calque : ${name}`]);
            } catch (err) {
              onEcho([String(err)]);
            }
          }}
        >
          {layers.map((l) => (
            <option key={l.id} value={l.id}>
              {l.name}
            </option>
          ))}
        </select>
      </div>
      <div className="cad-layer-tools">
        <button
          type="button"
          title="Allumer / éteindre"
          className={current?.off ? "is-on" : ""}
          onClick={() => {
            if (!current) return;
            try {
              cad.set_layer_flags(current.id, "toggle", "", "");
              onChanged();
              onEcho([
                `Calque ${current.name} : ${current.off ? "allumé" : "éteint"}`,
              ]);
            } catch (err) {
              onEcho([String(err)]);
            }
          }}
        >
          💡
        </button>
        <button
          type="button"
          title="Verrouiller / déverrouiller"
          className={current?.locked ? "is-on" : ""}
          onClick={() => {
            if (!current) return;
            try {
              cad.set_layer_flags(current.id, "", "", "toggle");
              onChanged();
              onEcho([
                `Calque ${current.name} : ${current.locked ? "déverrouillé" : "verrouillé"}`,
              ]);
            } catch (err) {
              onEcho([String(err)]);
            }
          }}
        >
          🔒
        </button>
        <button
          type="button"
          title="Geler / dégeler"
          className={current?.frozen ? "is-on" : ""}
          onClick={() => {
            if (!current) return;
            try {
              cad.set_layer_flags(current.id, "", "toggle", "");
              onChanged();
              onEcho([
                `Calque ${current.name} : ${current.frozen ? "dégelé" : "gelé"}`,
              ]);
            } catch (err) {
              onEcho([String(err)]);
            }
          }}
        >
          ❄
        </button>
      </div>
    </div>
  );
}

function renderItem(
  item: RibbonItem,
  handlers: RibbonHandlers,
  ctx: {
    cad: CadSession;
    revision: number;
  },
): ReactNode {
  switch (item.type) {
    case "tool": {
      const tool = getTool(item.id);
      if (!tool) return null;
      return (
        <ToolButton
          key={item.id}
          tool={tool}
          size={item.size ?? "md"}
          handlers={handlers}
        />
      );
    }
    case "flyout":
      return (
        <Flyout
          key={`fly-${item.main}`}
          mainId={item.main}
          itemIds={item.items}
          handlers={handlers}
          size={item.size ?? "lg"}
        />
      );
    case "stack":
      return (
        <div key={`stack-${item.ids.join("-")}`} className="cad-stack">
          {item.ids.map((id) => {
            const tool = getTool(id);
            if (!tool || tool.status === "unavailable") return null;
            return (
              <ToolButton
                key={id}
                tool={tool}
                size="sm"
                handlers={handlers}
              />
            );
          })}
        </div>
      );
    case "rows":
      return (
        <IconRows
          key={`rows-${item.ids.join("-")}`}
          ids={item.ids}
          handlers={handlers}
        />
      );
    case "expand":
      // Handled at Panel level via expandIds aggregation.
      return null;
    case "layer-combo":
      return (
        <LayerCombo
          key="layer-combo"
          cad={ctx.cad}
          revision={ctx.revision}
          onChanged={handlers.onChanged}
          onEcho={handlers.onEcho}
        />
      );
    case "props-stack":
      return (
        <PropsStack
          key="props-stack"
          revision={ctx.revision}
          cad={ctx.cad}
          onChanged={handlers.onChanged}
          onEcho={handlers.onEcho}
        />
      );
  }
}

export function Ribbon(props: Props) {
  const {
    cad,
    revision,
    projectName,
    viewMode,
    entityCount,
    solidCount,
    collapsed: collapsedProp,
    onCollapsedChange,
    ...handlers
  } = props;

  const [tab, setTab] = useState<RibbonTabId>("home");
  const [collapsedLocal, setCollapsedLocal] = useState(false);
  const collapsed = collapsedProp ?? collapsedLocal;
  const setCollapsed = onCollapsedChange ?? setCollapsedLocal;

  const active = RIBBON_TABS.find((t) => t.id === tab) ?? RIBBON_TABS[0];

  const qat = QAT_IDS.map((id) => CATALOG[id]).filter(Boolean);

  return (
    <div className={`cad-ribbon${collapsed ? " is-collapsed" : ""}`}>
      <div className="cad-qat">
        <span className="cad-brand">KELICAD</span>
        <div className="cad-qat-btns">
          {qat.map((tool) => (
            <button
              key={tool.id}
              type="button"
              title={tool.tip}
              disabled={tool.status === "unavailable"}
              style={{ color: TONE_CSS[tool.tone] ?? TONE_CSS.file }}
              onClick={() => invokeTool(tool, handlers)}
            >
              {tool.glyph}
            </button>
          ))}
        </div>
        <span className="cad-docname" title={projectName}>
          {projectName}
        </span>
        <span className="cad-stats">
          {entityCount} ent · {solidCount} sol · {viewMode.toUpperCase()}
        </span>
        <button
          type="button"
          className="cad-ribbon-toggle"
          title={collapsed ? "Développer le ruban" : "Réduire le ruban"}
          onClick={() => setCollapsed(!collapsed)}
        >
          {collapsed ? "▾" : "▴"}
        </button>
      </div>

      <div className="cad-tabs">
        {RIBBON_TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            className={`cad-tab ${tab === t.id ? "is-active" : ""}`}
            onClick={() => {
              setTab(t.id);
              if (collapsed) setCollapsed(false);
            }}
          >
            {t.label}
          </button>
        ))}
      </div>

      {!collapsed && (
        <div className="cad-ribbon-body">
          {active.panels.map((panel) => {
            const expandIds = panel.items.flatMap((it) =>
              it.type === "expand" ? it.ids : [],
            );
            return (
              <Panel
                key={`${active.id}-${panel.title}`}
                title={panel.title}
                expandIds={expandIds}
                handlers={handlers}
              >
                {panel.items.map((item, i) => {
                  if (item.type === "expand") return null;
                  return (
                    <span key={i} className="cad-ribbon-item">
                      {renderItem(item, handlers, { cad, revision })}
                    </span>
                  );
                })}
              </Panel>
            );
          })}
        </div>
      )}

      <div className="cad-filetabs">
        <button type="button" className="cad-filetab is-active" title={projectName}>
          {projectName}
        </button>
        <button
          type="button"
          className="cad-filetab-add"
          title="Nouveau dessin"
          onClick={() => handlers.onUiAction("new")}
        >
          +
        </button>
      </div>
    </div>
  );
}
