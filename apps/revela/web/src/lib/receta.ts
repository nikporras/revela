// La receta de edición: lo que el usuario ha tocado. Es pequeña y se guarda entera en cada paso,
// así que deshacer/rehacer y los lotes salen gratis. `aCameraRaw` la convierte en los ajustes del
// Camera Raw de PhotoCraft que usan la previa (GPU) y la exportación (motor).
import TONO from './tono.json';
import type { Filtro } from './filtros';
import { esGeometriaVacia, geometriaVacia, type Geometria } from './recorte';

export type IdSlider = 'exposicion' | 'contraste' | 'luces' | 'sombras' | 'temperatura' | 'tinte' | 'saturacion' | 'vibrancia';

export interface DefSlider {
  id: IdSlider;
  nombre: string;
  min: number;
  max: number;
  paso: number;
  /** Cómo se muestra el valor. */
  formato?: (v: number) => string;
}

export const SLIDERS: DefSlider[] = [
  { id: 'exposicion', nombre: 'Exposición', min: -3, max: 3, paso: 0.05, formato: (v) => `${v > 0 ? '+' : ''}${v.toFixed(2)} EV` },
  { id: 'contraste', nombre: 'Contraste', min: -100, max: 100, paso: 1 },
  { id: 'luces', nombre: 'Luces', min: -100, max: 100, paso: 1 },
  { id: 'sombras', nombre: 'Sombras', min: -100, max: 100, paso: 1 },
  { id: 'temperatura', nombre: 'Temperatura', min: -100, max: 100, paso: 1 },
  { id: 'tinte', nombre: 'Tinte', min: -100, max: 100, paso: 1 },
  { id: 'saturacion', nombre: 'Saturación', min: -100, max: 100, paso: 1 },
  { id: 'vibrancia', nombre: 'Vibrancia', min: -100, max: 100, paso: 1 },
];

/** Las secciones de Editar donde va cada slider. */
export const LUZ: IdSlider[] = ['exposicion', 'contraste', 'luces', 'sombras'];
export const COLOR: IdSlider[] = ['temperatura', 'tinte', 'vibrancia', 'saturacion'];

/** Las 8 bandas del HSL del motor (centro en grados), en su orden. */
export const BANDAS = [
  { nombre: 'Rojo', tono: 0 },
  { nombre: 'Naranja', tono: 30 },
  { nombre: 'Amarillo', tono: 60 },
  { nombre: 'Verde', tono: 120 },
  { nombre: 'Aguamarina', tono: 180 },
  { nombre: 'Azul', tono: 240 },
  { nombre: 'Morado', tono: 270 },
  { nombre: 'Magenta', tono: 300 },
];

export type CanalHsl = 'tono' | 'saturacion' | 'luminancia';

/**
 * Pesos de las 8 bandas para un tono (grados): triángulos entre centros vecinos. Es
 * `band_weights` del motor (`crates/algo/src/camera_raw.rs`).
 */
export function pesosBandas(h: number): number[] {
  const n = BANDAS.length;
  const rem = (a: number) => ((a % 360) + 360) % 360;
  return BANDAS.map((b, k) => {
    const c = b.tono;
    const antes = BANDAS[(k + n - 1) % n].tono;
    const despues = BANDAS[(k + 1) % n].tono;
    const dif = rem(h - c + 540) - 180;
    const ancho = dif >= 0 ? rem(despues - c) : rem(c - antes);
    return Math.max(0, 1 - Math.abs(dif) / Math.max(ancho, 1e-3));
  });
}

/**
 * Cómo llamaría una persona a un color que el HSL clasifica en la banda `banda` (para decir
 * por qué un marrón cuenta como naranja): `null` si el nombre de la banda ya basta. `s` y `v`
 * son la saturación y el brillo de HSV (0..1).
 */
export function matiz(banda: number, s: number, v: number): string | null {
  const nombre = BANDAS[banda].nombre;
  const calido = ['Rojo', 'Naranja', 'Amarillo'].includes(nombre);
  const frio = ['Aguamarina', 'Azul', 'Morado'].includes(nombre);
  if (v < 0.2) return frio ? 'sombra fría' : calido ? 'sombra cálida' : 'sombra';
  if (s < 0.3) {
    if (calido) return v > 0.7 ? 'beige' : 'gris cálido';
    if (frio) return 'gris frío';
    return 'apagado';
  }
  if (calido && nombre !== 'Amarillo' && v < 0.55) return 'marrón';
  if (nombre === 'Amarillo' && v < 0.55) return 'oliva';
  return null;
}

