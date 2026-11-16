/**
 * WebGPU lit triangle-mesh renderer for solid viewport (orbit camera).
 */

export type Camera3D = {
  /** Look-at target. */
  target: [number, number, number];
  /** Yaw radians (around Z-up). */
  yaw: number;
  /** Pitch radians. */
  pitch: number;
  /** Distance from target. */
  distance: number;
};

export type MeshBatch = {
  positions: Float32Array;
  indices: Uint32Array;
  r: number;
  g: number;
  b: number;
};

const SHADER = /* wgsl */ `
struct Uniforms {
  view_proj: mat4x4f,
  light_dir: vec4f,
  color: vec4f,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
  @builtin(position) pos: vec4f,
  @location(0) nrm: vec3f,
}

@vertex
fn vs_main(
  @location(0) position: vec3f,
  @location(1) normal: vec3f,
) -> VsOut {
  var o: VsOut;
  o.pos = u.view_proj * vec4f(position, 1.0);
  o.nrm = normal;
  return o;
}

@fragment
fn fs_main(inp: VsOut) -> @location(0) vec4f {
  let n = normalize(inp.nrm);
  let l = normalize(u.light_dir.xyz);
  let ndl = max(dot(n, l), 0.0);
  let ambient = 0.22;
  let lit = ambient + (1.0 - ambient) * ndl;
  return vec4f(u.color.rgb * lit, u.color.a);
}
`;

function perspective(fovy: number, aspect: number, near: number, far: number): Float32Array {
  const f = 1 / Math.tan(fovy / 2);
  const out = new Float32Array(16);
  out[0] = f / aspect;
  out[5] = f;
  out[10] = (far + near) / (near - far);
  out[11] = -1;
  out[14] = (2 * far * near) / (near - far);
  return out;
}

function lookAt(
  eye: [number, number, number],
  center: [number, number, number],
  up: [number, number, number],
): Float32Array {
  const zx = eye[0] - center[0];
  const zy = eye[1] - center[1];
  const zz = eye[2] - center[2];
  let zl = Math.hypot(zx, zy, zz) || 1;
  const z0 = zx / zl;
  const z1 = zy / zl;
  const z2 = zz / zl;
  // x = up × z
  let xx = up[1] * z2 - up[2] * z1;
  let xy = up[2] * z0 - up[0] * z2;
  let xz = up[0] * z1 - up[1] * z0;
  let xl = Math.hypot(xx, xy, xz) || 1;
  xx /= xl;
  xy /= xl;
  xz /= xl;
  // y = z × x
  const yx = z1 * xz - z2 * xy;
  const yy = z2 * xx - z0 * xz;
  const yz = z0 * xy - z1 * xx;
  const out = new Float32Array(16);
  out[0] = xx;
  out[1] = yx;
  out[2] = z0;
  out[4] = xy;
  out[5] = yy;
  out[6] = z1;
  out[8] = xz;
  out[9] = yz;
  out[10] = z2;
  out[12] = -(xx * eye[0] + xy * eye[1] + xz * eye[2]);
  out[13] = -(yx * eye[0] + yy * eye[1] + yz * eye[2]);
  out[14] = -(z0 * eye[0] + z1 * eye[1] + z2 * eye[2]);
  out[15] = 1;
  return out;
}

function mul4(a: Float32Array, b: Float32Array): Float32Array {
  const o = new Float32Array(16);
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) {
      o[c * 4 + r] =
        a[0 * 4 + r] * b[c * 4 + 0] +
        a[1 * 4 + r] * b[c * 4 + 1] +
        a[2 * 4 + r] * b[c * 4 + 2] +
        a[3 * 4 + r] * b[c * 4 + 3];
    }
  }
  return o;
}

export function cameraEye(cam: Camera3D): [number, number, number] {
  const cp = Math.cos(cam.pitch);
  const sp = Math.sin(cam.pitch);
  const cy = Math.cos(cam.yaw);
  const sy = Math.sin(cam.yaw);
  // Z-up: yaw around Z, pitch from XY plane
  const x = cam.target[0] + cam.distance * cp * cy;
  const y = cam.target[1] + cam.distance * cp * sy;
  const z = cam.target[2] + cam.distance * sp;
  return [x, y, z];
}

