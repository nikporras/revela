// La edición en curso, guardada en el dispositivo (IndexedDB) para que no se pierda si se
// actualiza la app o Android la cierra en segundo plano. Solo una: la última.
//
// Dos entradas: la foto (el archivo original, se escribe una vez al abrirla) y la receta con su
// historial (pequeña, se escribe en cada cambio). Así no se reescriben megas a cada paso.
import type { Receta } from './receta';

export interface FotoGuardada {
  archivo: File;
  deCamara: boolean;
}

export interface RecetaGuardada {
  receta: Receta;
  historial: Receta[];
  posicion: number;
}

const DB = 'revela-edicion';
const STORE = 'edicion';
let db: Promise<IDBDatabase> | undefined;

function abrir(): Promise<IDBDatabase> {
  db ??= new Promise((ok, mal) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => r.result.createObjectStore(STORE);
    r.onsuccess = () => ok(r.result);
    r.onerror = () => mal(r.error);
  });
  return db;
}

async function escribir(f: (s: IDBObjectStore) => void): Promise<void> {
  const d = await abrir();
  await new Promise<void>((ok, mal) => {
    const tx = d.transaction(STORE, 'readwrite');
    f(tx.objectStore(STORE));
    tx.oncomplete = () => ok();
    tx.onerror = () => mal(tx.error);
    tx.onabort = () => mal(tx.error);
  });
}

function leer<T>(clave: string): Promise<T | undefined> {
  return abrir().then(
    (d) =>
      new Promise((ok, mal) => {
        const r = d.transaction(STORE).objectStore(STORE).get(clave);
        r.onsuccess = () => ok(r.result as T | undefined);
        r.onerror = () => mal(r.error);
      }),
  );
}

/** Una foto nueva: sustituye a la anterior (y su receta). */
export async function guardarFoto(f: FotoGuardada, r: RecetaGuardada): Promise<void> {
  // Que el navegador no borre estos datos si le falta sitio (si lo permite).
  void navigator.storage?.persist?.().catch(() => {});
  await escribir((s) => {
    s.put(f, 'foto');
    s.put(r, 'receta');
  });
}

/** La receta cambió; se escribe la última (las anteriores en cola se saltan). */
let pendiente: RecetaGuardada | null = null;
let escribiendo = false;
export async function guardarReceta(r: RecetaGuardada): Promise<void> {
  pendiente = r;
  if (escribiendo) return;
  escribiendo = true;
  try {
    while (pendiente) {
      const x = pendiente;
      pendiente = null;
      await escribir((s) => s.put(x, 'receta'));
    }
  } finally {
    escribiendo = false;
  }
}

export async function leerEdicion(): Promise<{ foto: FotoGuardada; receta: RecetaGuardada | undefined } | null> {
  const foto = await leer<FotoGuardada>('foto');
  if (!foto?.archivo) return null;
  return { foto, receta: await leer<RecetaGuardada>('receta') };
}

export async function borrarEdicion(): Promise<void> {
  pendiente = null;
  await escribir((s) => s.clear());
}
