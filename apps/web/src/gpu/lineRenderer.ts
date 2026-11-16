/**
 * WebGPU solid stroke renderer.
 * Expands line segments into screen-constant-width triangle quads
 * (WebGPU line-list is 1px / non-AA and looks dashed on diagonals).
 */

export type Camera2D = {
  /** World X of view center. */
  x: number;
  /** World Y of view center. */
  y: number;
  /** World units visible along the smaller viewport axis. */
  zoom: number;
};

export type LineBatch = {
  r: number;
  g: number;
  b: number;
  /** Interleaved line-list pairs: x0,y0,x1,y1,... */
  verts: Float32Array;
};

const SHADER = /* wgsl */ `
struct Uniforms {
  view_proj: mat4x4f,
  color: vec4f,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs_main(@location(0) position: vec2f) -> @builtin(position) vec4f {
  return u.view_proj * vec4f(position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
  return u.color;
}
`;

function ortho(left: number, right: number, bottom: number, top: number): Float32Array {
  const out = new Float32Array(16);
  out[0] = 2 / (right - left);
  out[5] = 2 / (top - bottom);
  out[10] = -1;
  out[12] = -(right + left) / (right - left);
  out[13] = -(top + bottom) / (top - bottom);
  out[15] = 1;
  return out;
}

/** Expand line-list pairs into triangle-list quads (6 verts / segment). */
function expandStrokes(
  lines: Float32Array,
  halfWidthWorld: number,
): Float32Array {
  const segCount = Math.floor(lines.length / 4);
  if (segCount <= 0) return new Float32Array(0);
  const out = new Float32Array(segCount * 12); // 6 * vec2
  let o = 0;
  const hw = Math.max(halfWidthWorld, 1e-9);
  for (let i = 0; i < segCount; i++) {
    const ax = lines[i * 4]!;
    const ay = lines[i * 4 + 1]!;
    const bx = lines[i * 4 + 2]!;
    const by = lines[i * 4 + 3]!;
    let dx = bx - ax;
    let dy = by - ay;
    const len = Math.hypot(dx, dy);
    if (len < 1e-12) {
      dx = 1;
      dy = 0;
    } else {
      dx /= len;
      dy /= len;
    }
    const px = -dy * hw;
    const py = dx * hw;
    // tri 1: A+p, A-p, B+p
    out[o++] = ax + px;
    out[o++] = ay + py;
    out[o++] = ax - px;
    out[o++] = ay - py;
    out[o++] = bx + px;
    out[o++] = by + py;
    // tri 2: A-p, B-p, B+p
    out[o++] = ax - px;
    out[o++] = ay - py;
    out[o++] = bx - px;
    out[o++] = by - py;
    out[o++] = bx + px;
    out[o++] = by + py;
  }
  return out;
}

type CpuBatch = {
  lines: Float32Array;
  r: number;
  g: number;
  b: number;
};

type GpuBatch = {
  buffer: GPUBuffer;
  count: number;
  r: number;
  g: number;
  b: number;
};

export class LineRenderer {
  private readonly device: GPUDevice;
  private readonly context: GPUCanvasContext;
  private readonly pipeline: GPURenderPipeline;
  private readonly uniformBuffer: GPUBuffer;
  private readonly bindGroup: GPUBindGroup;
  private cpuBatches: CpuBatch[] = [];
  private gpuBatches: GpuBatch[] = [];
  /** Stroke width in CSS pixels. */
  strokePx = 2.0;

  private constructor(
    device: GPUDevice,
    context: GPUCanvasContext,
    pipeline: GPURenderPipeline,
    uniformBuffer: GPUBuffer,
    bindGroup: GPUBindGroup,
  ) {
    this.device = device;
    this.context = context;
    this.pipeline = pipeline;
    this.uniformBuffer = uniformBuffer;
    this.bindGroup = bindGroup;
  }

  static async create(canvas: HTMLCanvasElement): Promise<LineRenderer | null> {
    if (!navigator.gpu) {
      return null;
    }
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) {
      return null;
    }
    const device = await adapter.requestDevice();
    const context = canvas.getContext("webgpu");
    if (!context) {
      return null;
    }
    const format = navigator.gpu.getPreferredCanvasFormat();
    context.configure({
      device,
      format,
      alphaMode: "premultiplied",
    });