export function fitCameraToAabb(
  min: [number, number, number],
  max: [number, number, number],
): Camera3D {
  const cx = (min[0] + max[0]) * 0.5;
  const cy = (min[1] + max[1]) * 0.5;
  const cz = (min[2] + max[2]) * 0.5;
  const dx = max[0] - min[0];
  const dy = max[1] - min[1];
  const dz = max[2] - min[2];
  const span = Math.max(dx, dy, dz, 1);
  return {
    target: [cx, cy, cz],
    yaw: Math.PI * 0.25,
    pitch: Math.PI * 0.28,
    distance: span * 2.2,
  };
}

type GpuMesh = {
  vbo: GPUBuffer;
  ibo: GPUBuffer;
  indexCount: number;
  r: number;
  g: number;
  b: number;
};

function buildInterleaved(positions: Float32Array, indices: Uint32Array): {
  verts: Float32Array;
  idx: Uint32Array;
} {
  // Expand to unique verts with face normals (flat shading).
  const triCount = Math.floor(indices.length / 3);
  const verts = new Float32Array(triCount * 3 * 6); // pos3 + nrm3
  const idx = new Uint32Array(triCount * 3);
  let vi = 0;
  for (let t = 0; t < triCount; t++) {
    const ia = indices[t * 3] * 3;
    const ib = indices[t * 3 + 1] * 3;
    const ic = indices[t * 3 + 2] * 3;
    const ax = positions[ia];
    const ay = positions[ia + 1];
    const az = positions[ia + 2];
    const bx = positions[ib];
    const by = positions[ib + 1];
    const bz = positions[ib + 2];
    const cx = positions[ic];
    const cy = positions[ic + 1];
    const cz = positions[ic + 2];
    const ux = bx - ax;
    const uy = by - ay;
    const uz = bz - az;
    const vx = cx - ax;
    const vy = cy - ay;
    const vz = cz - az;
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    const nl = Math.hypot(nx, ny, nz) || 1;
    nx /= nl;
    ny /= nl;
    nz /= nl;
    const pts = [
      [ax, ay, az],
      [bx, by, bz],
      [cx, cy, cz],
    ];
    for (let k = 0; k < 3; k++) {
      const base = vi * 6;
      verts[base] = pts[k][0];
      verts[base + 1] = pts[k][1];
      verts[base + 2] = pts[k][2];
      verts[base + 3] = nx;
      verts[base + 4] = ny;
      verts[base + 5] = nz;
      idx[t * 3 + k] = vi;
      vi++;
    }
  }
  return { verts, idx };
}

export class MeshRenderer {
  private readonly device: GPUDevice;
  private readonly context: GPUCanvasContext;
  private readonly pipeline: GPURenderPipeline;
  private readonly uniformBuffer: GPUBuffer;
  private readonly bindGroup: GPUBindGroup;
  private depthTexture: GPUTexture;
  private depthView: GPUTextureView;
  private width = 1;
  private height = 1;
  private meshes: GpuMesh[] = [];
  private wireframe = false;

  private constructor(
    device: GPUDevice,
    context: GPUCanvasContext,
    pipeline: GPURenderPipeline,
    uniformBuffer: GPUBuffer,
    bindGroup: GPUBindGroup,
    depthTexture: GPUTexture,
  ) {
    this.device = device;
    this.context = context;
    this.pipeline = pipeline;
    this.uniformBuffer = uniformBuffer;
    this.bindGroup = bindGroup;
    this.depthTexture = depthTexture;
    this.depthView = depthTexture.createView();
  }

