// Guardar en una carpeta elegida por el usuario (por ejemplo Pictures/Revela, que la galería
// muestra como álbum). Usa la File System Access API cuando el navegador la ofrece y recuerda la
// carpeta en IndexedDB; si no la ofrece, la app sigue descargando a Download.

interface DirHandle {
  name: string;
  getFileHandle(name: string, opts?: { create?: boolean }): Promise<FileHandle>;
  queryPermission?(opts: { mode: 'readwrite' }): Promise<PermissionState>;
  requestPermission?(opts: { mode: 'readwrite' }): Promise<PermissionState>;
}

interface FileHandle {
  createWritable(): Promise<{ write(data: Blob): Promise<void>; close(): Promise<void> }>;
}

type ConSelector = Window & {
  showDirectoryPicker?: (opts?: { id?: string; mode?: 'readwrite'; startIn?: string }) => Promise<DirHandle>;
};

export const soportaCarpeta = typeof (window as ConSelector).showDirectoryPicker === 'function';

const DB = 'revela';
const STORE = 'ajustes';
const CLAVE = 'carpetaGuardar';

function abrirDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => r.result.createObjectStore(STORE);
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });
}

async function leerCarpeta(): Promise<DirHandle | undefined> {
  try {
    const db = await abrirDb();
    return await new Promise((resolve) => {
      const r = db.transaction(STORE).objectStore(STORE).get(CLAVE);
      r.onsuccess = () => resolve(r.result as DirHandle | undefined);
      r.onerror = () => resolve(undefined);
    });
  } catch {
    return undefined;
  }
}

async function recordarCarpeta(h: DirHandle | undefined) {
  try {
    const db = await abrirDb();
    const tx = db.transaction(STORE, 'readwrite');
    if (h) tx.objectStore(STORE).put(h, CLAVE);
    else tx.objectStore(STORE).delete(CLAVE);
  } catch {
    // Sin IndexedDB, la carpeta se vuelve a pedir la próxima vez.
  }
}

/** Nombre de la carpeta recordada, para mostrarlo en el botón. */
export async function carpetaRecordada(): Promise<string | undefined> {
  return (await leerCarpeta())?.name;
}

export async function olvidarCarpeta() {
  await recordarCarpeta(undefined);
}

export type ResultadoGuardar = { estado: 'guardado'; carpeta: string; nombre: string } | { estado: 'cancelado' } | { estado: 'error'; detalle: string };

/**
 * Guarda `archivo` en la carpeta recordada, o pide una (la primera vez, o con `elegir`). Si ya
 * existe un archivo con ese nombre, añade "-2", "-3"… Debe llamarse desde un toque del usuario.
 */
export async function guardarEnCarpeta(archivo: File, elegir = false): Promise<ResultadoGuardar> {
  const selector = (window as ConSelector).showDirectoryPicker;
  if (!selector) return { estado: 'error', detalle: 'este navegador no permite elegir carpetas' };
  try {
    let dir = elegir ? undefined : await leerCarpeta();
    if (dir) {
      const permiso = (await dir.queryPermission?.({ mode: 'readwrite' })) ?? 'granted';
      if (permiso !== 'granted' && (await dir.requestPermission?.({ mode: 'readwrite' })) !== 'granted') dir = undefined;
    }
    if (!dir) {
      dir = await selector({ id: 'revela-guardar', mode: 'readwrite', startIn: 'pictures' });
      await recordarCarpeta(dir);
    }
    const nombre = await nombreLibre(dir, archivo.name);
    const w = await (await dir.getFileHandle(nombre, { create: true })).createWritable();
    await w.write(archivo);
    await w.close();
    return { estado: 'guardado', carpeta: dir.name, nombre };
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') return { estado: 'cancelado' };
    return { estado: 'error', detalle: e instanceof Error ? `${e.name}: ${e.message}` : String(e) };
  }
}

/** `nombre`, o `base-2.ext`, `base-3.ext`… el primero que no exista en `dir`. */
async function nombreLibre(dir: DirHandle, nombre: string): Promise<string> {
  const m = /^(.*?)(\.[^.]*)?$/.exec(nombre);
  const base = m?.[1] ?? nombre;
  const ext = m?.[2] ?? '';
  for (let i = 1; i < 1000; i++) {
    const n = i === 1 ? nombre : `${base}-${i}${ext}`;
    try {
      await dir.getFileHandle(n);
    } catch {
      return n;
    }
  }
  return `${base}-${Date.now()}${ext}`;
}

/**
 * Nombre base para la exportación. Android entrega fotos de la galería con su número interno
 * ("1000205213.jpg") e iOS a veces como "image.jpg": en esos casos se usa la fecha de captura,
 * como la nombra la cámara ("20261011_011847").
 */
export function nombreBase(nombreArchivo: string, tomadaEl: string | null): string {
  const base = nombreArchivo.replace(/\.[^.]+$/, '').trim();
  const sinSentido = base === '' || /^\d+$/.test(base) || /^(image|imagen|photo|foto|img)$/i.test(base);
  if (sinSentido && tomadaEl) return tomadaEl;
  return base || 'foto';
}