/** Tono (grados), saturación y brillo (0..1, los de HSV como el motor) de un color 0..1. */
export function tonoYSaturacion(r: number, g: number, b: number): [number, number, number] {
  const mx = Math.max(r, g, b);
  const mn = Math.min(r, g, b);
  const d = mx - mn;
  let h = 0;
  if (d > 1e-9) {
    if (mx === r) h = 60 * ((((g - b) / d) % 6) + 6) % 360;
    else if (mx === g) h = 60 * ((b - r) / d + 2);
    else h = 60 * ((r - g) / d + 4);
  }
  return [h, mx > 1e-9 ? d / mx : 0, mx];
}

/** Una rueda de la gradación: tono 0..360, saturación 0..100, luminancia −100..100. */
export interface Rueda {
  tono: number;
  sat: number;
  lum: number;
}

export type IdRueda = 'sombras' | 'medios' | 'luces' | 'global';

export interface Gradacion {
  ruedas: Record<IdRueda, Rueda>;
  /** 0..100 (50 por defecto, como el motor). */
  mezcla: number;
  /** −100..100. */
  balance: number;
}

export const MEZCLA_DEFECTO = 50;

export type Punto = [number, number];
export type Canal = 'master' | 'red' | 'green' | 'blue';

export interface Receta {
  sliders: Record<IdSlider, number>;
  /** Curvas de puntos (0..255). Vacía o de dos puntos en la diagonal = sin curva. */
  curvas: Record<Canal, Punto[]>;
  filtro: { id: string; intensidad: number } | null;
  geometria: Geometria;
  /** Mezcla de color: 8 valores por canal (−100..100). */
  hsl: Record<CanalHsl, number[]>;
  gradacion: Gradacion;
}

export const DIAGONAL: Punto[] = [
  [0, 0],
  [255, 255],
];

export function recetaVacia(): Receta {
  return {
    sliders: { exposicion: 0, contraste: 0, luces: 0, sombras: 0, temperatura: 0, tinte: 0, saturacion: 0, vibrancia: 0 },
    curvas: { master: DIAGONAL, red: DIAGONAL, green: DIAGONAL, blue: DIAGONAL },
    filtro: null,
    geometria: geometriaVacia(),
    hsl: { tono: ceros(), saturacion: ceros(), luminancia: ceros() },
    gradacion: gradacionVacia(),
  };
}

const ceros = () => Array.from({ length: 8 }, () => 0);

export const ruedaVacia = (): Rueda => ({ tono: 0, sat: 0, lum: 0 });

export function gradacionVacia(): Gradacion {
  return { ruedas: { sombras: ruedaVacia(), medios: ruedaVacia(), luces: ruedaVacia(), global: ruedaVacia() }, mezcla: MEZCLA_DEFECTO, balance: 0 };
}

export const esRuedaNeutra = (r: Rueda) => r.sat === 0 && r.lum === 0;

export function esHslNeutro(h: Receta['hsl']): boolean {
  return Object.values(h).every((v) => v.every((x) => x === 0));
}

export function esGradacionNeutra(g: Gradacion): boolean {
  return Object.values(g.ruedas).every(esRuedaNeutra) && g.mezcla === MEZCLA_DEFECTO && g.balance === 0;
}

export function esCurvaNeutra(c: Punto[]): boolean {
  return c.length < 2 || c.every(([x, y]) => Math.abs(x - y) < 1e-3);
}

export function esVacia(r: Receta): boolean {
  return Object.values(r.sliders).every((v) => v === 0) && Object.values(r.curvas).every(esCurvaNeutra) && !r.filtro && esGeometriaVacia(r.geometria) && esHslNeutro(r.hsl) && esGradacionNeutra(r.gradacion);
}

type Ajustes = Record<string, unknown>;

const ESCALARES = ['exposure', 'contrast', 'temperature', 'tint', 'saturation', 'vibrance', 'curveShadows', 'curveDarks', 'curveLights', 'curveHighlights'];
const LIMITE: Record<string, number> = { exposure: 5 };
const CLAVE_HSL: Record<CanalHsl, string> = { tono: 'hslHue', saturacion: 'hslSat', luminancia: 'hslLum' };
const CLAVE_RUEDA: Record<IdRueda, string> = { sombras: 'gradeShadows', medios: 'gradeMidtones', luces: 'gradeHighlights', global: 'gradeGlobal' };
const CLAVE_CURVA: Record<Canal, string> = { master: 'pointCurve', red: 'pointCurveRed', green: 'pointCurveGreen', blue: 'pointCurveBlue' };

