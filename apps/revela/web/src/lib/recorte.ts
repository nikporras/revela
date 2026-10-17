// Geometría del recorte en la interfaz: girar, voltear, enderezar y recortar sin esquinas vacías.
// El núcleo (`geometria.rs`) hace el remuestreo; aquí solo se decide el rectángulo.
//
// Convenciones (las mismas que el núcleo): el marco es la foto ya girada y volteada, de
// `ancho`×`alto` píxeles; el enderezado gira la foto `angulo` grados en el sentido del reloj
// alrededor del centro; el recorte es `[cx, cy, w, h]` en fracciones del marco.

export type Rect = [number, number, number, number];

export type Proporcion = 'libre' | 'original' | '1:1' | '4:5' | '3:4' | '9:16' | '16:9';

export const PROPORCIONES: { id: Proporcion; nombre: string }[] = [
  { id: 'libre', nombre: 'Libre' },
  { id: 'original', nombre: 'Original' },
  { id: '1:1', nombre: '1:1' },
  { id: '4:5', nombre: '4:5' },
  { id: '3:4', nombre: '3:4' },
  { id: '9:16', nombre: '9:16' },
  { id: '16:9', nombre: '16:9' },
];

export interface Geometria {
  /** Giros de 90° en el sentido del reloj (0..3). */
  giro: number;
  espejoH: boolean;
  espejoV: boolean;
  /** Grados (−45..45), positivo en el sentido del reloj. */
  angulo: number;
  recorte: Rect | null;
  proporcion: Proporcion;
  /** El usuario movió el recorte a mano: al enderezar solo se encoge lo necesario. */
  manual: boolean;
}

export const ANGULO_MAX = 45;
export const COMPLETO: Rect = [0.5, 0.5, 1, 1];

export function geometriaVacia(): Geometria {
  return { giro: 0, espejoH: false, espejoV: false, angulo: 0, recorte: null, proporcion: 'libre', manual: false };
}

export function esGeometriaVacia(g: Geometria): boolean {
  return g.giro === 0 && !g.espejoH && !g.espejoV && g.angulo === 0 && (!g.recorte || g.recorte.every((v, i) => Math.abs(v - COMPLETO[i]) < 1e-9));
}

/** El JSON que entiende el núcleo; `sinRecorte` muestra el marco entero (mientras se recorta). */
export function geometriaJson(g: Geometria, sinRecorte = false): string {
  return JSON.stringify({ giro: g.giro, espejoH: g.espejoH, espejoV: g.espejoV, angulo: g.angulo, recorte: sinRecorte ? null : g.recorte });
}

/** Tamaño del marco (la foto girada) para una foto de `w`×`h`. */
export function marco(g: Pick<Geometria, 'giro'>, w: number, h: number): [number, number] {
  return g.giro % 2 === 1 ? [h, w] : [w, h];
}

/** Ancho/alto en píxeles de una proporción (`null`: libre). */
export function aspecto(p: Proporcion, fw: number, fh: number): number | null {
  switch (p) {
    case 'libre':
      return null;
    case 'original':
      return fw / fh;
    default: {
      const [a, b] = p.split(':').map(Number);
      return a / b;
    }
  }
}

/** ¿El recorte cae entero dentro de la foto enderezada (y del marco)? */
export function valido(r: Rect, angulo: number, fw: number, fh: number): boolean {
  const eps = 1e-6;
  const [cx, cy, w, h] = r;
  if (w <= 0 || h <= 0) return false;
  if (cx - w / 2 < -eps || cy - h / 2 < -eps || cx + w / 2 > 1 + eps || cy + h / 2 > 1 + eps) return false;
  const t = (-angulo * Math.PI) / 180;
  const [s, c] = [Math.sin(t), Math.cos(t)];
  const tol = 0.25; // un cuarto de píxel
  for (const [dx, dy] of [
    [-1, -1],
    [1, -1],
    [-1, 1],
    [1, 1],
  ]) {
    const px = (cx + (dx * w) / 2) * fw - fw / 2;
    const py = (cy + (dy * h) / 2) * fh - fh / 2;
    const qx = c * px - s * py + fw / 2;
    const qy = s * px + c * py + fh / 2;
    if (qx < -tol || qy < -tol || qx > fw + tol || qy > fh + tol) return false;
  }
  return true;
}

