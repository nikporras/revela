// Zoom y arrastre de la foto en pantalla. La foto se dibuja "ajustada" (cabe entera) y encima se
// le aplica `translate(x, y) scale(z)` con el origen en su centro: `z = 1` es ajustar a pantalla.
// Todo en píxeles CSS, relativo a la sección de la foto (cuyo centro es el de la foto ajustada).

export interface Vista {
  z: number;
  x: number;
  y: number;
}

export const AJUSTADA: Vista = { z: 1, x: 0, y: 0 };

export interface Medidas {
  /** Tamaño de la foto ajustada (sin zoom). */
  fotoW: number;
  fotoH: number;
  /** Tamaño de la sección donde se ve. */
  areaW: number;
  areaH: number;
  zMax: number;
}

export function esAjustada(v: Vista): boolean {
  return v.z <= 1.001;
}

/** Sin pasarse de zoom y sin dejar hueco a los lados si la foto es más grande que el área. */
export function limitar(v: Vista, m: Medidas): Vista {
  const z = Math.min(Math.max(v.z, 1), Math.max(m.zMax, 1));
  const mx = Math.max(0, (m.fotoW * z - m.areaW) / 2);
  const my = Math.max(0, (m.fotoH * z - m.areaH) / 2);
  return { z, x: Math.min(mx, Math.max(-mx, v.x)), y: Math.min(my, Math.max(-my, v.y)) };
}

/**
 * Cambia el zoom a `z` dejando quieto el punto de la foto que está bajo `(fx, fy)` (coordenadas
 * de la sección), y además lo desplaza `(dx, dy)` (el arrastre de dos dedos).
 */
export function acercar(v: Vista, z: number, fx: number, fy: number, m: Medidas, dx = 0, dy = 0): Vista {
  const cx = m.areaW / 2;
  const cy = m.areaH / 2;
  // Punto de la foto (respecto a su centro, sin zoom) bajo el foco.
  const px = (fx - cx - v.x) / v.z;
  const py = (fy - cy - v.y) / v.z;
  return limitar({ z, x: fx + dx - cx - z * px, y: fy + dy - cy - z * py }, m);
}

/** Interpola dos vistas (para animar el doble toque). */
export function mezclar(a: Vista, b: Vista, t: number): Vista {
  const e = t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
  return { z: a.z + (b.z - a.z) * e, x: a.x + (b.x - a.x) * e, y: a.y + (b.y - a.y) * e };
}