/**
 * Los ajustes de Camera Raw para la receta: los del filtro elegido escalados por su intensidad,
 * más los sliders del usuario encima. Las curvas del usuario mandan; si no hay, las del filtro.
 * El HSL se suma banda a banda; las ruedas de gradación se suman como colores (vectores de tono
 * y saturación), y la mezcla del usuario manda si la ha cambiado.
 */
export function aCameraRaw(r: Receta, filtros: Filtro[]): Ajustes {
  const out: Ajustes = {};
  const sumar = (clave: string, v: number) => {
    const lim = LIMITE[clave] ?? 100;
    out[clave] = Math.max(-lim, Math.min(lim, ((out[clave] as number) ?? 0) + v));
  };
  const f = r.filtro ? filtros.find((x) => x.id === r.filtro!.id) : undefined;
  if (f && r.filtro) {
    const k = r.filtro.intensidad;
    for (const [clave, v] of Object.entries(f.ajustes)) {
      if (ESCALARES.includes(clave)) sumar(clave, (v as number) * k);
      else if (clave.startsWith('hsl')) out[clave] = (v as number[]).map((x) => x * k);
      else if (clave.startsWith('grade') && typeof v === 'object' && v) {
        const w = v as { hue: number; sat: number; lum: number };
        out[clave] = { hue: w.hue, sat: w.sat * k, lum: w.lum * k };
      } else if (clave === 'gradeBalance') out[clave] = (v as number) * k;
      else out[clave] = v; // gradeBlending, curveSplits
    }
  }
  const s = r.sliders;
  sumar('exposure', s.exposicion);
  sumar('contrast', s.contraste);
  sumar('temperature', s.temperatura);
  sumar('tint', s.tinte);
  sumar('saturation', s.saturacion);
  sumar('vibrance', s.vibrancia);
  for (const [clave, k] of Object.entries(TONO.sombras)) sumar(clave, s.sombras * k);
  for (const [clave, k] of Object.entries(TONO.luces)) sumar(clave, s.luces * k);
  const lim = (v: number, a: number) => Math.max(-a, Math.min(a, v));
  for (const canal of Object.keys(CLAVE_HSL) as CanalHsl[]) {
    const u = r.hsl[canal];
    if (u.every((x) => x === 0)) continue;
    const f = (out[CLAVE_HSL[canal]] as number[] | undefined) ?? ceros();
    out[CLAVE_HSL[canal]] = f.map((x, i) => lim(x + u[i], 100));
  }
  for (const id of Object.keys(CLAVE_RUEDA) as IdRueda[]) {
    const u = r.gradacion.ruedas[id];
    if (esRuedaNeutra(u)) continue;
    const f = (out[CLAVE_RUEDA[id]] as { hue: number; sat: number; lum: number } | undefined) ?? { hue: 0, sat: 0, lum: 0 };
    out[CLAVE_RUEDA[id]] = sumarRuedas(f, u);
  }
  const g = r.gradacion;
  if (g.mezcla !== MEZCLA_DEFECTO) out.gradeBlending = g.mezcla;
  if (g.balance !== 0) out.gradeBalance = lim(((out.gradeBalance as number) ?? 0) + g.balance, 100);
  for (const canal of Object.keys(CLAVE_CURVA) as Canal[]) {
    if (!esCurvaNeutra(r.curvas[canal])) out[CLAVE_CURVA[canal]] = r.curvas[canal];
  }
  // Quita los ceros: una receta vacía es "{}".
  // (La mezcla no: su valor por defecto es 50.)
  for (const [k, v] of Object.entries(out)) if (v === 0 && k !== 'gradeBlending') delete out[k];
  return out;
}

/** Dos tintes de gradación sumados como colores: la saturación es la longitud del vector. */
export function sumarRuedas(f: { hue: number; sat: number; lum: number }, u: Rueda): { hue: number; sat: number; lum: number } {
  const rad = Math.PI / 180;
  const x = f.sat * Math.cos(f.hue * rad) + u.sat * Math.cos(u.tono * rad);
  const y = f.sat * Math.sin(f.hue * rad) + u.sat * Math.sin(u.tono * rad);
  const sat = Math.min(100, Math.hypot(x, y));
  const hue = sat > 1e-6 ? ((Math.atan2(y, x) / rad) % 360 + 360) % 360 : u.tono;
  return { hue: Math.round(hue * 100) / 100, sat: Math.round(sat * 100) / 100, lum: Math.max(-100, Math.min(100, f.lum + u.lum)) };
}
