// Actualizaciones de la app instalada. El service worker nuevo se descarga solo, pero espera:
// la página avisa ("Nueva versión disponible") y se recarga cuando el usuario quiere, para no
// perder una edición a medias. También se puede buscar a mano (ⓘ › Buscar actualización).
import { registerSW } from 'virtual:pwa-register';

export type EstadoActualizacion = 'al-dia' | 'buscando' | 'disponible' | 'sin-conexion' | 'no-disponible';

let registro: ServiceWorkerRegistration | undefined;
let registrado = false;
const oyentes = new Set<(e: EstadoActualizacion) => void>();
let estado: EstadoActualizacion = 'serviceWorker' in navigator ? 'al-dia' : 'no-disponible';

function poner(e: EstadoActualizacion) {
  estado = e;
  for (const f of oyentes) f(e);
}

/** Registra el service worker y avisa de los cambios de estado a `f` (devuelve cómo dejar de oír). */
export function vigilarActualizaciones(f: (e: EstadoActualizacion) => void): () => void {
  oyentes.add(f);
  f(estado);
  if (!registrado && estado !== 'no-disponible') {
    registrado = true;
    registerSW({
      onNeedRefresh: () => poner('disponible'),
      onRegisteredSW: (_url, r) => {
        registro = r;
        // Al volver a la app, se mira si hay algo nuevo.
        document.addEventListener('visibilitychange', () => {
          if (document.visibilityState === 'visible') void r?.update().catch(() => {});
        });
      },
    });
  }
  return () => oyentes.delete(f);
}

/** Pregunta al servidor si hay una versión nueva. */
export async function buscarActualizacion(): Promise<EstadoActualizacion> {
  if (!registro) return estado;
  if (estado === 'disponible') return estado;
  poner('buscando');
  try {
    await registro.update();
    // Si hay una nueva, se instala y `onNeedRefresh` la anuncia; se le da un momento.
    const nueva = registro.installing ?? registro.waiting;
    if (nueva && nueva.state !== 'installed' && nueva.state !== 'activated') {
      await new Promise<void>((ok) => {
        const fin = () => ok();
        nueva.addEventListener('statechange', () => nueva.state !== 'installing' && fin());
        setTimeout(fin, 15000);
      });
    }
    if (registro.waiting) poner('disponible');
    else if (estado === 'buscando') poner('al-dia');
  } catch {
    poner('sin-conexion');
  }
  return estado;
}

/**
 * Activa la versión nueva y recarga la página. Se hace a mano (y no con la función de
 * `registerSW`) porque esa solo recarga si fue ella quien encontró la versión, y aquí también
 * se encuentra al buscar a mano o al volver a la app.
 */
export function actualizarAhora() {
  const nueva = registro?.waiting;
  if (!nueva) return location.reload();
  navigator.serviceWorker.addEventListener('controllerchange', () => location.reload(), { once: true });
  nueva.postMessage({ type: 'SKIP_WAITING' });
  // Por si el cambio no llega a avisar.
  setTimeout(() => location.reload(), 5000);
}
