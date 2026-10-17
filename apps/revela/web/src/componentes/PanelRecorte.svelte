<script lang="ts">
  import { ANGULO_MAX, PROPORCIONES, type Geometria, type Proporcion } from '../lib/recorte';

  let {
    geometria,
    modoLinea,
    onproporcion,
    onangulo,
    onfin,
    ongirar,
    onvoltearH,
    onvoltearV,
    onlinea,
    onrestablecer,
  }: {
    geometria: Geometria;
    modoLinea: boolean;
    onproporcion: (p: Proporcion) => void;
    /** Mientras se arrastra la rueda de enderezado. */
    onangulo: (grados: number) => void;
    /** Al soltar: un paso de deshacer. */
    onfin: () => void;
    ongirar: () => void;
    onvoltearH: () => void;
    onvoltearV: () => void;
    onlinea: () => void;
    onrestablecer: () => void;
  } = $props();

  const angulo = $derived(geometria.angulo);
</script>

<div class="chips" role="radiogroup" aria-label="Proporción">
  {#each PROPORCIONES as p (p.id)}
    <button role="radio" aria-checked={geometria.proporcion === p.id} class:activo={geometria.proporcion === p.id} onclick={() => onproporcion(p.id)}>
      {p.nombre}
    </button>
  {/each}
</div>

<div class="enderezar">
  <span>Enderezar</span>
  <b>{angulo > 0 ? '+' : ''}{angulo.toFixed(1)}°</b>
  <button class="cero" disabled={angulo === 0} onclick={() => { onangulo(0); onfin(); }}>0°</button>
</div>
<input
  class="rueda"
  type="range"
  aria-label="Enderezar"
  min={-ANGULO_MAX}
  max={ANGULO_MAX}
  step="0.1"
  value={angulo}
  oninput={(e) => onangulo(Number((e.currentTarget as HTMLInputElement).value))}
  onchange={onfin}
/>

<div class="acciones">
  <button onclick={ongirar} aria-label="Girar 90° a la izquierda">
    <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 3-6.7" /><path d="M3 4v5h5" /></svg>
    <span>Girar</span>
  </button>
  <button onclick={onvoltearH} aria-label="Voltear en horizontal">
    <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M12 3v18" stroke-dasharray="2 2" /><path d="M9 7 3 17h6z" /><path d="m15 7 6 10h-6z" fill="currentColor" /></svg>
    <span>Voltear ⇆</span>
  </button>
  <button onclick={onvoltearV} aria-label="Voltear en vertical">
    <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M3 12h18" stroke-dasharray="2 2" /><path d="M7 9 17 3v6z" /><path d="m7 15 10 6v-6z" fill="currentColor" /></svg>
    <span>Voltear ⇅</span>
  </button>
  <button class:activo={modoLinea} onclick={onlinea} aria-pressed={modoLinea} aria-label="Enderezar con una línea">
    <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 18 21 8" /><circle cx="3" cy="18" r="1.6" fill="currentColor" /><circle cx="21" cy="8" r="1.6" fill="currentColor" /></svg>
    <span>Con línea</span>
  </button>
  <button onclick={onrestablecer} aria-label="Restablecer encuadre">
    <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4v6h6" /><path d="M20 20v-6h-6" /><path d="M5 15a8 8 0 0 0 14 2M19 9A8 8 0 0 0 5 7" /></svg>
    <span>Restablecer</span>
  </button>
</div>
{#if modoLinea}
  <p class="ayuda">Traza una línea sobre algo que deba quedar horizontal o vertical (el horizonte, una pared).</p>
{/if}

<style>
  .chips {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    padding: 4px 16px 6px;
    scrollbar-width: none;
  }
  .chips button {
    flex: none;
    min-height: 38px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--panel-2);
    color: var(--texto-2);
    font-variant-numeric: tabular-nums;
  }
  .chips .activo {
    background: var(--texto);
    color: var(--fondo);
    font-weight: 600;
  }
  .enderezar {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 2px 16px 0;
  }
  .enderezar b {
    font-variant-numeric: tabular-nums;
  }
  .cero {
    margin-left: auto;
    color: var(--acento);
    min-height: 32px;
  }
  .rueda {
    display: block;
    width: calc(100% - 32px);
    margin: 0 16px;
    height: 36px;
    accent-color: var(--acento);
  }
  .acciones {
    display: flex;
    justify-content: space-around;
    padding: 2px 8px 0;
  }
  .acciones button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 60px;
    min-height: 52px;
    color: var(--texto-2);
    font-size: 12px;
    border-radius: 12px;
  }
  .acciones .activo {
    color: var(--acento);
    background: var(--panel-2);
  }
  .ayuda {
    margin: 4px 16px 0;
    color: var(--texto-2);
    font-size: 12px;
    text-align: center;
  }
</style>
