// Filtros: presets de LightCraft traducidos a los ajustes de Revela
// (scripts/traducir_filtros.py → filtros.json; ver la atribución en ese JSON y en ATTRIBUTION.md).
import DATOS from './filtros.json';

export interface Filtro {
  id: string;
  nombre: string;
  /** Nombre del preset en LightCraft. */
  original: string;
  grupo: string;
  /** "completo": todos sus controles existen en Revela. "aproximado": ver `nota`. */
  tipo: 'completo' | 'aproximado';
  nota: string;
  ajustes: Record<string, unknown>;
}

export const FILTROS: Filtro[] = DATOS.filtros as Filtro[];

export const GRUPOS: string[] = [...new Set(FILTROS.map((f) => f.grupo))];

/** Cada filtro al 100 % como receta de prueba (para la prueba de igualdad). */
export function recetasDeFiltros(): { nombre: string; json: string }[] {
  return FILTROS.map((f) => ({ nombre: `filtro ${f.nombre}`, json: JSON.stringify(f.ajustes) }));
}
