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

/** Un worker con su propia copia del motor. */
export class Motor {
  private worker: Worker | undefined;
  private siguiente = 1;
  private pendientes = new Map<number, Pendiente>();

  constructor(private nombre = 'motor-revela') {}

  pedir<K extends Op>(op: K, ...args: Peticiones[K]['args']): Promise<Resultado<Peticiones[K]['result']>> {
    this.worker ??= this.crearWorker();
    const id = this.siguiente++;
    const transfer = args.filter((a): a is ArrayBuffer => a instanceof ArrayBuffer);
    const msg: Peticion<K> = { id, op, args };
    return new Promise((resolve, reject) => {
      this.pendientes.set(id, { resolve: resolve as Pendiente['resolve'], reject });
      this.worker!.postMessage(msg, transfer);
    });
  }

  /** Descarta el worker (por ejemplo, si el wasm se quedó sin memoria). El próximo pedido crea otro. */
  reiniciar() {
    this.worker?.terminate();
    this.worker = undefined;
  }

  private crearWorker(): Worker {
    const w = new Worker(new URL('../worker.ts', import.meta.url), { type: 'module', name: this.nombre });
    w.onmessage = (e: MessageEvent<Respuesta>) => {
      const r = e.data;
      const p = this.pendientes.get(r.id);
      if (!p) return;
      this.pendientes.delete(r.id);
      if (r.ok) {
        p.resolve({ valor: r.result, ms: r.ms, memoria: r.memoria });
      } else {
        if (r.fatal) this.reiniciar();
        p.reject(new Error(r.error));
      }
    };
    w.onerror = (e) => {
      e.preventDefault();
      const msg = e.message || 'el motor dejó de responder';
      for (const p of this.pendientes.values()) p.reject(new Error(msg));
      this.pendientes.clear();
      this.reiniciar();
    };
    return w;
  }
}

/** El motor de la app (la foto que se está editando). */
export const motor = new Motor();

export function pedir<K extends Op>(op: K, ...args: Peticiones[K]['args']): Promise<Resultado<Peticiones[K]['result']>> {
  return motor.pedir(op, ...args);
}

/**
 * Pide siempre con los últimos argumentos, sin acumular: mientras una petición está en curso solo
 * se guarda la más reciente (un slider que se arrastra genera muchas).
 */
export function ultimaGana<A extends unknown[], R>(f: (...a: A) => Promise<R>, alTerminar: (r: R) => void, alFallar: (e: Error) => void) {
  let enCurso = false;
  let pendiente: A | null = null;
  const correr = async (a: A) => {
    enCurso = true;
    try {
      alTerminar(await f(...a));
    } catch (e) {
      alFallar(e as Error);
    }
    enCurso = false;
    if (pendiente) {
      const p = pendiente;
      pendiente = null;
      void correr(p);
    }
  };
  return (...a: A) => {
    if (enCurso) pendiente = a;
    else void correr(a);
  };
}