  static async create(canvas: HTMLCanvasElement): Promise<MeshRenderer | null> {
    if (!navigator.gpu) return null;
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) return null;
    const device = await adapter.requestDevice();
    const context = canvas.getContext("webgpu");
    if (!context) return null;
    const format = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format, alphaMode: "premultiplied" });

    const module = device.createShaderModule({ code: SHADER });
    const pipeline = device.createRenderPipeline({
      layout: "auto",
      vertex: {
        module,
        entryPoint: "vs_main",
        buffers: [
          {
            arrayStride: 24,
            attributes: [
              { shaderLocation: 0, offset: 0, format: "float32x3" },
              { shaderLocation: 1, offset: 12, format: "float32x3" },
            ],
          },
        ],
      },
      fragment: {
        module,
        entryPoint: "fs_main",
        targets: [{ format }],
      },
      primitive: {
        topology: "triangle-list",
        cullMode: "none",
      },
      depthStencil: {
        format: "depth24plus",
        depthWriteEnabled: true,
        depthCompare: "less",
      },
    });

    const uniformBuffer = device.createBuffer({
      size: 96,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    const bindGroup = device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: [{ binding: 0, resource: { buffer: uniformBuffer } }],
    });

    const depthTexture = device.createTexture({
      size: [1, 1],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });

    return new MeshRenderer(
      device,
      context,
      pipeline,
      uniformBuffer,
      bindGroup,
      depthTexture,
    );
  }

  setWireframe(on: boolean) {
    this.wireframe = on;
  }

  resize(width: number, height: number) {
    const w = Math.max(1, Math.floor(width));
    const h = Math.max(1, Math.floor(height));
    if (w === this.width && h === this.height) return;
    this.width = w;
    this.height = h;
    this.depthTexture.destroy();
    this.depthTexture = this.device.createTexture({
      size: [w, h],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
    this.depthView = this.depthTexture.createView();
  }

  setMeshes(batches: MeshBatch[]) {
    for (const m of this.meshes) {
      m.vbo.destroy();
      m.ibo.destroy();
    }
    this.meshes = [];
    for (const b of batches) {
      if (!b.positions.length || !b.indices.length) continue;
      const { verts, idx } = buildInterleaved(b.positions, b.indices);
      const vbo = this.device.createBuffer({
        size: verts.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
      });
      this.device.queue.writeBuffer(vbo, 0, verts);
      const ibo = this.device.createBuffer({
        size: idx.byteLength,
        usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
      });
      this.device.queue.writeBuffer(ibo, 0, idx);
      this.meshes.push({
        vbo,
        ibo,
        indexCount: idx.length,
        r: b.r,
        g: b.g,
        b: b.b,
      });
    }
  }

  draw(cam: Camera3D) {
    const eye = cameraEye(cam);
    const view = lookAt(eye, cam.target, [0, 0, 1]);
    const proj = perspective(
      (50 * Math.PI) / 180,
      this.width / Math.max(this.height, 1),
      0.05,
      Math.max(cam.distance * 20, 500),
    );
    const vp = mul4(proj, view);
    const light = new Float32Array([0.45, 0.35, 0.82, 0]);
    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: 0.06, g: 0.08, b: 0.11, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
      depthStencilAttachment: {
        view: this.depthView,
        depthClearValue: 1,
        depthLoadOp: "clear",
        depthStoreOp: "store",
      },
    });
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.bindGroup);

    for (const m of this.meshes) {
      const color = this.wireframe
        ? new Float32Array([0.7, 0.85, 1, 1])
        : new Float32Array([m.r, m.g, m.b, 1]);
      const ubo = new Float32Array(24);
      ubo.set(vp, 0);
      ubo.set(light, 16);
      ubo.set(color, 20);
      this.device.queue.writeBuffer(this.uniformBuffer, 0, ubo);
      pass.setVertexBuffer(0, m.vbo);
      pass.setIndexBuffer(m.ibo, "uint32");
      pass.drawIndexed(m.indexCount);
    }
    pass.end();
    this.device.queue.submit([encoder.finish()]);
  }

  destroy() {
    for (const m of this.meshes) {
      m.vbo.destroy();
      m.ibo.destroy();
    }
    this.meshes = [];
    this.uniformBuffer.destroy();
    this.depthTexture.destroy();
  }
}
