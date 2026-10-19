// Prueba de igualdad previa (GPU) / exportación (motor). La usan el CI (`pruebas/igualdad.mjs`,
// bloquea el despliegue si falla) y la app en el propio teléfono (ⓘ › Diagnóstico).
//
// Para cada imagen de prueba y cada receta, dibuja la copia de previa con el shader y la revela
// con el motor (`camera_raw::develop`, lo mismo que la exportación) y compara:
// - diferencia máxima por canal ≤ 1/255;
// - bandas: en los degradados suaves, la GPU debe dar tantos niveles distintos como el motor (una
//   GPU con poca precisión los aplasta en escalones).
import type { ParametrosGpu, PlanoGpu, Revelador } from './gpu';
import type { Previa } from './protocolo';

export interface MotorDePrueba {
  abrir(nombre: string, bytes: Uint8Array): Promise<void>;
  previa(lado: number): Promise<Previa>;
  previaMotor(lado: number, json: string, geometria: string): Promise<Previa>;
  previaGpu(json: string, geometria: string, ancho: number, alto: number): Promise<{ params: ParametrosGpu; plano: PlanoGpu }>;
}

export interface Receta {
  nombre: string;
  json: string;
}

export interface Resultado {
  imagen: string;
  receta: string;
  /** Encuadre (vacío: la foto tal cual). */
  geometria?: string;
  max: number;
  /** Píxeles donde uno ve foto y el otro transparente (solo en el borde de la foto girada). */
  borde?: number;
  /** Píxeles con más de 1/255 de diferencia. */
  fuera: number;
  /** Niveles distintos a lo largo del degradado: GPU / motor (solo en las imágenes suaves). */
  niveles?: [number, number];
}

export interface Informe {
  ok: boolean;
  peor: Resultado;
  bandas: boolean;
  resultados: Resultado[];
  ms: number;
}

const LADO = 512;
export const TOLERANCIA = 1;
/** Píxeles de borde (dentro/fuera de la foto girada) que pueden diferir por redondeo. */
export const MAX_BORDE = 4;

