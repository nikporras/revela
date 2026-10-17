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

import type { ParametrosGpu, PlanoGpu } from './gpu';

export type FormatoExport = 'jpg' | 'png';

export interface Peticiones {
  iniciar: { args: []; result: { ms: number } };
  abrir: { args: [nombre: string, bytes: ArrayBuffer, maxMegapixeles: number, ladoPrevia: number]; result: InfoFoto };
  previa: { args: [ladoMax: number]; result: Previa };
  /** La previa con ajustes y geometría aplicados por el motor (sin WebGL2, y para la autoprueba). */
  previaMotor: { args: [ladoMax: number, ajustes: string, geometria: string]; result: Previa };
  /** Parámetros del shader para unos ajustes (JSON de Camera Raw). */
  gpuParams: { args: [ajustes: string]; result: ParametrosGpu };
  /** Parámetros del shader y encuadre para una foto de `ancho`×`alto`, en un solo viaje. */
  previaGpu: { args: [ajustes: string, geometria: string, ancho: number, alto: number]; result: { params: ParametrosGpu; plano: PlanoGpu } };
  /** Ángulo que deja horizontal o vertical una línea (regla de "Enderezar capa" del motor). */
  anguloLinea: { args: [x0: number, y0: number, x1: number, y1: number]; result: number };
  /**
   * Las filas `fila0..fila1` de una ventana (fracciones) del encuadre a resolución completa, sin
   * ajustes: el zoom nítido, por franjas. `ancho`×`alto` es el tamaño de la ventana entera.
   */
  detalle: {
    args: [geometria: string, ventana: [number, number, number, number], ancho: number, alto: number, fila0: number, fila1: number];
    result: { ancho: number; alto: number; fila0: number; pixels: Uint8ClampedArray };
  };
  tamanoExport: { args: [geometria: string]; result: [number, number] };
  exportar: { args: [formato: FormatoExport, calidad: number, conservarUbicacion: boolean, ajustes: string, geometria: string]; result: Uint8Array };
  memoria: { args: []; result: number };
}

export type Op = keyof Peticiones;

export interface Peticion<K extends Op = Op> {
  id: number;
  op: K;
  args: Peticiones[K]['args'];
}

export type Respuesta =
  | { id: number; progreso: number }
  | { id: number; ok: true; result: unknown; ms: number; memoria: number }
  | { id: number; ok: false; error: string; fatal: boolean };
