// Cliente del motor: la interfaz llama a estas funciones y el trabajo pesado ocurre en el worker.
import type { Op, Peticion, Peticiones, Respuesta } from './protocolo';

export interface Resultado<T> {
  valor: T;
  /** Tiempo dentro del worker, en milisegundos. */
  ms: number;
  /** Memoria del wasm después de la operación, en bytes (solo crece: es el pico). */
  memoria: number;
}

type Pendiente = { resolve: (r: Resultado<unknown>) => void; reject: (e: Error) => void };

let worker: Worker | undefined;
let siguiente = 1;
const pendientes = new Map<number, Pendiente>();

function crearWorker(): Worker {
  const w = new Worker(new URL('../worker.ts', import.meta.url), { type: 'module', name: 'motor-revela' });
  w.onmessage = (e: MessageEvent<Respuesta>) => {
    const r = e.data;
    const p = pendientes.get(r.id);
    if (!p) return;
    pendientes.delete(r.id);
    if (r.ok) {
      p.resolve({ valor: r.result, ms: r.ms, memoria: r.memoria });
    } else {
      if (r.fatal) reiniciar();
      p.reject(new Error(r.error));
    }
  };
  w.onerror = (e) => {
    e.preventDefault();
    const msg = e.message || 'el motor dejó de responder';
    for (const p of pendientes.values()) p.reject(new Error(msg));
    pendientes.clear();
    reiniciar();
  };
  return w;
}

/** Descarta el worker (por ejemplo, si el wasm se quedó sin memoria). El próximo pedido crea otro. */
export function reiniciar() {
  worker?.terminate();
  worker = undefined;
}

export function pedir<K extends Op>(op: K, ...args: Peticiones[K]['args']): Promise<Resultado<Peticiones[K]['result']>> {
  worker ??= crearWorker();
  const id = siguiente++;
  const transfer = args.filter((a): a is ArrayBuffer => a instanceof ArrayBuffer);
  const msg: Peticion<K> = { id, op, args };
  return new Promise((resolve, reject) => {
    pendientes.set(id, { resolve: resolve as Pendiente['resolve'], reject });
    worker!.postMessage(msg, transfer);
  });
}
