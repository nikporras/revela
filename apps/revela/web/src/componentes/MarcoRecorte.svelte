<script lang="ts">
  import type { Rect } from '../lib/recorte';

  let {
    recorte,
    aspecto,
    modoLinea,
    oncambio,
    onfin,
    onlinea,
  }: {
    /** `[cx, cy, w, h]` en fracciones del marco (la caja de este componente). */
    recorte: Rect;
    /** Ancho/alto del recorte en fracciones del marco si la proporción está fija; `null`: libre. */
    aspecto: number | null;
    modoLinea: boolean;
    /** Propone un recorte; el padre lo encaja dentro de la foto (sin esquinas vacías). */
    oncambio: (deseado: Rect) => void;
    onfin: () => void;
    /** Una línea trazada en modo línea, en fracciones del marco. */
    onlinea: (x0: number, y0: number, x1: number, y1: number) => void;
  } = $props();

  type Asa = 'mover' | 'n' | 's' | 'e' | 'o' | 'ne' | 'no' | 'se' | 'so';
  const MIN = 0.06;

  let caja: HTMLDivElement | undefined = $state();
  let arrastre: { asa: Asa; x: number; y: number; inicio: Rect } | null = null;
  let linea = $state<[number, number, number, number] | null>(null);

  const izq = $derived((recorte[0] - recorte[2] / 2) * 100);
  const arr = $derived((recorte[1] - recorte[3] / 2) * 100);

  function punto(e: PointerEvent): [number, number] {
    const r = caja!.getBoundingClientRect();
    return [(e.clientX - r.left) / r.width, (e.clientY - r.top) / r.height];
  }

  function bajar(e: PointerEvent) {
    if (!caja) return;
    caja.setPointerCapture(e.pointerId);
    const [x, y] = punto(e);
    if (modoLinea) {
      linea = [x, y, x, y];
      return;
    }
    const asa = ((e.target as HTMLElement).dataset.asa as Asa | undefined) ?? (dentro(x, y) ? 'mover' : null);
    if (!asa) return;
    arrastre = { asa, x, y, inicio: [...recorte] as Rect };
  }

  function dentro(x: number, y: number) {
    return Math.abs(x - recorte[0]) <= recorte[2] / 2 && Math.abs(y - recorte[1]) <= recorte[3] / 2;
  }

  function mover(e: PointerEvent) {
    const [x, y] = punto(e);
    if (linea) {
      linea = [linea[0], linea[1], x, y];
      return;
    }
    if (!arrastre) return;
    const { asa, inicio } = arrastre;
    const dx = x - arrastre.x;
    const dy = y - arrastre.y;
    if (asa === 'mover') {
      oncambio([inicio[0] + dx, inicio[1] + dy, inicio[2], inicio[3]]);
      return;
    }
    // Bordes del recorte; se mueven los que toca el asa.
    let [x0, y0, x1, y1] = [inicio[0] - inicio[2] / 2, inicio[1] - inicio[3] / 2, inicio[0] + inicio[2] / 2, inicio[1] + inicio[3] / 2];
    if (asa.includes('o')) x0 = Math.min(x0 + dx, x1 - MIN);
    if (asa.includes('e')) x1 = Math.max(x1 + dx, x0 + MIN);
    if (asa.includes('n')) y0 = Math.min(y0 + dy, y1 - MIN);
    if (asa.includes('s')) y1 = Math.max(y1 + dy, y0 + MIN);
    if (aspecto) {
      const horizontal = asa === 'e' || asa === 'o';
      const vertical = asa === 'n' || asa === 's';
      let w = x1 - x0;
      let h = y1 - y0;
      if (horizontal) h = w / aspecto;
      else if (vertical) w = h * aspecto;
      else if (w / aspecto > h) h = w / aspecto;
      else w = h * aspecto;
      // El lado opuesto al asa queda fijo; en las asas de un lado, el otro eje crece centrado.
      if (horizontal) {
        const c = (y0 + y1) / 2;
        [y0, y1] = [c - h / 2, c + h / 2];
      } else if (asa.includes('n')) y0 = y1 - h;
      else y1 = y0 + h;
      if (vertical) {
        const c = (x0 + x1) / 2;
        [x0, x1] = [c - w / 2, c + w / 2];
      } else if (asa.includes('o')) x0 = x1 - w;
      else x1 = x0 + w;
    }
    oncambio([(x0 + x1) / 2, (y0 + y1) / 2, x1 - x0, y1 - y0]);
  }

  /** Un segundo dedo (zoom): se abandona la línea y se da por terminado el arrastre. */
  export function cancelar() {
    linea = null;
    if (arrastre) {
      arrastre = null;
      onfin();
    }
  }

  function soltar() {
    if (linea) {
      const [x0, y0, x1, y1] = linea;
      linea = null;
      if (Math.hypot(x1 - x0, y1 - y0) > 0.05) onlinea(x0, y0, x1, y1);
      return;
    }
    if (arrastre) {
      arrastre = null;
      onfin();
    }
  }
