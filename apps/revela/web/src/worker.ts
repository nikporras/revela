// Web Worker del motor: aquí corre el wasm de Revela, fuera del hilo de la interfaz.
import init, { Revela, memoryBytes } from './wasm/revela_core.js';
import type { Peticion, Respuesta } from './lib/protocolo';

let motor: Revela | undefined;
let listo: Promise<number> | undefined;

function iniciar(): Promise<number> {
  listo ??= (async () => {
    const t0 = performance.now();
    await init();
    motor = new Revela();
    return performance.now() - t0;
  })();
  return listo;
}

function enviar(r: Respuesta, transfer: Transferable[] = []) {
  (self as unknown as Worker).postMessage(r, transfer);
}

self.onmessage = async (e: MessageEvent<Peticion>) => {
  const { id, op, args } = e.data;
  try {
    const msInicio = await iniciar();
    const m = motor!;
    const t0 = performance.now();
    let result: unknown;
    let transfer: Transferable[] = [];
    switch (op) {
      case 'iniciar':
        result = { ms: msInicio };
        break;
      case 'abrir': {
        const [nombre, bytes, maxMp, lado] = args as Peticion<'abrir'>['args'];
        result = m.open(nombre, new Uint8Array(bytes), maxMp, lado);
        break;
      }
      case 'previa': {
        const [lado] = args as Peticion<'previa'>['args'];
        const p = m.preview(lado);
        result = p;
        transfer = [p.pixels.buffer];
        break;
      }
      case 'exportar': {
        const [formato, calidad, conservar] = args as Peticion<'exportar'>['args'];
        const bytes = m.export(formato, calidad, conservar);
        result = bytes;
        transfer = [bytes.buffer];
        break;
      }
      case 'memoria':
        result = memoryBytes();
        break;
      default:
        throw new Error(`operación desconocida: ${String(op)}`);
    }
    enviar({ id, ok: true, result, ms: performance.now() - t0, memoria: memoryBytes() }, transfer);
  } catch (err) {
    // Un pánico o falta de memoria en Rust deja el wasm inservible: la interfaz debe crear otro
    // worker (`fatal`).
    const fatal = err instanceof WebAssembly.RuntimeError;
    enviar({ id, ok: false, error: err instanceof Error ? err.message : String(err), fatal });
  }
};
