// La receta de edición: lo que el usuario ha tocado. Es pequeña y se guarda entera en cada paso,
// así que deshacer/rehacer y los lotes salen gratis. `aCameraRaw` la convierte en los ajustes del
// Camera Raw de PhotoCraft que usan la previa (GPU) y la exportación (motor).
import TONO from './tono.json';
import type { Filtro } from './filtros';

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

export type Punto = [number, number];
export type Canal = 'master' | 'red' | 'green' | 'blue';

export interface Receta {
  sliders: Record<IdSlider, number>;
  /** Curvas de puntos (0..255). Vacía o de dos puntos en la diagonal = sin curva. */
  curvas: Record<Canal, Punto[]>;
  filtro: { id: string; intensidad: number } | null;
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
  };
}

export function esCurvaNeutra(c: Punto[]): boolean {
  return c.length < 2 || c.every(([x, y]) => Math.abs(x - y) < 1e-3);
}

export function esVacia(r: Receta): boolean {
  return Object.values(r.sliders).every((v) => v === 0) && Object.values(r.curvas).every(esCurvaNeutra) && !r.filtro;
}

type Ajustes = Record<string, unknown>;

const ESCALARES = ['exposure', 'contrast', 'temperature', 'tint', 'saturation', 'vibrance', 'curveShadows', 'curveDarks', 'curveLights', 'curveHighlights'];
const LIMITE: Record<string, number> = { exposure: 5 };
const CLAVE_CURVA: Record<Canal, string> = { master: 'pointCurve', red: 'pointCurveRed', green: 'pointCurveGreen', blue: 'pointCurveBlue' };

/**
 * Los ajustes de Camera Raw para la receta: los del filtro elegido escalados por su intensidad,
 * más los sliders del usuario encima. Las curvas del usuario mandan; si no hay, las del filtro.
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
  for (const canal of Object.keys(CLAVE_CURVA) as Canal[]) {
    if (!esCurvaNeutra(r.curvas[canal])) out[CLAVE_CURVA[canal]] = r.curvas[canal];
  }
  // Quita los ceros: una receta vacía es "{}".
  for (const [k, v] of Object.entries(out)) if (v === 0) delete out[k];
  return out;
}
