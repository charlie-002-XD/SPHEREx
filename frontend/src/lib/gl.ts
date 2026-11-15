// Frame renderer: WebGL2 path uploads each frame as an R32F texture and
// applies stretch + colormap in the fragment shader. Fallback renders the
// same pipeline in JS onto a 2D canvas when float textures are unavailable.
import { colormapLut, type ColormapName } from "./colormaps";
import {
  ASINH_SOFTENING,
  normalizeValue,
  type StretchKind,
} from "./stretch";

export interface DrawParams {
  lo: number;
  hi: number;
  stretch: StretchKind;
  cmap: ColormapName;
}

export interface FrameRenderer {
  readonly kind: "webgl2" | "canvas2d";
  upload(pixels: Float32Array, width: number, height: number): void;
  draw(params: DrawParams): void;
  dispose(): void;
}

const VERT_SRC = `#version 300 es
layout(location=0) in vec2 a_pos;
out vec2 v_uv;
void main() {
  // row 0 of the row-major payload is drawn at the top of the canvas
  v_uv = vec2(a_pos.x * 0.5 + 0.5, 0.5 - a_pos.y * 0.5);
  gl_Position = vec4(a_pos, 0.0, 1.0);
}`;

const FRAG_SRC = `#version 300 es
precision highp float;
uniform sampler2D u_data;
uniform sampler2D u_cmap;
uniform float u_lo;
uniform float u_hi;
uniform float u_soft;
uniform int u_stretch;
in vec2 v_uv;
out vec4 outColor;
void main() {
  float v = texture(u_data, v_uv).r;
  if (isnan(v) || isinf(v)) {
    outColor = vec4(0.059, 0.055, 0.078, 1.0);
    return;
  }
  float range = max(u_hi - u_lo, 1e-30);
  float t = clamp((v - u_lo) / range, 0.0, 1.0);
  if (u_stretch == 1) {
    t = log(1.0 + t * 999.0) / log(1000.0);
  } else if (u_stretch == 2) {
    t = asinh(t * u_soft) / asinh(u_soft);
  }
  outColor = texture(u_cmap, vec2(t, 0.5));
}`;

function compileShader(
  gl: WebGL2RenderingContext,
  type: number,
  src: string,
): WebGLShader | null {
  const sh = gl.createShader(type);
  if (sh === null) return null;
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (gl.getShaderParameter(sh, gl.COMPILE_STATUS) !== true) {
    gl.deleteShader(sh);
    return null;
  }
  return sh;
}

const STRETCH_INDEX: Record<StretchKind, number> = {
  linear: 0,
  log: 1,
  asinh: 2,
  zscale: 3,
};

export class WebGL2Renderer implements FrameRenderer {
  readonly kind = "webgl2" as const;
  private gl: WebGL2RenderingContext;
  private program: WebGLProgram;
  private dataTex: WebGLTexture;
  private cmapTex: WebGLTexture;
  private cmapName: ColormapName | null = null;
  private w = 0;
  private h = 0;

  private constructor(
    gl: WebGL2RenderingContext,
    program: WebGLProgram,
    dataTex: WebGLTexture,
    cmapTex: WebGLTexture,
  ) {
    this.gl = gl;
    this.program = program;
    this.dataTex = dataTex;
    this.cmapTex = cmapTex;
  }

  static create(canvas: HTMLCanvasElement): WebGL2Renderer | null {
    const gl = canvas.getContext("webgl2", {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      preserveDrawingBuffer: false,
    });
    if (gl === null) return null;
    const vs = compileShader(gl, gl.VERTEX_SHADER, VERT_SRC);
    const fs = compileShader(gl, gl.FRAGMENT_SHADER, FRAG_SRC);
    if (vs === null || fs === null) return null;
    const program = gl.createProgram();
    if (program === null) return null;
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);
    if (gl.getProgramParameter(program, gl.LINK_STATUS) !== true) return null;

