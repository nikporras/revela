// Detalles del dispositivo y del navegador.

export const esIOS =
  /iPad|iPhone|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);

/** Límite de megapíxeles por defecto: Safari de iOS mata la pestaña con mucha menos memoria. */
export const LIMITE_MP_DEFECTO = esIOS ? 16 : 24;
export const LIMITES_MP = [8, 12, 16, 24, 36, 48];

const CLAVE_LIMITE = 'revela.limiteMp';

export function leerLimiteMp(): number {
  try {
    const v = Number(localStorage.getItem(CLAVE_LIMITE));
    if (LIMITES_MP.includes(v)) return v;
  } catch {
    // Almacenamiento bloqueado (modo privado): se usa el valor por defecto.
  }
  return LIMITE_MP_DEFECTO;
}

export function guardarLimiteMp(v: number) {
  try {
    localStorage.setItem(CLAVE_LIMITE, String(v));
  } catch {
    // Sin almacenamiento, el ajuste dura solo esta sesión.
  }
}

/** Lado mayor de la vista previa: lo que la pantalla puede mostrar, como mucho 2048 px. */
export function ladoPrevia(): number {
  const lado = Math.max(screen.width, screen.height) * (window.devicePixelRatio || 1);
  return Math.min(2048, Math.max(1024, Math.round(lado)));
}

/** `true` si el navegador puede pasar este archivo a la hoja de compartir del sistema. */
export function puedeCompartir(archivo: File): boolean {
  try {
    return !!navigator.canShare?.({ files: [archivo] });
  } catch {
    return false;
  }
}

export type ResultadoCompartir =
  | { estado: 'compartido' | 'cancelado' }
  | { estado: 'no-soportado' }
  | { estado: 'error'; detalle: string };

/**
 * Abre la hoja de compartir del sistema con el archivo. Nunca descarga a escondidas: si no se
 * puede compartir, lo dice, y "Guardar" queda como acción aparte.
 */
export async function compartir(archivo: File): Promise<ResultadoCompartir> {
  if (!puedeCompartir(archivo)) return { estado: 'no-soportado' };
  try {
    // Solo `files`: con `title`/`text`, algunas apps de Android comparten el texto y no la foto.
    await navigator.share({ files: [archivo] });
    return { estado: 'compartido' };
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') return { estado: 'cancelado' };
    const detalle = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
    return { estado: 'error', detalle };
  }
}

export function descargar(archivo: File) {
  const url = URL.createObjectURL(archivo);
  const a = document.createElement('a');
  a.href = url;
  a.download = archivo.name;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

export function mb(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function ms(v: number): string {
  return v >= 1000 ? `${(v / 1000).toFixed(2)} s` : `${Math.round(v)} ms`;
}
