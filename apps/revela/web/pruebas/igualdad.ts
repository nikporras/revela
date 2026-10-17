// Página de prueba (no se despliega): compara la previa de la GPU con el motor.
// La abre `igualdad.mjs` en Chromium headless; deja el informe en `window.__igualdad`.
import init, { Revela, gpuParams } from '../src/wasm/revela_core.js';
import { Revelador } from '../src/lib/gpu';
import { probar, recetasDePrueba, type MotorDePrueba } from '../src/lib/autoprueba';
import { recetasDeFiltros } from '../src/lib/filtros';

const estado = document.getElementById('estado')!;
(async () => {
  try {
    await init();
    const r = new Revela();
    const motor: MotorDePrueba = {
      abrir: async (nombre, bytes) => void r.open(nombre, bytes, 100, 512),
      previa: async (lado) => r.preview(lado),
      previaMotor: async (lado, json) => r.developPreview(lado, json),
      gpuParams: async (json) => gpuParams(json),
    };
    const rev = new Revelador(document.getElementById('gpu') as HTMLCanvasElement);
    const informe = await probar(motor, rev, recetasDePrueba(recetasDeFiltros()), (t) => (estado.textContent = t));
    (window as unknown as { __igualdad: unknown }).__igualdad = { ...informe, gpu: rev.descripcion() };
    estado.textContent = informe.ok ? 'OK' : 'FALLO';
  } catch (e) {
    (window as unknown as { __igualdad: unknown }).__igualdad = { error: String(e) };
    estado.textContent = String(e);
  }
})();
