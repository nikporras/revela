<script module lang="ts">
  export type Seccion = 'luz' | 'color' | 'efectos' | 'curvas';

  /** El control elegido en cada sección: se recuerda al ir a otra pestaña y volver. */
  const elegidos = $state({ luz: 'exposicion', color: 'temperatura' });
</script>

<script lang="ts">
  // Pestaña Editar: secciones Luz, Color, Efectos y Curvas (como Lightroom móvil).
  import Sliders from './Sliders.svelte';
  import Curvas from './Curvas.svelte';
  import Mezcla from './Mezcla.svelte';
  import Gradacion from './Gradacion.svelte';
  import {
    COLOR,
    LUZ,
    esCurvaNeutra,
    esGradacionNeutra,
    esHslNeutro,
    type Canal,
    type CanalHsl,
    type IdRueda,
    type IdSlider,
    type Punto,
    type Receta,
    type Rueda,
  } from '../lib/receta';

  let {
    receta,
    luts,
    seccion = $bindable('luz'),
    bandaMezcla = $bindable(0),
    directo,
    ondirecto,
    oncambio,
    oncurva,
    onhsl,
    onrueda,
    onmezcla,
    onbalance,
    onfin,
  }: {
    receta: Receta;
    luts: Float32Array | null;
    seccion?: Seccion;
    bandaMezcla?: number;
    directo: CanalHsl | null;
    ondirecto: (c: CanalHsl | null) => void;
    oncambio: (id: IdSlider, v: number) => void;
    oncurva: (canal: Canal, puntos: Punto[]) => void;
    onhsl: (canal: CanalHsl, banda: number, v: number) => void;
    onrueda: (id: IdRueda, r: Rueda) => void;
    onmezcla: (v: number) => void;
    onbalance: (v: number) => void;
    onfin: () => void;
  } = $props();

  const s = $derived(receta.sliders);
  const SECCIONES: { id: Seccion; nombre: string; cambiada: () => boolean }[] = [
    { id: 'luz', nombre: 'Luz', cambiada: () => LUZ.some((id) => s[id] !== 0) },
    { id: 'color', nombre: 'Color', cambiada: () => COLOR.some((id) => s[id] !== 0) || !esHslNeutro(receta.hsl) || !esGradacionNeutra(receta.gradacion) },
    { id: 'efectos', nombre: 'Efectos', cambiada: () => false },
    { id: 'curvas', nombre: 'Curvas', cambiada: () => Object.values(receta.curvas).some((c) => !esCurvaNeutra(c)) },
  ];
</script>

<div class="secciones" role="tablist" aria-label="Sección">
  {#each SECCIONES as x (x.id)}
    <button role="tab" aria-selected={seccion === x.id} class:activa={seccion === x.id} onclick={() => (seccion = x.id)}>
      {x.nombre}{#if x.cambiada()}<span class="punto" aria-label="(modificada)"></span>{/if}
    </button>
  {/each}
</div>

{#if seccion === 'luz'}
  <Sliders ids={LUZ} valores={s} bind:activo={elegidos.luz} {oncambio} {onfin} />
{:else if seccion === 'color'}
  <Sliders
    ids={COLOR}
    valores={s}
    bind:activo={elegidos.color}
    extras={[
      { id: 'mezcla', nombre: 'Mezcla', cambiado: !esHslNeutro(receta.hsl) },
      { id: 'gradacion', nombre: 'Gradación', cambiado: !esGradacionNeutra(receta.gradacion) },
    ]}
    {oncambio}
    {onfin}
  >
    {#snippet panel(id)}
      {#if id === 'mezcla'}
        <Mezcla hsl={receta.hsl} bind:banda={bandaMezcla} {directo} {ondirecto} oncambio={onhsl} {onfin} />
      {:else if id === 'gradacion'}
        <Gradacion gradacion={receta.gradacion} {onrueda} {onmezcla} {onbalance} {onfin} />
      {/if}
    {/snippet}
  </Sliders>
{:else if seccion === 'efectos'}
  <p class="pronto">Viñeta, grano y nitidez llegan en los próximos pasos.</p>
{:else}
  <Curvas curvas={receta.curvas} {luts} oncambio={oncurva} {onfin} />
{/if}

<style>
  .secciones {
    display: flex;
    gap: 4px;
    margin: 0 16px 6px;
    padding: 3px;
    border-radius: 12px;
    background: var(--panel-2);
  }
  .secciones button {
    flex: 1;
    position: relative;
    min-height: 34px;
    border-radius: 9px;
    color: var(--texto-2);
    font-size: 14px;
  }
  .secciones .activa {
    background: var(--panel);
    color: var(--texto);
    font-weight: 600;
    box-shadow: 0 1px 3px #0006;
  }
  .punto {
    position: absolute;
    top: 6px;
    right: 8px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--acento);
  }
  .pronto {
    margin: 12px 16px;
    color: var(--texto-2);
    font-size: 14px;
    text-align: center;
  }
</style>
