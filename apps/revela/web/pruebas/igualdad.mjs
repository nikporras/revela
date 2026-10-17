// Prueba de igualdad previa (GPU) / exportación (motor) en Chromium headless.
//   node pruebas/igualdad.mjs      (desde apps/revela/web, con el wasm ya compilado)
// Sale con código 1 si alguna diferencia pasa de 1/255 o si la GPU aplana los degradados: el
// workflow de despliegue no publica nada en ese caso.
import { createServer } from 'vite';
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';

const raiz = fileURLToPath(new URL('..', import.meta.url));
const servidor = await createServer({ root: raiz, logLevel: 'error', server: { port: 0, host: '127.0.0.1' } });
await servidor.listen();
const { port } = servidor.httpServer.address();
const navegador = await chromium.launch({ args: ['--enable-unsafe-swiftshader', '--use-angle=swiftshader', '--ignore-gpu-blocklist'] });
let codigo = 1;
try {
  const pagina = await navegador.newPage();
  pagina.on('pageerror', (e) => console.error('[error en la página]', e.message));
  await pagina.goto(`http://127.0.0.1:${port}/revela/pruebas/igualdad.html`);
  await pagina.waitForFunction(() => window.__igualdad, null, { timeout: 600_000, polling: 500 });
  const inf = await pagina.evaluate(() => window.__igualdad);
  if (inf.error) throw new Error(inf.error);
  console.log(`GPU: ${inf.gpu}`);
  console.log(`${inf.resultados.length} comparaciones en ${(inf.ms / 1000).toFixed(1)} s`);
  const malos = inf.resultados.filter((r) => r.max > 1 || (r.niveles && r.niveles[0] < r.niveles[1] * 0.98));
  for (const r of malos.slice(0, 30)) console.log(`  ✗ ${r.imagen} · ${r.receta}: máx ${r.max}/255, ${r.fuera} px, niveles ${r.niveles ?? '-'}`);
  console.log(`Peor: ${inf.peor.imagen} · ${inf.peor.receta}: ${inf.peor.max}/255`);
  console.log(inf.ok ? 'IGUALDAD OK (≤ 1/255, sin bandas)' : 'IGUALDAD FALLA');
  codigo = inf.ok ? 0 : 1;
} catch (e) {
  console.error(e);
} finally {
  await navegador.close();
  await servidor.close();
}
process.exit(codigo);
