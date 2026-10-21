<script lang="ts">
  // Color puntual (como el Point Color de Lightroom): se elige un color en la foto (con la lupa)
  // y solo los colores parecidos cambian de tono, saturación y luminancia. Varios a la vez.
  // Elegir en la foto lo lleva App (sabe dónde está la foto).
  import Fila from './Fila.svelte';
  import { MAX_PUNTUALES } from '../lib/gpu';
  import { RANGO_DEFECTO, esPuntualNeutro, nombreVisible, type Puntual } from '../lib/receta';

  let {
    puntuales,
    elegido = $bindable(0),
    eligiendo,
    ver,
    onelegir,
    oncancelar,
    oncambio,
    onquitar,
    onver,
    onfin,
  }: {
    puntuales: Puntual[];
    /** El color puntual que se está editando. */
    elegido?: number;
    /** Se está eligiendo un color en la foto (`null`: uno nuevo; un número: para ese). */
    eligiendo: { indice: number | null } | null;
    /** El que se resalta (vista de zonas afectadas); −1: ninguno. */
    ver: number;
    onelegir: (indice: number | null) => void;
    oncancelar: () => void;
    oncambio: (indice: number, campo: keyof Puntual, v: number) => void;
    onquitar: (indice: number) => void;
    onver: (indice: number) => void;
    onfin: () => void;
  } = $props();

  const p = $derived(puntuales[elegido] as Puntual | undefined);
  /** El color elegido, en CSS (de HSV a HSL). */
  function color(x: Puntual): string {
    const l = x.brillo0 * (1 - x.sat0 / 2);
    const s = l === 0 || l === 1 ? 0 : (x.brillo0 - l) / Math.min(l, 1 - l);
    return `hsl(${x.tono0} ${Math.round(s * 100)}% ${Math.round(l * 100)}%)`;
  }

  // Al salir del panel se deja de elegir y de resaltar.
  $effect(() => () => {
    oncancelar();
    onver(-1);
  });
</script>

<div class="muestras" role="radiogroup" aria-label="Colores puntuales">
  {#each puntuales as x, i (i)}
    <button role="radio" aria-checked={elegido === i} aria-label="Color puntual {i + 1}: {nombreVisible(x.tono0, x.sat0, x.brillo0)}" class:activo={elegido === i} style:--c={color(x)} onclick={() => (elegido = i)}>
      {#if !esPuntualNeutro(x)}<span class="punto"></span>{/if}
    </button>
  {/each}
  {#if puntuales.length < MAX_PUNTUALES}
    <button class="nuevo" class:activo={eligiendo?.indice === null} aria-label="Añadir un color puntual" onclick={() => (eligiendo?.indice === null ? oncancelar() : onelegir(null))}>+</button>
  {/if}
</div>

{#if eligiendo}
  <div class="aviso">
    <span>{eligiendo.indice === null ? 'Toca un color en la foto' : 'Toca el nuevo color en la foto'} (mantén pulsado para la lupa).</span>
    <button class="enlace" onclick={oncancelar}>Cancelar</button>
  </div>
{:else if !p}
  <p class="ayuda">Toca <b>+</b> y elige un color en la foto: solo los colores parecidos cambiarán.</p>
{/if}

{#if p && !eligiendo}
  <div class="cabecera">
    <span class="nombre">{nombreVisible(p.tono0, p.sat0, p.brillo0)}</span>
    <button class="icono" class:activo={ver === elegido} aria-pressed={ver === elegido} aria-label="Ver las zonas afectadas" onclick={() => onver(ver === elegido ? -1 : elegido)}>
      <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z" /><circle cx="12" cy="12" r="3" /></svg>
    </button>
    <button class="icono" aria-label="Volver a elegir el color en la foto" onclick={() => onelegir(elegido)}>
      <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m14 4 6 6" /><path d="m17 7-9.5 9.5L4 20l3.5-3.5" /><path d="M12 9l3 3" /></svg>
    </button>
    <button class="icono" aria-label="Quitar este color puntual" onclick={() => onquitar(elegido)}>
      <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h16" /><path d="M6 7l1 13h10l1-13" /><path d="M9 7V4h6v3" /></svg>
    </button>
  </div>
  <div class="filas">
    <Fila nombre="Rango" valor={p.rango} min={0} max={100} neutro={RANGO_DEFECTO} formato={(v) => String(Math.round(v))} oncambio={(v) => oncambio(elegido, 'rango', v)} {onfin} />
    <!-- Hasta ±45° de giro. -->
    <Fila
      nombre="Tono"
      valor={p.tono}
      min={-100}
      max={100}
      pista="linear-gradient(to right, hsl({p.tono0 - 45} 85% 55%), hsl({p.tono0} 85% 55%), hsl({p.tono0 + 45} 85% 55%))"
      oncambio={(v) => oncambio(elegido, 'tono', v)}
      {onfin}
    />
    <Fila
      nombre="Saturación"
      valor={p.saturacion}
      min={-100}
      max={100}
      pista="linear-gradient(to right, hsl({p.tono0} 0% 55%), hsl({p.tono0} 100% 50%))"
      oncambio={(v) => oncambio(elegido, 'saturacion', v)}
      {onfin}
    />
    <Fila
      nombre="Luminancia"
      valor={p.luminancia}
      min={-100}
      max={100}
      pista="linear-gradient(to right, hsl({p.tono0} 70% 12%), hsl({p.tono0} 70% 50%), hsl({p.tono0} 70% 88%))"
      oncambio={(v) => oncambio(elegido, 'luminancia', v)}
      {onfin}
    />
  </div>
{/if}

<style>
  .muestras {
    display: flex;
    gap: 10px;
    padding: 2px 16px 6px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .muestras button {
    flex: none;
    position: relative;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: var(--c);
    border: 3px solid var(--panel);
  }
  .muestras .activo {
    box-shadow: 0 0 0 2px var(--texto);
  }
  .muestras .nuevo {
    background: var(--panel-2);
    color: var(--texto);
    font-size: 22px;
    line-height: 1;
    border-color: var(--panel-2);
  }
  .muestras .nuevo.activo {
    background: var(--acento);
    color: var(--fondo);
    box-shadow: none;
  }
  .punto {
    position: absolute;
    top: -4px;
    right: -4px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--acento);
    border: 1px solid var(--panel);
  }
  .aviso,
  .ayuda {
    margin: 0;
    padding: 4px 16px 6px;
    font-size: 13px;
    color: var(--texto-2);
  }
  .aviso {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .aviso span {
    flex: 1;
  }
  .enlace {
    color: var(--acento);
    font-weight: 600;
    min-height: 36px;
  }
  .cabecera {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 12px 0 16px;
    min-height: 40px;
    font-size: 14px;
  }
  .nombre {
    flex: 1;
  }
  .icono {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    color: var(--texto-2);
  }
  .icono.activo {
    background: var(--acento);
    color: var(--fondo);
  }
  .filas {
    padding: 0 16px;
  }
</style>