/** El recorte más grande con ancho/alto `a` (px) centrado en la foto enderezada. */
export function maximo(a: number, angulo: number, fw: number, fh: number): Rect {
  const t = (Math.abs(angulo) * Math.PI) / 180;
  const [s, c] = [Math.sin(t), Math.cos(t)];
  // Semiancho máximo: las cuatro esquinas, devueltas al marco sin enderezar, caben en la foto.
  const hw = Math.min(fw / 2 / (c + s / a), fh / 2 / (s + c / a), fw / 2, (fh / 2) * a) * (1 - 1e-6);
  return [0.5, 0.5, (2 * hw) / fw, (2 * hw) / a / fh];
}

function mezclar(a: Rect, b: Rect, t: number): Rect {
  return [0, 1, 2, 3].map((i) => a[i] + (b[i] - a[i]) * t) as Rect;
}

/**
 * `deseado` si es válido; si no, lo más cerca de él (en línea recta desde `valido_`, que sí lo
 * es) sin salirse de la foto. Sirve para mover, redimensionar y encoger.
 */
export function encajar(deseado: Rect, valido_: Rect, angulo: number, fw: number, fh: number): Rect {
  if (valido(deseado, angulo, fw, fh)) return deseado;
  let [lo, hi] = [0, 1];
  for (let i = 0; i < 30; i++) {
    const m = (lo + hi) / 2;
    if (valido(mezclar(valido_, deseado, m), angulo, fw, fh)) lo = m;
    else hi = m;
  }
  return mezclar(valido_, deseado, lo);
}

/** Proporción en fracciones del marco (ancho/alto) para `a` en píxeles. */
export function aspectoNormal(a: number, fw: number, fh: number): number {
  return (a * fh) / fw;
}

/** Vuelve a encajar el recorte tras cambiar el ángulo o la proporción. */
export function reencuadrar(g: Geometria, fw: number, fh: number): Rect {
  const actual = g.recorte ?? COMPLETO;
  const a = aspecto(g.proporcion, fw, fh) ?? (actual[2] * fw) / (actual[3] * fh);
  const max = maximo(a, g.angulo, fw, fh);
  if (!g.manual) return max;
  // A mano: se conserva centro y tamaño si caben; si no, se encoge hacia el máximo.
  return encajar(actual, max, g.angulo, fw, fh);
}

/** Girar 90° en sentido contrario al reloj (como en las galerías de los teléfonos). */
export function girar(g: Geometria): Geometria {
  const r = g.recorte ?? COMPLETO;
  // Un punto (x, y) del marco va a (y, 1 − x) en el nuevo.
  const recorte: Rect = [r[1], 1 - r[0], r[3], r[2]];
  const fija = g.proporcion !== 'libre' && g.proporcion !== 'original' && g.proporcion !== '1:1';
  return {
    ...g,
    giro: (g.giro + 3) % 4,
    // Girar intercambia los ejes de los volteos.
    espejoH: g.espejoV,
    espejoV: g.espejoH,
    recorte,
    proporcion: fija ? 'libre' : g.proporcion,
  };
}

export function voltearH(g: Geometria): Geometria {
  const r = g.recorte ?? COMPLETO;
  return { ...g, espejoH: !g.espejoH, angulo: -g.angulo, recorte: [1 - r[0], r[1], r[2], r[3]] };
}

export function voltearV(g: Geometria): Geometria {
  const r = g.recorte ?? COMPLETO;
  return { ...g, espejoV: !g.espejoV, angulo: -g.angulo, recorte: [r[0], 1 - r[1], r[2], r[3]] };
}
