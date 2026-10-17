// Previa en la GPU: un shader WebGL2 que replica, paso a paso, las etapas por píxel del filtro
// Camera Raw de PhotoCraft (`crates/algo/src/camera_raw.rs`, función `develop`). Las constantes
// y las curvas las calcula el motor (`gpuParams`, ver `apps/revela/src/ajustes.rs`); aquí solo se
// aplican. Todo en `highp` (float de 32 bits, como el motor).
//
// La exportación la hace el motor. `pruebas/igualdad.mjs` compara los dos (≤ 1/255) en el CI y
// la app repite la prueba en el dispositivo (ⓘ › Diagnóstico › Probar GPU).

export interface ParametrosGpu {
  wb: boolean;
  gain: Float32Array;
  contrastK: Float32Array;
  color: boolean;
  satVib: Float32Array;
  lutOn: boolean[];
  luts: Float32Array;
  hsl: boolean;
  hslHue: Float32Array;
  hslSat: Float32Array;
  hslLum: Float32Array;
  grade: boolean;
  tints: Float32Array;
  lums: Float32Array;
  blendBalance: Float32Array;
}

/** Encuadre: tamaño de la salida y transformación salida → foto (ver `geometria.rs`). */
export interface PlanoGpu {
  ancho: number;
  alto: number;
  m: Float32Array;
}

const LUT = 1024;

const VERT = `#version 300 es
// Un triángulo que cubre todo el lienzo.
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;

const FRAG = `#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;

uniform sampler2D uImg;
uniform sampler2D uLut;
uniform int uAlto;
// Geometría: centro del píxel de salida (u + ½, v + ½) → punto de la foto (m · (u', v', 1)).
uniform bool uGeo;
uniform vec3 uMx;
uniform vec3 uMy;
uniform ivec2 uOrigen;
uniform bool uWb;
uniform vec3 uGain;
uniform float uConK;
uniform bool uColor;
uniform float uSat;
uniform float uVib;
uniform bool uLutOn[5];
uniform bool uHsl;
uniform float uHslHue[8];
uniform float uHslSat[8];
uniform float uHslLum[8];
uniform bool uGrade;
uniform vec3 uTint[4];
uniform float uLum[4];
uniform float uBlend;
uniform float uBal;
out vec4 salida;

const float TAU = 6.2831855;
const float BANDAS[8] = float[8](0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 270.0, 300.0);

float luma(vec3 c) { return 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b; }

float s2l(float v) { return v <= 0.04045 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4); }
float l2s(float v) { return v <= 0.0031308 ? v * 12.92 : 1.055 * pow(v, 1.0 / 2.4) - 0.055; }

float suave(float e0, float e1, float x) {
  float t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}

// lut_eval del motor: interpolación lineal; fuera de 0..1 se extiende con pendiente 1.
float curva(int k, float v) {
  if (v < 0.0) return v + texelFetch(uLut, ivec2(0, k), 0).r;
  if (v > 1.0) return texelFetch(uLut, ivec2(${LUT - 1}, k), 0).r + (v - 1.0);
  float x = v * ${(LUT - 1).toFixed(1)};
  int i = min(int(x), ${LUT - 2});
  float t = x - float(i);
  float a = texelFetch(uLut, ivec2(i, k), 0).r;
  float b = texelFetch(uLut, ivec2(i + 1, k), 0).r;
  return a + (b - a) * t;
}

vec3 rgb2hsv(vec3 c) {
  float mx = max(c.r, max(c.g, c.b));
  float mn = min(c.r, min(c.g, c.b));
  float d = mx - mn;
  float h;
  if (d <= 1e-9) h = 0.0;
  else if (mx == c.r) h = 60.0 * mod((c.g - c.b) / d, 6.0);
  else if (mx == c.g) h = 60.0 * ((c.b - c.r) / d + 2.0);
  else h = 60.0 * ((c.r - c.g) / d + 4.0);
  return vec3(h, mx > 1e-9 ? d / mx : 0.0, mx);
}

