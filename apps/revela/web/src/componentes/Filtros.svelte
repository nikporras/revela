<script lang="ts">
  import { FILTROS, type Filtro } from '../lib/filtros';
  import { iman } from '../lib/iman';
  import Valor from './Valor.svelte';

  let {
    filtro,
    dibujarMiniatura,
    claveFoto,
    oncambio,
    onfin,
  }: {
    filtro: { id: string; intensidad: number } | null;
    /** Dibuja la foto con `f` (o sin filtro) en `lienzo`. */
    dibujarMiniatura: ((lienzo: HTMLCanvasElement, f: Filtro | null) => Promise<void>) | null;
    /** Cambia cuando se abre otra foto: hay que volver a dibujar las miniaturas. */
    claveFoto: number;
    oncambio: (f: { id: string; intensidad: number } | null) => void;
    onfin: () => void;
  } = $props();

  const lienzos: HTMLCanvasElement[] = $state([]);
  let lienzoNinguno: HTMLCanvasElement | undefined = $state();

  $effect(() => {
    void claveFoto;
    const dibujar = dibujarMiniatura;
    if (!dibujar) return;
    (async () => {
      if (lienzoNinguno) await dibujar(lienzoNinguno, null);
      for (let i = 0; i < FILTROS.length; i++) if (lienzos[i]) await dibujar(lienzos[i], FILTROS[i]);
    })();
  });

  const actual = $derived(filtro ? FILTROS.find((f) => f.id === filtro.id) : undefined);

  function elegir(f: Filtro | null) {
    oncambio(f ? { id: f.id, intensidad: filtro?.id === f.id ? filtro.intensidad : 1 } : null);
    onfin();
  }
</script>

<div class="tira" role="listbox" aria-label="Filtros">
  <button role="option" aria-selected={!filtro} class:activo={!filtro} onclick={() => elegir(null)}>
    <canvas bind:this={lienzoNinguno}></canvas>
    <span>Original</span>
  </button>
  {#each FILTROS as f, i (f.id)}
    <button role="option" aria-selected={filtro?.id === f.id} class:activo={filtro?.id === f.id} onclick={() => elegir(f)} title={f.nota || 'Igual que el original de LightCraft'}>
      <canvas bind:this={lienzos[i]}></canvas>
      <span>{f.nombre}{f.tipo === 'aproximado' ? ' ≈' : ''}</span>
    </button>
  {/each}
</div>

{#if filtro && actual}
  <div class="intensidad">
    <span>{actual.nombre}</span>
    <!-- El contenedor recibe el dedo (arrastre relativo y ajuste fino, ver lib/iman.ts). -->
    <span class="pista">
      <input
        type="range"
        use:iman={{ neutro: 100 }}
        aria-label="Intensidad"
        min="0"
        max="100"
        step="1"
        value={Math.round(filtro.intensidad * 100)}
        oninput={(e) => oncambio({ id: filtro.id, intensidad: Number((e.currentTarget as HTMLInputElement).value) / 100 })}
        onchange={onfin}
      />
    </span>
    <Valor
      texto="{Math.round(filtro.intensidad * 100)} %"
      valor={Math.round(filtro.intensidad * 100)}
      min={0}
      max={100}
      nombre="Intensidad"
      oncambio={(v) => oncambio({ id: filtro.id, intensidad: v / 100 })}
      {onfin}
    />
  </div>
  {#if actual.nota}<p class="nota">≈ {actual.nota}</p>{/if}
{/if}

<style>
  .tira {
    display: flex;
    gap: 10px;
    overflow-x: auto;
    padding: 4px 16px 8px;
    scrollbar-width: none;
  }
  .tira button {
    flex: none;
    width: 76px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    color: var(--texto-2);
    font-size: 12px;
  }
  canvas {
    width: 72px;
    height: 72px;
    object-fit: cover;
    border-radius: 10px;
    background: var(--panel-2);
    border: 2px solid transparent;
  }
  .activo canvas {
    border-color: var(--acento);
  }
  .activo {
    color: var(--texto) !important;
    font-weight: 600;
  }
  span {
    line-height: 1.2;
    text-align: center;
  }
  .intensidad {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 16px;
  }
  .pista {
    flex: 1;
    display: flex;
  }
  .intensidad input {
    flex: 1;
    height: 40px;
    accent-color: var(--acento);
  }
  .nota {
    margin: 0 16px;
    color: var(--texto-2);
    font-size: 12px;
  }
</style>
