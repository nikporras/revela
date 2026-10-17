<script lang="ts">
  import { SLIDERS, type IdSlider } from '../lib/receta';

  let {
    valores,
    oncambio,
    onfin,
  }: {
    valores: Record<IdSlider, number>;
    /** Mientras se arrastra (previa en vivo). */
    oncambio: (id: IdSlider, v: number) => void;
    /** Al soltar: un paso de deshacer. */
    onfin: () => void;
  } = $props();

  let activo = $state<IdSlider>('exposicion');
  const def = $derived(SLIDERS.find((s) => s.id === activo) ?? SLIDERS[0]);
  const valor = $derived(valores[activo]);

  function texto(v: number): string {
    if (def.formato) return def.formato(v);
    return `${v > 0 ? '+' : ''}${Math.round(v)}`;
  }

  function restablecer() {
    oncambio(activo, 0);
    onfin();
  }
</script>

<div class="chips" role="tablist" aria-label="Ajuste">
  {#each SLIDERS as s (s.id)}
    <button role="tab" aria-selected={s.id === activo} class:activo={s.id === activo} onclick={() => (activo = s.id)}>
      {s.nombre}{#if valores[s.id] !== 0}<span class="punto" aria-label="(modificado)"></span>{/if}
    </button>
  {/each}
</div>

<div class="control">
  <div class="cabecera">
    <span>{def.nombre}</span>
    <b>{texto(valor)}</b>
    <button class="cero" disabled={valor === 0} onclick={restablecer}>Restablecer</button>
  </div>
  <input
    type="range"
    aria-label={def.nombre}
    min={def.min}
    max={def.max}
    step={def.paso}
    value={valor}
    style="--cero: {((0 - def.min) / (def.max - def.min)) * 100}%"
    oninput={(e) => oncambio(activo, Number((e.currentTarget as HTMLInputElement).value))}
    onchange={onfin}
  />
</div>

<style>
  .chips {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    padding: 4px 16px 8px;
    scrollbar-width: none;
  }
  .chips button {
    flex: none;
    min-height: 40px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--panel-2);
    color: var(--texto-2);
    font-size: 15px;
    position: relative;
  }
  .chips .activo {
    background: var(--texto);
    color: var(--fondo);
    font-weight: 600;
  }
  .punto {
    position: absolute;
    top: 6px;
    right: 7px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--acento);
  }
  .control {
    padding: 4px 16px 0;
  }
  .cabecera {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .cabecera b {
    font-variant-numeric: tabular-nums;
  }
  .cero {
    margin-left: auto;
    color: var(--acento);
    font-size: 14px;
    min-height: 36px;
  }
  input[type='range'] {
    width: 100%;
    height: 44px;
    margin: 0;
    background: transparent;
    -webkit-appearance: none;
    appearance: none;
  }
  input[type='range']::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(to right, var(--borde) calc(var(--cero) - 1px), var(--texto-2) calc(var(--cero) - 1px), var(--texto-2) calc(var(--cero) + 1px), var(--borde) calc(var(--cero) + 1px));
  }
  input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 28px;
    height: 28px;
    margin-top: -12px;
    border-radius: 50%;
    background: var(--texto);
    border: 3px solid var(--fondo);
    box-shadow: 0 0 0 1px var(--borde);
  }
  input[type='range']::-moz-range-track {
    height: 4px;
    background: var(--borde);
  }
  input[type='range']::-moz-range-thumb {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--texto);
  }
</style>