    const vao = gl.createVertexArray();
    if (vao === null) return null;
    gl.bindVertexArray(vao);
    const vb = gl.createBuffer();
    if (vb === null) return null;
    gl.bindBuffer(gl.ARRAY_BUFFER, vb);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 3, -1, -1, 3]),
      gl.STATIC_DRAW,
    );
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

    const dataTex = gl.createTexture();
    const cmapTex = gl.createTexture();
    if (dataTex === null || cmapTex === null) return null;
    gl.bindTexture(gl.TEXTURE_2D, dataTex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    // probe: R32F upload must not error, else caller falls back to 2D
    gl.texImage2D(
      gl.TEXTURE_2D, 0, gl.R32F, 1, 1, 0, gl.RED, gl.FLOAT,
      new Float32Array([0]),
    );
    if (gl.getError() !== gl.NO_ERROR) return null;
    gl.bindTexture(gl.TEXTURE_2D, cmapTex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    return new WebGL2Renderer(gl, program, dataTex, cmapTex);
  }

  upload(pixels: Float32Array, width: number, height: number): void {
    const gl = this.gl;
    this.w = width;
    this.h = height;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.dataTex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(
      gl.TEXTURE_2D, 0, gl.R32F, width, height, 0, gl.RED, gl.FLOAT, pixels,
    );
  }

  private uploadCmap(name: ColormapName): void {
    if (this.cmapName === name) return;
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.cmapTex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
    gl.texImage2D(
      gl.TEXTURE_2D, 0, gl.RGBA, 256, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE,
      colormapLut(name),
    );
    this.cmapName = name;
  }

  draw(params: DrawParams): void {
    const gl = this.gl;
    gl.viewport(0, 0, this.w, this.h);
    gl.useProgram(this.program);
    this.uploadCmap(params.cmap);
    gl.uniform1i(gl.getUniformLocation(this.program, "u_data"), 0);
    gl.uniform1i(gl.getUniformLocation(this.program, "u_cmap"), 1);
    gl.uniform1f(gl.getUniformLocation(this.program, "u_lo"), params.lo);
    gl.uniform1f(gl.getUniformLocation(this.program, "u_hi"), params.hi);
    gl.uniform1f(
      gl.getUniformLocation(this.program, "u_soft"),
      ASINH_SOFTENING,
    );
    gl.uniform1i(
      gl.getUniformLocation(this.program, "u_stretch"),
      STRETCH_INDEX[params.stretch],
    );
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.dataTex);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  dispose(): void {
    const gl = this.gl;
    gl.deleteTexture(this.dataTex);
    gl.deleteTexture(this.cmapTex);
    gl.deleteProgram(this.program);
    // No WEBGL_lose_context here: a canvas keeps one context for life, so
    // losing it would blank any renderer re-created on the same canvas
    // (React StrictMode mounts effects twice in development).
  }
}

export class Canvas2DRenderer implements FrameRenderer {
  readonly kind = "canvas2d" as const;
  private ctx: CanvasRenderingContext2D;
  private canvas: HTMLCanvasElement;
  private image: ImageData | null = null;
  private pixels: Float32Array | null = null;

  private constructor(canvas: HTMLCanvasElement, ctx: CanvasRenderingContext2D) {
    this.canvas = canvas;
    this.ctx = ctx;
  }

  static create(canvas: HTMLCanvasElement): Canvas2DRenderer | null {
    const ctx = canvas.getContext("2d");
    return ctx === null ? null : new Canvas2DRenderer(canvas, ctx);
  }

  upload(pixels: Float32Array, width: number, height: number): void {
    this.pixels = pixels;
    this.canvas.width = width;
    this.canvas.height = height;
    this.image = this.ctx.createImageData(width, height);
  }

  draw(params: DrawParams): void {
    if (this.pixels === null || this.image === null) return;
    const lut = colormapLut(params.cmap);
    const out = this.image.data;
    for (let i = 0; i < this.pixels.length; i++) {
      const v = this.pixels[i] ?? NaN;
      const t = normalizeValue(v, params.lo, params.hi, params.stretch);
      const o = i * 4;
      if (Number.isNaN(t)) {
        out[o] = 15;
        out[o + 1] = 14;
        out[o + 2] = 20;
        out[o + 3] = 255;
      } else {
        const li = Math.round(t * 255) * 4;
        out[o] = lut[li] ?? 0;
        out[o + 1] = lut[li + 1] ?? 0;
        out[o + 2] = lut[li + 2] ?? 0;
        out[o + 3] = 255;
      }
    }
    this.ctx.putImageData(this.image, 0, 0);
  }

  dispose(): void {
    this.image = null;
    this.pixels = null;
  }
}

export function createFrameRenderer(
  canvas: HTMLCanvasElement,
): FrameRenderer {
  // Spec: fall back to 2D canvas when float-texture rendering (R32F /
  // EXT_color_buffer_float) is missing. The R32F upload probe inside
  // WebGL2Renderer.create is the definitive capability check.
  const webgl = WebGL2Renderer.create(canvas);
  if (webgl !== null) return webgl;
  const ctx2d = Canvas2DRenderer.create(canvas);
  if (ctx2d !== null) return ctx2d;
  return new NullRenderer();
}

class NullRenderer implements FrameRenderer {
  readonly kind = "canvas2d" as const;
  upload(): void {}
  draw(): void {}
  dispose(): void {}
}
