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

/**
 * Comparte el archivo con la hoja de compartir del sistema, o lo descarga si el navegador no
 * puede compartir archivos. Devuelve cómo terminó.
 */
export async function compartirODescargar(archivo: File): Promise<'compartido' | 'cancelado' | 'descargado'> {
  if (navigator.canShare?.({ files: [archivo] })) {
    try {
      await navigator.share({ files: [archivo], title: archivo.name });
      return 'compartido';
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return 'cancelado';
      // Otro error (por ejemplo, sin gesto del usuario): se cae a la descarga.
    }
  }
  descargar(archivo);
  return 'descargado';
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
