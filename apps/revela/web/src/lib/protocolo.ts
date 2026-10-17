// Mensajes entre la interfaz y el Web Worker del motor.

export interface InfoFoto {
  width: number;
  height: number;
  originalWidth: number;
  originalHeight: number;
  /** Formato detectado por el contenido ("jpeg", "heif", "png"…). */
  format: string;
  warnings: string[];
  /** Fecha de captura del EXIF como "AAAAMMDD_HHMMSS", o null. */
  takenAt: string | null;
}

export interface Previa {
  width: number;
  height: number;
  pixels: Uint8ClampedArray;
}

import type { ParametrosGpu } from './gpu';

export type FormatoExport = 'jpg' | 'png';

export interface Peticiones {
  iniciar: { args: []; result: { ms: number } };
  abrir: { args: [nombre: string, bytes: ArrayBuffer, maxMegapixeles: number, ladoPrevia: number]; result: InfoFoto };
  previa: { args: [ladoMax: number]; result: Previa };
  /** La previa con los ajustes aplicados por el motor (sin WebGL2, y para la autoprueba). */
  previaMotor: { args: [ladoMax: number, ajustes: string]; result: Previa };
  /** Parámetros del shader para unos ajustes (JSON de Camera Raw). */
  gpuParams: { args: [ajustes: string]; result: ParametrosGpu };
  exportar: { args: [formato: FormatoExport, calidad: number, conservarUbicacion: boolean, ajustes: string]; result: Uint8Array };
  memoria: { args: []; result: number };
}

export type Op = keyof Peticiones;

export interface Peticion<K extends Op = Op> {
  id: number;
  op: K;
  args: Peticiones[K]['args'];
}

export type Respuesta =
  | { id: number; ok: true; result: unknown; ms: number; memoria: number }
  | { id: number; ok: false; error: string; fatal: boolean };