    const shader = device.createShaderModule({ code: SHADER });
    const bindGroupLayout = device.createBindGroupLayout({
      entries: [
        {
          binding: 0,
          visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
          buffer: { type: "uniform" },
        },
      ],
    });
    const pipeline = device.createRenderPipeline({
      layout: device.createPipelineLayout({ bindGroupLayouts: [bindGroupLayout] }),
      vertex: {
        module: shader,
        entryPoint: "vs_main",
        buffers: [
          {
            arrayStride: 8,
            attributes: [{ shaderLocation: 0, offset: 0, format: "float32x2" }],
          },
        ],
      },
      fragment: {
        module: shader,
        entryPoint: "fs_main",
        targets: [{ format }],
      },
      // Solid filled strokes (not hairline line-list).
      primitive: { topology: "triangle-list" },
    });

    const uniformBuffer = device.createBuffer({
      size: 96,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    const bindGroup = device.createBindGroup({
      layout: bindGroupLayout,
      entries: [{ binding: 0, resource: { buffer: uniformBuffer } }],
    });

    return new LineRenderer(device, context, pipeline, uniformBuffer, bindGroup);
  }

  /** Legacy single-color path. */
  setLines(vertices: Float32Array): void {
    this.setBatches([{ r: 0.92, g: 0.92, b: 0.92, verts: vertices }]);
  }

  setBatches(batches: LineBatch[]): void {
    this.cpuBatches = batches
      .filter((b) => b.verts.length >= 4)
      .map((b) => ({
        lines: b.verts,
        r: b.r,
        g: b.g,
        b: b.b,
      }));
  }

  resize(width: number, height: number): void {
    const canvas = this.context.canvas as HTMLCanvasElement;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.max(1, Math.floor(width * dpr));
    const h = Math.max(1, Math.floor(height * dpr));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
  }

  private rebuildGpu(halfWidthWorld: number): void {
    for (const b of this.gpuBatches) {
      b.buffer.destroy();
    }
    this.gpuBatches = [];
    for (const batch of this.cpuBatches) {
      const tris = expandStrokes(batch.lines, halfWidthWorld);
      if (!tris.length) continue;
      const buffer = this.device.createBuffer({
        size: tris.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
        mappedAtCreation: true,
      });
      new Float32Array(buffer.getMappedRange()).set(tris);
      buffer.unmap();
      this.gpuBatches.push({
        buffer,
        count: tris.length / 2,
        r: batch.r,
        g: batch.g,
        b: batch.b,
      });
    }
  }

  draw(camera: Camera2D): void {
    const canvas = this.context.canvas as HTMLCanvasElement;
    const aspect = canvas.width / Math.max(canvas.height, 1);
    const halfH = camera.zoom * 0.5;
    const halfW = halfH * aspect;
    const viewProj = ortho(
      camera.x - halfW,
      camera.x + halfW,
      camera.y - halfH,
      camera.y + halfH,
    );

    // Keep stroke ~strokePx CSS pixels wide at current zoom/DPR.
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const worldPerCssPx = camera.zoom / Math.max(canvas.height / dpr, 1);
    const halfWidthWorld = (this.strokePx * 0.5) * worldPerCssPx;
    this.rebuildGpu(halfWidthWorld);

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: 0.06, g: 0.06, b: 0.06, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.bindGroup);

    for (const batch of this.gpuBatches) {
      const color = new Float32Array([batch.r, batch.g, batch.b, 1.0]);
      const payload = new Float32Array(24);
      payload.set(viewProj, 0);
      payload.set(color, 16);
      this.device.queue.writeBuffer(this.uniformBuffer, 0, payload);
      pass.setVertexBuffer(0, batch.buffer);
      pass.draw(batch.count);
    }

    pass.end();
    this.device.queue.submit([encoder.finish()]);
  }

  destroy(): void {
    for (const b of this.gpuBatches) {
      b.buffer.destroy();
    }
    this.gpuBatches = [];
    this.cpuBatches = [];
    this.uniformBuffer.destroy();
  }
}
