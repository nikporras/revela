<script lang="ts">
  // Mezcla de color (HSL del motor): se elige uno de los 8 colores y se ajustan su tono, su
  // saturación y su luminancia.
  import Fila from './Fila.svelte';
  import { BANDAS, type CanalHsl } from '../lib/receta';

  let {
    hsl,
    oncambio,
    onfin,
  }: {
    hsl: Record<CanalHsl, number[]>;
    oncambio: (canal: CanalHsl, banda: number, v: number) => void;
    onfin: () => void;
  } = $props();

  let banda = $state(0);
  const t = $derived(BANDAS[banda].tono);
  const tocada = (i: number) => hsl.tono[i] !== 0 || hsl.saturacion[i] !== 0 || hsl.luminancia[i] !== 0;

  function restablecer() {
    for (const c of ['tono', 'saturacion', 'luminancia'] as CanalHsl[]) oncambio(c, banda, 0);
    onfin();
  }
</script>

<div class="colores" role="radiogroup" aria-label="Color">
  {#each BANDAS as b, i (b.nombre)}
    <button
      role="radio"
      aria-checked={banda === i}
      aria-label={b.nombre}
      class:activo={banda === i}
      style:--c="hsl({b.tono} 85% 55%)"
      onclick={() => (banda = i)}
    >
      {#if tocada(i)}<span class="punto"></span>{/if}
    </button>
  {/each}
</div>

<div class="cabecera">
  <span>{BANDAS[banda].nombre}</span>
  <button class="cero" disabled={!tocada(banda)} onclick={restablecer}>Restablecer</button>
</div>

<div class="filas">
  <!-- El motor mueve el tono hasta ±30° en cada extremo. -->
  <Fila
    nombre="Tono"
    valor={hsl.tono[banda]}
    min={-100}
    max={100}
    pista="linear-gradient(to right, hsl({t - 30} 85% 55%), hsl({t} 85% 55%), hsl({t + 30} 85% 55%))"
    oncambio={(v) => oncambio('tono', banda, v)}
    {onfin}
  />
  <Fila
    nombre="Saturación"
    valor={hsl.saturacion[banda]}
    min={-100}
    max={100}
    pista="linear-gradient(to right, hsl({t} 0% 55%), hsl({t} 100% 50%))"
    oncambio={(v) => oncambio('saturacion', banda, v)}
    {onfin}
  />
  <Fila
    nombre="Luminancia"
    valor={hsl.luminancia[banda]}
    min={-100}
    max={100}
    pista="linear-gradient(to right, hsl({t} 70% 12%), hsl({t} 70% 50%), hsl({t} 70% 88%))"
    oncambio={(v) => oncambio('luminancia', banda, v)}
    {onfin}
  />
</div>

<style>
  .colores {
    display: flex;
    justify-content: space-between;
    padding: 2px 16px 4px;
  }
  .colores button {
    position: relative;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: var(--c);
    border: 3px solid var(--panel);
    box-shadow: 0 0 0 1px transparent;
  }
  .colores .activo {
    box-shadow: 0 0 0 2px var(--texto);
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
  .cabecera {
    display: flex;
    align-items: center;
    padding: 0 16px;
    min-height: 32px;
    font-size: 14px;
  }
  .cero {
    margin-left: auto;
    color: var(--acento);
    font-size: 14px;
    min-height: 32px;
  }
  .filas {
    padding: 0 16px;
  }
</style>
