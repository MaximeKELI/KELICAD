/**
 * WebGPU line-list renderer with multi-batch layer colors.
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
  private batches: GpuBatch[] = [];

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
      primitive: { topology: "line-list" },
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
    this.setBatches([{ r: 0.55, g: 0.78, b: 1.0, verts: vertices }]);
  }

  setBatches(batches: LineBatch[]): void {
    for (const b of this.batches) {
      b.buffer.destroy();
    }
    this.batches = [];
    for (const batch of batches) {
      if (!batch.verts.length) continue;
      const buffer = this.device.createBuffer({
        size: batch.verts.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
        mappedAtCreation: true,
      });
      new Float32Array(buffer.getMappedRange()).set(batch.verts);
      buffer.unmap();
      this.batches.push({
        buffer,
        count: batch.verts.length / 2,
        r: batch.r,
        g: batch.g,
        b: batch.b,
      });
    }
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

    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: 0.043, g: 0.071, b: 0.125, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.bindGroup);

    for (const batch of this.batches) {
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
    for (const b of this.batches) {
      b.buffer.destroy();
    }
    this.batches = [];
    this.uniformBuffer.destroy();
  }
}