vec3 hsv2rgb(float h, float s, float v) {
  h = mod(h, 360.0) / 60.0;
  float c = v * s;
  float x = c * (1.0 - abs(mod(h, 2.0) - 1.0));
  int i = int(h);
  vec3 r;
  if (i == 0) r = vec3(c, x, 0.0);
  else if (i == 1) r = vec3(x, c, 0.0);
  else if (i == 2) r = vec3(0.0, c, x);
  else if (i == 3) r = vec3(0.0, x, c);
  else if (i == 4) r = vec3(x, 0.0, c);
  else r = vec3(c, 0.0, x);
  return r + (v - c);
}

// Bilineal con los centros de píxel en i + ½ y los vecinos sujetos al borde; fuera de la foto,
// transparente. Igual que geometria::bilineal en el núcleo.
vec4 muestra(float x, float y) {
  if (!(x >= 0.0 && y >= 0.0 && x <= float(uOrigen.x) && y <= float(uOrigen.y))) return vec4(0.0);
  float fx = x - 0.5;
  float fy = y - 0.5;
  float ix = floor(fx);
  float iy = floor(fy);
  float tx = fx - ix;
  float ty = fy - iy;
  ivec2 lim = uOrigen - 1;
  int x0 = clamp(int(ix), 0, lim.x);
  int y0 = clamp(int(iy), 0, lim.y);
  int x1 = clamp(int(ix) + 1, 0, lim.x);
  int y1 = clamp(int(iy) + 1, 0, lim.y);
  vec4 a = texelFetch(uImg, ivec2(x0, y0), 0);
  vec4 b = texelFetch(uImg, ivec2(x1, y0), 0);
  vec4 c = texelFetch(uImg, ivec2(x0, y1), 0);
  vec4 d = texelFetch(uImg, ivec2(x1, y1), 0);
  vec4 arriba = a + (b - a) * tx;
  vec4 abajo = c + (d - c) * tx;
  return arriba + (abajo - arriba) * ty;
}

