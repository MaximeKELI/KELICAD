/** Deterministic demo line vertices: interleaved [x0,y0,x1,y1,…]. */
export function generateDemoLineVertices(count: number): Float32Array {
  const out = new Float32Array(count * 4);
  const cols = Math.max(1, Math.ceil(Math.sqrt(count)));
  for (let i = 0; i < count; i += 1) {
    const col = i % cols;
    const row = Math.floor(i / cols);
    const x0 = col * 2;
    const y0 = row * 2;
    const horizontal = i % 2 === 0;
    const x1 = horizontal ? x0 + 1.5 : x0;
    const y1 = horizontal ? y0 : y0 + 1.5;
    const o = i * 4;
    out[o] = x0;
    out[o + 1] = y0;
    out[o + 2] = x1;
    out[o + 3] = y1;
  }
  return out;
}

/** World-space bounds of the demo grid. */
export function demoBounds(count: number): { width: number; height: number } {
  const cols = Math.max(1, Math.ceil(Math.sqrt(count)));
  const rows = Math.max(1, Math.ceil(count / cols));
  return { width: cols * 2 + 2, height: rows * 2 + 2 };
}
