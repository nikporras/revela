<script lang="ts">
  // Mezcla de color (HSL del motor), como en Lightroom móvil: se elige uno de los 8 colores y se
  // ajustan su tono, su saturación y su luminancia. Con "Sobre la foto", se toca un color en la
  // foto y se arrastra en vertical (el gesto lo lleva App, que sabe dónde está la foto).
  import Fila from './Fila.svelte';
  import { BANDAS, type CanalHsl } from '../lib/receta';

  let {
    hsl,
    banda = $bindable(0),
    directo,
    ondirecto,
    oncambio,
    onfin,
  }: {
    hsl: Record<CanalHsl, number[]>;
    /** El color elegido (el ajuste sobre la foto elige el que detecta). */
    banda?: number;
    /** Canal que cambia el ajuste sobre la foto, o `null` si está apagado. */
    directo: CanalHsl | null;
    ondirecto: (c: CanalHsl | null) => void;
    oncambio: (canal: CanalHsl, banda: number, v: number) => void;
    onfin: () => void;
  } = $props();

  const CANALES: { id: CanalHsl; nombre: string }[] = [
    { id: 'tono', nombre: 'Tono' },
    { id: 'saturacion', nombre: 'Saturación' },
    { id: 'luminancia', nombre: 'Luminancia' },
  ];

  const n = BANDAS.length;
  const t = $derived(BANDAS[banda].tono);
  /** Tonos de los colores vecinos, del lado correcto del círculo (rojo: de magenta a naranja). */
  const antes = $derived.by(() => {
    const a = BANDAS[(banda + n - 1) % n].tono;
    return a > t ? a - 360 : a;
  });
  const despues = $derived.by(() => {
    const d = BANDAS[(banda + 1) % n].tono;
    return d < t ? d + 360 : d;
  });
  /** Tono: de un vecino al otro pasando por el color, con varias paradas para seguir el círculo. */
  const pistaTono = $derived.by(() => {
    const paradas: string[] = [];
    for (let i = 0; i <= 8; i++) {
      const h = i <= 4 ? antes + ((t - antes) * i) / 4 : t + ((despues - t) * (i - 4)) / 4;
      paradas.push(`hsl(${h} 85% 55%) ${(i / 8) * 100}%`);
    }
    return `linear-gradient(to right, ${paradas.join(', ')})`;
  });

  const tocada = (i: number) => hsl.tono[i] !== 0 || hsl.saturacion[i] !== 0 || hsl.luminancia[i] !== 0;

  function restablecer() {
    for (const c of ['tono', 'saturacion', 'luminancia'] as CanalHsl[]) oncambio(c, banda, 0);
    onfin();
  }

  // Al salir de la mezcla (otra sección, otra pestaña) se apaga el ajuste sobre la foto.
  $effect(() => () => ondirecto(null));
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
  <button
    class="directo"
    class:activo={directo !== null}
    aria-pressed={directo !== null}
    onclick={() => ondirecto(directo ? null : 'saturacion')}
  >
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <circle cx="12" cy="12" r="7" /><circle cx="12" cy="12" r="1.5" fill="currentColor" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3" />
    </svg>
    Sobre la foto
  </button>
  <button class="cero" disabled={!tocada(banda)} onclick={restablecer}>Restablecer</button>
</div>

{#if directo}
  <div class="canal" role="radiogroup" aria-label="Arrastrar sobre la foto cambia">
    <span>Arrastrar ↕ cambia</span>
    {#each CANALES as c (c.id)}
      <button role="radio" aria-checked={directo === c.id} class:activo={directo === c.id} onclick={() => ondirecto(c.id)}>{c.nombre}</button>
    {/each}
  </div>
{/if}

<div class="filas">
  <Fila nombre="Tono" valor={hsl.tono[banda]} min={-100} max={100} pista={pistaTono} oncambio={(v) => oncambio('tono', banda, v)} {onfin} />
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
    gap: 8px;
    padding: 0 16px;
    min-height: 36px;
    font-size: 14px;
  }
  .directo {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    min-height: 32px;
    padding: 0 10px;
    border-radius: 999px;
    background: var(--panel-2);
    color: var(--texto-2);
    font-size: 13px;
  }
  .directo.activo {
    background: var(--acento);
    color: var(--fondo);
    font-weight: 600;
  }
  .cero {
    color: var(--acento);
    font-size: 14px;
    min-height: 32px;
  }
  .canal {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 16px 4px;
    font-size: 12px;
    color: var(--texto-2);
  }
  .canal span {
    margin-right: auto;
  }
  .canal button {
    min-height: 30px;
    padding: 0 10px;
    border-radius: 999px;
    background: var(--panel-2);
    color: var(--texto-2);
    font-size: 13px;
  }
  .canal .activo {
    background: var(--texto);
    color: var(--fondo);
    font-weight: 600;
  }
  .filas {
    padding: 0 16px;
  }
</style>