void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  p.y = uAlto - 1 - p.y;
  vec4 src;
  if (uGeo) {
    vec2 q = vec2(float(p.x) + 0.5, float(p.y) + 0.5);
    src = muestra(uMx.x * q.x + uMx.y * q.y + uMx.z, uMy.x * q.x + uMy.y * q.y + uMy.z);
  } else {
    src = texelFetch(uImg, p, 0);
  }
  vec3 c = src.rgb;

  // 1. Temperatura/tinte y exposición, en luz lineal.
  if (uWb) {
    c = vec3(l2s(max(s2l(c.r) * uGain.r, 0.0)), l2s(max(s2l(c.g) * uGain.g, 0.0)), l2s(max(s2l(c.b) * uGain.b, 0.0)));
  }

  // 3. Contraste: curva en S sobre la luminancia, conservando las proporciones de color.
  if (uConK != 0.0) {
    float l0 = max(luma(c), 0.0);
    float lc = min(l0, 1.0);
    float lv = lc;
    float xx = clamp(lv, 0.0, 1.0);
    lv += -uConK * sin(TAU * xx) / TAU;
    lv = max(lv, 0.0) + (l0 - lc);
    if (l0 > 1e-5) c = c * (lv / l0);
    else c = c + (lv - l0);
  }

  // 4–7. Color.
  if (uColor) {
    if (uSat != 0.0 || uVib != 0.0) {
      float l = luma(c);
      float mx = max(c.r, max(c.g, c.b));
      float mn = min(c.r, min(c.g, c.b));
      float sat = mx > 1e-6 ? (mx - mn) / mx : 0.0;
      float hue = rgb2hsv(c).x;
      float piel = clamp(1.0 - abs(hue - 25.0) / 25.0, 0.0, 1.0) * 0.5;
      float k = 1.0 + uSat + uVib * (1.0 - sat) * (1.0 - piel);
      c = l + (c - l) * max(k, 0.0);
    }
    if (uLutOn[0]) c = vec3(curva(0, c.r), curva(0, c.g), curva(0, c.b));
    if (uLutOn[1]) c = vec3(curva(1, c.r), curva(1, c.g), curva(1, c.b));
    if (uLutOn[2]) c.r = curva(2, c.r);
    if (uLutOn[3]) c.g = curva(3, c.g);
    if (uLutOn[4]) c.b = curva(4, c.b);
    if (uHsl) {
      vec3 hsv = rgb2hsv(max(c, 0.0));
      if (hsv.y > 1e-4) {
        float dh = 0.0, ds = 0.0, dl = 0.0;
        for (int k = 0; k < 8; k++) {
          float centro = BANDAS[k];
          float antes = BANDAS[(k + 7) % 8];
          float despues = BANDAS[(k + 1) % 8];
          float dif = mod(hsv.x - centro + 540.0, 360.0) - 180.0;
          float ancho = dif >= 0.0 ? mod(despues - centro, 360.0) : mod(centro - antes, 360.0);
          float w = max(1.0 - abs(dif) / max(ancho, 1e-3), 0.0);
          dh += w * uHslHue[k];
          ds += w * uHslSat[k];
          dl += w * uHslLum[k];
        }
        float nh = hsv.x + dh / 100.0 * 30.0;
        float ns = clamp(hsv.y * (1.0 + ds / 100.0), 0.0, 1.0);
        vec3 o = hsv2rgb(nh, ns, hsv.z);
        float l0 = luma(c);
        float l1 = luma(o);
        if (l1 > 1e-6) o = o * l0 / l1;
        c = o * exp2(dl / 100.0 * 1.0 * hsv.y);
      }
    }
    if (uGrade) {
      float l = clamp(luma(c), 0.0, 1.0);
      float sombras = 1.0 - suave(0.0, 0.5 + uBal + uBlend, l);
      float luces = suave(0.5 + uBal - uBlend, 1.0, l);
      float medios = max(1.0 - sombras - luces, 0.0);
      float m[4] = float[4](sombras, medios, luces, 1.0);
      for (int k = 0; k < 4; k++) {
        c += m[k] * uTint[k];
        if (uLum[k] != 0.0) c *= exp2(m[k] * uLum[k] / 100.0 * 0.6);
      }
    }
  }
  salida = vec4(clamp(c, 0.0, 1.0), src.a);
}`;

export function hayWebGL2(): boolean {
  try {
    return !!document.createElement('canvas').getContext('webgl2');
  } catch {
    return false;
  }
}

/** Dibuja una foto con ajustes en un `<canvas>` usando la GPU. */
export class Revelador {
  private gl: WebGL2RenderingContext;
  private prog: WebGLProgram;
  private img: WebGLTexture;
  private lut: WebGLTexture;
  private u = new Map<string, WebGLUniformLocation | null>();
  /** Tamaño de la foto cargada (la textura). */
  ancho = 0;
  alto = 0;
  /** Tamaño de lo último dibujado. */
  salida: [number, number] = [0, 0];

  constructor(readonly lienzo: HTMLCanvasElement) {
    const gl = lienzo.getContext('webgl2', { antialias: false, premultipliedAlpha: false, preserveDrawingBuffer: true });
    if (!gl) throw new Error('este navegador no tiene WebGL2');
    this.gl = gl;
    this.prog = programa(gl, VERT, FRAG);
    this.img = textura(gl);
    this.lut = textura(gl);
  }

  /** Información de la GPU para el diagnóstico. */
  descripcion(): string {
    const gl = this.gl;
    const f = gl.getShaderPrecisionFormat(gl.FRAGMENT_SHADER, gl.HIGH_FLOAT);
    const ext = gl.getExtension('WEBGL_debug_renderer_info');
    const nombre = ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
    return `${nombre} · highp float: ${f ? `${f.precision} bits` : 'desconocida'}`;
  }

  /** La foto sin ajustes (RGBA8, filas de arriba abajo). */
  cargar(ancho: number, alto: number, pixeles: Uint8ClampedArray | Uint8Array) {
    const gl = this.gl;
    this.ancho = ancho;
    this.alto = alto;
    this.salida = [ancho, alto];
    this.lienzo.width = ancho;
    this.lienzo.height = alto;
    gl.bindTexture(gl.TEXTURE_2D, this.img);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, ancho, alto, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(pixeles.buffer, pixeles.byteOffset, pixeles.byteLength));
  }

  /**
   * Dibuja con los ajustes `p` (o sin ajustes si es `null`, para comparar antes/después) y el
   * encuadre `plano` (o la foto tal cual si es `null`).
   */
  dibujar(p: ParametrosGpu | null, plano: PlanoGpu | null = null) {
    const gl = this.gl;
    const w = plano ? plano.ancho : this.ancho;
    const h = plano ? plano.alto : this.alto;
    this.salida = [w, h];
    if (this.lienzo.width !== w) this.lienzo.width = w;
    if (this.lienzo.height !== h) this.lienzo.height = h;
    gl.viewport(0, 0, w, h);
    gl.useProgram(this.prog);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.img);
    gl.uniform1i(this.loc('uImg'), 0);
    gl.uniform1i(this.loc('uAlto'), h);
    gl.uniform1i(this.loc('uGeo'), plano ? 1 : 0);
    if (plano) {
      gl.uniform3f(this.loc('uMx'), plano.m[0], plano.m[1], plano.m[2]);
      gl.uniform3f(this.loc('uMy'), plano.m[3], plano.m[4], plano.m[5]);
      gl.uniform2i(this.loc('uOrigen'), this.ancho, this.alto);
    }
    const nada = !p;
    gl.uniform1i(this.loc('uWb'), nada || !p.wb ? 0 : 1);
    gl.uniform1f(this.loc('uConK'), nada ? 0 : p.contrastK[0]);
    gl.uniform1i(this.loc('uColor'), nada || !p.color ? 0 : 1);
    if (p) {
      gl.uniform3fv(this.loc('uGain'), p.gain);
      gl.uniform1f(this.loc('uSat'), p.satVib[0]);
      gl.uniform1f(this.loc('uVib'), p.satVib[1]);
      gl.uniform1iv(this.loc('uLutOn'), p.lutOn.map((b) => (b ? 1 : 0)));
      gl.uniform1i(this.loc('uHsl'), p.hsl ? 1 : 0);
      gl.uniform1fv(this.loc('uHslHue'), p.hslHue);
      gl.uniform1fv(this.loc('uHslSat'), p.hslSat);
      gl.uniform1fv(this.loc('uHslLum'), p.hslLum);
      gl.uniform1i(this.loc('uGrade'), p.grade ? 1 : 0);
      gl.uniform3fv(this.loc('uTint'), p.tints);
      gl.uniform1fv(this.loc('uLum'), p.lums);
      gl.uniform1f(this.loc('uBlend'), p.blendBalance[0]);
      gl.uniform1f(this.loc('uBal'), p.blendBalance[1]);
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, this.lut);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.R32F, LUT, 5, 0, gl.RED, gl.FLOAT, p.luts);
      gl.uniform1i(this.loc('uLut'), 1);
    }
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  /** Lo que hay dibujado, RGBA8 de arriba abajo (como la previa del motor). */
  leer(): Uint8Array {
    const gl = this.gl;
    const [w, h] = this.salida;
    const abajoArriba = new Uint8Array(w * h * 4);
    gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, abajoArriba);
    const out = new Uint8Array(w * h * 4);
    for (let y = 0; y < h; y++) out.set(abajoArriba.subarray((h - 1 - y) * w * 4, (h - y) * w * 4), y * w * 4);
    return out;
  }

  private loc(nombre: string): WebGLUniformLocation | null {
    if (!this.u.has(nombre)) this.u.set(nombre, this.gl.getUniformLocation(this.prog, nombre));
    return this.u.get(nombre) ?? null;
  }
}

function textura(gl: WebGL2RenderingContext): WebGLTexture {
  const t = gl.createTexture();
  if (!t) throw new Error('no se pudo crear una textura');
  gl.bindTexture(gl.TEXTURE_2D, t);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return t;
}

function programa(gl: WebGL2RenderingContext, vs: string, fs: string): WebGLProgram {
  const compilar = (tipo: number, src: string) => {
    const s = gl.createShader(tipo);
    if (!s) throw new Error('no se pudo crear el shader');
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(`shader: ${gl.getShaderInfoLog(s)}`);
    return s;
  };
  const p = gl.createProgram();
  if (!p) throw new Error('no se pudo crear el programa');
  gl.attachShader(p, compilar(gl.VERTEX_SHADER, vs));
  gl.attachShader(p, compilar(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(`programa: ${gl.getProgramInfoLog(p)}`);
  return p;
}
