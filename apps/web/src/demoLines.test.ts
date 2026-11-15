import { describe, expect, it } from "vitest";

import { demoBounds, generateDemoLineVertices } from "./demoLines";

describe("generateDemoLineVertices", () => {
  it("emits four floats per line", () => {
    const v = generateDemoLineVertices(100);
    expect(v.length).toBe(400);
  });

  it("is deterministic", () => {
    const a = generateDemoLineVertices(16);
    const b = generateDemoLineVertices(16);
    expect(Array.from(a)).toEqual(Array.from(b));
  });

  it("bounds grow with count", () => {
    const small = demoBounds(100);
    const large = demoBounds(100_000);
    expect(large.width).toBeGreaterThan(small.width);
    expect(large.height).toBeGreaterThan(small.height);
  });
});
