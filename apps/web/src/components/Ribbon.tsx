import { useMemo, useState, type ReactNode } from "react";

import {
  CATALOG,
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
      <span className="cad-toolbtn-glyph">{tool.glyph}</span>
      {size !== "sm" && (
        <span className="cad-toolbtn-text">{tool.label}</span>
      )}
    </button>
  );
}

function Flyout({
  mainId,
  itemIds,
  handlers,
}: {
  mainId: string;
  itemIds: string[];
  handlers: RibbonHandlers;
}) {
  const [open, setOpen] = useState(false);
  const main = getTool(mainId);
  if (!main) return null;
  const items = itemIds
    .map((id) => getTool(id))
    .filter((t): t is CatalogTool => !!t);

  return (
    <div className="cad-flyout" onMouseLeave={() => setOpen(false)}>
      <div className="cad-flyout-main">
        <ToolButton tool={main} size="md" handlers={handlers} />
        <button
          type="button"
          className="cad-flyout-caret"
          aria-label="Plus"
          disabled={main.status === "unavailable"}
          onClick={() => setOpen((v) => !v)}
        >
          ▾
        </button>
      </div>
      {open && (
        <div className="cad-flyout-menu">
          {items.map((it) => (
            <button
              key={it.id}
              type="button"
              className={it.status === "unavailable" ? "is-disabled" : ""}
              disabled={it.status === "unavailable"}
              title={it.tip}
              onClick={() => {
                invokeTool(it, handlers);
                setOpen(false);
              }}
            >
              <span>{it.glyph}</span>
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
  const expandTools = (expandIds ?? [])
    .map((id) => getTool(id))
    .filter((t): t is CatalogTool => !!t && t.status !== "unavailable");

  return (
    <div className="cad-panel">
      <div className="cad-panel-body">{children}</div>
      <div className="cad-panel-title">
        <span>{title}</span>
        {expandTools.length > 0 && (
          <button
            type="button"
            className="cad-panel-expand"
            title="Plus d'outils"
            onClick={() => setOpen((v) => !v)}
          >
            ▾
          </button>
        )}
      </div>
      {open && expandTools.length > 0 && (
        <div className="cad-panel-flyout">
          {expandTools.map((t) => (
            <button
              key={t.id}
              type="button"
              title={t.tip}
              onClick={() => {
                invokeTool(t, handlers);
                setOpen(false);
              }}
            >
              <span>{t.glyph}</span>
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
          <span className="cad-icon-cell-glyph">{tool.glyph}</span>
          <span className="cad-icon-cell-label">{tool.label}</span>
        </button>
      ))}
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
      <span
        className="cad-swatch"
        style={{
          background: current
            ? `rgb(${current.color.join(",")})`
            : "#888",
        }}
      />
      <select
        className="cad-select"
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
            {l.frozen ? " ❄" : ""}
            {l.locked ? " 🔒" : ""}
          </option>
        ))}
      </select>
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
                {panel.items.map((item, i) => (
                  <span key={i} className="cad-ribbon-item">
                    {renderItem(item, handlers, { cad, revision })}
                  </span>
                ))}
              </Panel>
            );
          })}
        </div>
      )}
    </div>
  );
}
