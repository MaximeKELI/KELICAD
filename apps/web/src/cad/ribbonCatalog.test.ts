import { describe, expect, it } from "vitest";

import { CATALOG, getTool, type CatalogTool } from "./commandCatalog";
import { QAT_IDS, RIBBON_TABS, type RibbonItem } from "./ribbonLayout";

function collectIds(items: RibbonItem[]): string[] {
  const ids: string[] = [];
  for (const it of items) {
    switch (it.type) {
      case "tool":
        ids.push(it.id);
        break;
      case "flyout":
        ids.push(it.main, ...it.items);
        break;
      case "stack":
      case "rows":
      case "expand":
        ids.push(...it.ids);
        break;
      case "layer-combo":
      case "props-stack":
        break;
    }
  }
  return ids;
}

describe("command catalog", () => {
  it("has unique tool ids", () => {
    const ids = Object.keys(CATALOG);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("every cad tool has a non-empty command token", () => {
    const cadTools = Object.values(CATALOG).filter(
      (t): t is CatalogTool & { kind: { type: "cad"; cmd: string } } =>
        t.kind.type === "cad",
    );
    expect(cadTools.length).toBeGreaterThan(20);
    for (const t of cadTools) {
      expect(t.kind.cmd.trim().length).toBeGreaterThan(0);
      if (t.status === "working") {
        expect(t.kind.cmd).toMatch(/^[A-Z0-9]+$/i);
      }
    }
  });
});

describe("ribbon layout", () => {
  it("every referenced tool id exists in the catalog", () => {
    const missing: string[] = [];
    for (const id of QAT_IDS) {
      if (!getTool(id)) missing.push(`qat:${id}`);
    }
    for (const tab of RIBBON_TABS) {
      for (const panel of tab.panels) {
        for (const id of collectIds(panel.items)) {
          if (!getTool(id)) missing.push(`${tab.id}/${panel.title}:${id}`);
        }
      }
    }
    expect(missing).toEqual([]);
  });

  it("has at least one flyout and one expand panel", () => {
    let flyouts = 0;
    let expands = 0;
    for (const tab of RIBBON_TABS) {
      for (const panel of tab.panels) {
        for (const it of panel.items) {
          if (it.type === "flyout") flyouts += 1;
          if (it.type === "expand") expands += 1;
        }
      }
    }
    expect(flyouts).toBeGreaterThan(0);
    expect(expands).toBeGreaterThan(0);
  });

  it("working cad tools are invokable (not soon/noop)", () => {
    const broken: string[] = [];
    for (const tab of RIBBON_TABS) {
      for (const panel of tab.panels) {
        for (const id of collectIds(panel.items)) {
          const t = getTool(id);
          if (!t || t.status !== "working") continue;
          if (t.kind.type === "ui" && t.kind.action === "noop") {
            broken.push(id);
          }
        }
      }
    }
    expect(broken).toEqual([]);
  });
});