</script>

<div
  bind:this={caja}
  class="marco"
  class:linea-activa={modoLinea}
  role="application"
  aria-label={modoLinea ? 'Traza una línea para enderezar' : 'Recorte: arrastra para mover, usa las esquinas para cambiar el tamaño'}
  onpointerdown={bajar}
  onpointermove={mover}
  onpointerup={soltar}
  onpointercancel={soltar}
>
  <div class="recorte" style="left: {izq}%; top: {arr}%; width: {recorte[2] * 100}%; height: {recorte[3] * 100}%">
    <!-- Cuadrícula de tercios -->
    <div class="tercio v" style="left: 33.333%"></div>
    <div class="tercio v" style="left: 66.666%"></div>
    <div class="tercio h" style="top: 33.333%"></div>
    <div class="tercio h" style="top: 66.666%"></div>
    {#if !modoLinea}
      {#each ['no', 'ne', 'so', 'se'] as a (a)}<div class="asa esquina {a}" data-asa={a}></div>{/each}
      {#each ['n', 's', 'e', 'o'] as a (a)}<div class="asa lado {a}" data-asa={a}></div>{/each}
    {/if}
  </div>
  {#if linea}
    <svg class="trazo" viewBox="0 0 1 1" preserveAspectRatio="none">
      <line x1={linea[0]} y1={linea[1]} x2={linea[2]} y2={linea[3]} vector-effect="non-scaling-stroke" />
    </svg>
  {/if}
</div>

<style>
  .marco {
    position: absolute;
    inset: 0;
    touch-action: none;
    overflow: hidden;
  }
  .linea-activa {
    cursor: crosshair;
  }
  .recorte {
    position: absolute;
    border: 1.5px solid #fff;
    box-shadow: 0 0 0 9999px #000a;
    pointer-events: none;
  }
  .tercio {
    position: absolute;
    background: #fff8;
  }
  .tercio.v {
    top: 0;
    bottom: 0;
    width: 1px;
  }
  .tercio.h {
    left: 0;
    right: 0;
    height: 1px;
  }
  .asa {
    position: absolute;
    pointer-events: auto;
  }
  .esquina {
    width: 34px;
    height: 34px;
    border: 0 solid #fff;
  }
  .no {
    left: -5px;
    top: -5px;
    border-left-width: 4px;
    border-top-width: 4px;
  }
  .ne {
    right: -5px;
    top: -5px;
    border-right-width: 4px;
    border-top-width: 4px;
  }
  .so {
    left: -5px;
    bottom: -5px;
    border-left-width: 4px;
    border-bottom-width: 4px;
  }
  .se {
    right: -5px;
    bottom: -5px;
    border-right-width: 4px;
    border-bottom-width: 4px;
  }
  .lado.n,
  .lado.s {
    left: 30%;
    right: 30%;
    height: 30px;
  }
  .lado.n {
    top: -15px;
  }
  .lado.s {
    bottom: -15px;
  }
  .lado.e,
  .lado.o {
    top: 30%;
    bottom: 30%;
    width: 30px;
  }
  .lado.e {
    right: -15px;
  }
  .lado.o {
    left: -15px;
  }
  .lado::after {
    content: '';
    position: absolute;
    background: #fff;
    border-radius: 2px;
  }
  .lado.n::after,
  .lado.s::after {
    left: 50%;
    top: 50%;
    width: 28px;
    height: 4px;
    transform: translate(-50%, -50%);
  }
  .lado.e::after,
  .lado.o::after {
    left: 50%;
    top: 50%;
    width: 4px;
    height: 28px;
    transform: translate(-50%, -50%);
  }
  .trazo {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .trazo line {
    stroke: var(--acento);
    stroke-width: 3;
  }
</style>