/** Recetas que tocan cada etapa, solas y juntas, en sus extremos. */
export function recetasDePrueba(extra: Receta[] = []): Receta[] {
  const curva = '"pointCurve":[[0,12],[64,40],[128,140],[192,215],[255,245]]';
  const base: Receta[] = [
    { nombre: 'sin ajustes', json: '{}' },
    { nombre: 'exposición +2', json: '{"exposure":2}' },
    { nombre: 'exposición −2', json: '{"exposure":-2}' },
    { nombre: 'temperatura +100 / tinte −100', json: '{"temperature":100,"tint":-100}' },
    { nombre: 'temperatura −100 / tinte +100', json: '{"temperature":-100,"tint":100}' },
    { nombre: 'contraste +100', json: '{"contrast":100}' },
    { nombre: 'contraste −100', json: '{"contrast":-100}' },
    { nombre: 'saturación +100', json: '{"saturation":100}' },
    { nombre: 'saturación −100', json: '{"saturation":-100}' },
    { nombre: 'vibrancia +100', json: '{"vibrance":100}' },
    { nombre: 'luces/sombras globales', json: '{"curveHighlights":-100,"curveLights":-50,"curveDarks":50,"curveShadows":100}' },
    { nombre: 'curva maestra', json: `{${curva}}` },
    { nombre: 'curvas RGB', json: '{"pointCurveRed":[[0,0],[128,150],[255,255]],"pointCurveGreen":[[0,10],[255,240]],"pointCurveBlue":[[0,30],[128,110],[255,230]]}' },
    { nombre: 'HSL', json: '{"hslHue":[30,-20,15,40,-30,20,-10,25],"hslSat":[50,-40,30,-60,40,60,-20,10],"hslLum":[20,-30,10,-40,30,-20,40,-10]}' },
    { nombre: 'gradación', json: '{"gradeShadows":{"hue":195,"sat":80,"lum":-30},"gradeMidtones":{"hue":300,"sat":40,"lum":10},"gradeHighlights":{"hue":35,"sat":70,"lum":20},"gradeGlobal":{"hue":120,"sat":20,"lum":0},"gradeBlending":80,"gradeBalance":-40}' },
    { nombre: 'viñeta −70', json: '{"vignetteAmount":-70}' },
    { nombre: 'viñeta +60 redonda', json: '{"vignetteAmount":60,"vignetteMidpoint":20,"vignetteRoundness":80,"vignetteFeather":90}' },
    { nombre: 'viñeta −90 rectangular con luces', json: '{"vignetteAmount":-90,"vignetteMidpoint":70,"vignetteRoundness":-60,"vignetteFeather":10,"vignetteHighlights":80,"exposure":1}' },
    { nombre: 'viñeta prioridad color', json: '{"vignetteAmount":-60,"vignetteStyle":"colorPriority","saturation":40}' },
    { nombre: 'viñeta pintura', json: '{"vignetteAmount":50,"vignetteStyle":"paintOverlay"}' },
    {
      nombre: 'todo fuerte',
      json: `{"temperature":40,"tint":-20,"exposure":0.7,"contrast":60,"vibrance":50,"saturation":-20,"curveShadows":60,"curveHighlights":-60,${curva},"hslSat":[0,30,0,-40,0,50,0,0],"gradeShadows":{"hue":195,"sat":40,"lum":0},"gradeHighlights":{"hue":35,"sat":30,"lum":0}}`,
    },
  ];
  // Combinaciones al azar (semilla fija: siempre las mismas).
  let s = 12345;
  const azar = () => ((s = (s * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
  const v = (lo: number, hi: number) => Math.round((lo + (hi - lo) * azar()) * 100) / 100;
  for (let i = 0; i < 12; i++) {
    base.push({
      nombre: `azar ${i + 1}`,
      json: JSON.stringify({
        temperature: v(-100, 100),
        tint: v(-100, 100),
        exposure: v(-2, 2),
        contrast: v(-100, 100),
        vibrance: v(-100, 100),
        saturation: v(-100, 100),
        curveShadows: v(-100, 100),
        curveDarks: v(-100, 100),
        curveLights: v(-100, 100),
        curveHighlights: v(-100, 100),
        hslHue: Array.from({ length: 8 }, () => v(-100, 100)),
        hslSat: Array.from({ length: 8 }, () => v(-100, 100)),
        hslLum: Array.from({ length: 8 }, () => v(-100, 100)),
        gradeShadows: { hue: v(0, 360), sat: v(0, 100), lum: v(-50, 50) },
        gradeHighlights: { hue: v(0, 360), sat: v(0, 100), lum: v(-50, 50) },
        // La mitad, con viñeta.
        ...(i % 2 ? { vignetteAmount: v(-100, 100), vignetteMidpoint: v(0, 100), vignetteRoundness: v(-100, 100), vignetteFeather: v(0, 100), vignetteHighlights: v(0, 100) } : {}),
      }),
    });
  }
  return base.concat(extra);
}

/** Encuadres de prueba: giros, volteos, enderezados (con y sin esquinas vacías) y recortes. */
export const GEOMETRIAS: { nombre: string; json: string }[] = [
  { nombre: 'girar 90°', json: '{"giro":1}' },
  { nombre: 'girar 270° + voltear H', json: '{"giro":3,"espejoH":true}' },
  { nombre: 'voltear V + recorte', json: '{"espejoV":true,"recorte":[0.4,0.55,0.5,0.6]}' },
  { nombre: 'enderezar 12,5° recortado', json: '{"angulo":12.5,"recorte":[0.5,0.5,0.62,0.62]}' },
  { nombre: 'enderezar −30° sin recorte', json: '{"angulo":-30}' },
  { nombre: 'todo junto', json: '{"giro":1,"espejoV":true,"angulo":-7.3,"recorte":[0.45,0.5,0.6,0.55]}' },
];

/** Recetas de color que se cruzan con los encuadres (no todas: sería muy lento en el teléfono). */
function recetasConGeometria(todas: Receta[]): Receta[] {
  const nombres = ['sin ajustes', 'todo fuerte', 'azar 3', 'azar 9', 'viñeta −90 rectangular con luces'];
  const filtro = todas.find((r) => r.nombre.startsWith('filtro '));
  return todas.filter((r) => nombres.includes(r.nombre)).concat(filtro ? [filtro] : []);
}

/** Imágenes sintéticas en PNG: degradados suaves (cielos), todos los colores, ruido y grises. */
export async function imagenesDePrueba(): Promise<{ nombre: string; suave: boolean; bytes: Uint8Array }[]> {
  const crear = async (nombre: string, w: number, h: number, suave: boolean, f: (x: number, y: number) => [number, number, number]) => {
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    const ctx = c.getContext('2d');
    if (!ctx) throw new Error('sin canvas 2D');
    const img = ctx.createImageData(w, h);
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const [r, g, b] = f(x, y);
        const i = (y * w + x) * 4;
        img.data[i] = r;
        img.data[i + 1] = g;
        img.data[i + 2] = b;
        img.data[i + 3] = 255;
      }
    ctx.putImageData(img, 0, 0);
    const blob = await new Promise<Blob | null>((ok) => c.toBlob(ok, 'image/png'));
    if (!blob) throw new Error('no se pudo crear la imagen de prueba');
    return { nombre, suave, bytes: new Uint8Array(await blob.arrayBuffer()) };
  };
  const hsv = (h: number, s: number, v: number): [number, number, number] => {
    const k = (n: number) => (n + h / 60) % 6;
    const f = (n: number) => v - v * s * Math.max(0, Math.min(k(n), 4 - k(n), 1));
    return [Math.round(f(5) * 255), Math.round(f(3) * 255), Math.round(f(1) * 255)];
  };
  let s = 7;
  const azar = () => ((s = (s * 1103515245 + 12345) & 0x7fffffff) & 255);
  return [
    // Cielo: de azul medio a casi blanco, muy suave (pocos niveles por columna).
    await crear('cielo', LADO, LADO, true, (x, y) => {
      const t = y / (LADO - 1);
      const u = x / (LADO - 1);
      return [Math.round(70 + 120 * t + 8 * u), Math.round(120 + 95 * t + 6 * u), Math.round(190 + 50 * t)];
    }),
    await crear('rampa de grises', LADO, 64, true, (x) => {
      const v = Math.round((x / (LADO - 1)) * 255);
      return [v, v, v];
    }),
    await crear('todos los colores', LADO, LADO, false, (x, y) => hsv((x / LADO) * 360, y < LADO / 2 ? y / (LADO / 2) : 1, y < LADO / 2 ? 1 : 1 - (y - LADO / 2) / (LADO / 2))),
    await crear('ruido', 256, 256, false, () => [azar(), azar(), azar()]),
  ];
}

function niveles(px: Uint8Array | Uint8ClampedArray, w: number, h: number): number {
  // Niveles distintos de verde en la columna central: lo que se ve como bandas en un cielo.
  const vistos = new Set<number>();
  const x = w >> 1;
  for (let y = 0; y < h; y++) vistos.add(px[(y * w + x) * 4 + 1]);
  // Y a lo largo de la fila central (la rampa de grises es horizontal).
  const fila = new Set<number>();
  const yc = h >> 1;
  for (let i = 0; i < w; i++) fila.add(px[(yc * w + i) * 4 + 1]);
  return Math.max(vistos.size, fila.size);
}

export async function probar(motor: MotorDePrueba, rev: Revelador, recetas = recetasDePrueba(), progreso?: (txt: string) => void): Promise<Informe> {
  const t0 = performance.now();
  const resultados: Resultado[] = [];
  let bandas = false;
  for (const img of await imagenesDePrueba()) {
    await motor.abrir(`${img.nombre}.png`, img.bytes);
    const base = await motor.previa(LADO);
    rev.cargar(base.width, base.height, base.pixels);
    const casos: { r: Receta; g: { nombre: string; json: string } | null }[] = recetas.map((r) => ({ r, g: null }));
    for (const g of GEOMETRIAS) for (const r of recetasConGeometria(recetas)) casos.push({ r, g });
    for (const { r, g } of casos) {
      progreso?.(`${img.nombre}: ${r.nombre}${g ? ` · ${g.nombre}` : ''}`);
      const geo = g ? g.json : '{}';
      const { params, plano } = await motor.previaGpu(r.json, geo, base.width, base.height);
      rev.dibujar(params, g ? plano : null);
      const gpu = rev.leer();
      const ref = (await motor.previaMotor(LADO, r.json, geo)).pixels;
      let max = 0;
      let fuera = 0;
      let borde = 0;
      if (gpu.length !== ref.length) throw new Error(`tamaños distintos (${r.nombre}${g ? `, ${g.nombre}` : ''})`);
      for (let i = 0; i < gpu.length; i += 4) {
        const da = Math.abs(gpu[i + 3] - ref[i + 3]);
        if (da > TOLERANCIA) {
          // Un punto justo en el borde de la foto girada: uno lo ve dentro y el otro fuera.
          borde++;
          continue;
        }
        if (ref[i + 3] === 0) continue;
        const d = Math.max(Math.abs(gpu[i] - ref[i]), Math.abs(gpu[i + 1] - ref[i + 1]), Math.abs(gpu[i + 2] - ref[i + 2]));
        if (d > max) max = d;
        if (d > TOLERANCIA) fuera++;
      }
      const res: Resultado = { imagen: img.nombre, receta: r.nombre, max, fuera };
      if (g) {
        res.geometria = g.nombre;
        res.borde = borde;
        // Se admite un puñado de píxeles de borde por redondeo (nunca dentro de un recorte válido).
        if (borde > MAX_BORDE) res.max = Math.max(res.max, 255);
      }
      if (img.suave && !g) {
        res.niveles = [niveles(gpu, base.width, base.height), niveles(ref, base.width, base.height)];
        // Una GPU con poca precisión pierde niveles: se exige al menos el 98 % de los del motor.
        if (res.niveles[0] < res.niveles[1] * 0.98) bandas = true;
      }
      resultados.push(res);
    }
  }
  const peor = resultados.reduce((a, b) => (b.max > a.max ? b : a));
  return { ok: peor.max <= TOLERANCIA && !bandas, peor, bandas, resultados, ms: performance.now() - t0 };
}
