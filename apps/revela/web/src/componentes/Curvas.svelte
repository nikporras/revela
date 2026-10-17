<script lang="ts">
  import { DIAGONAL, esCurvaNeutra, type Canal, type Punto } from '../lib/receta';

  let {
    curvas,
    luts,
    oncambio,
    onfin,
  }: {
    curvas: Record<Canal, Punto[]>;
    /** Las curvas tal como las calcula el motor (las mismas que usa el shader), para dibujarlas. */
    luts: Float32Array | null;
    oncambio: (canal: Canal, puntos: Punto[]) => void;
    onfin: () => void;
  } = $props();

  const CANALES: { id: Canal; nombre: string; color: string; lut: number }[] = [
    { id: 'master', nombre: 'Luz', color: 'var(--texto)', lut: 1 },
    { id: 'red', nombre: 'Rojo', color: '#ff6b6b', lut: 2 },
    { id: 'green', nombre: 'Verde', color: '#5fd38d', lut: 3 },
    { id: 'blue', nombre: 'Azul', color: '#6aa8ff', lut: 4 },
  ];
  const MAX_PUNTOS = 16;
  const RADIO_TOQUE = 22; // px de pantalla

  let canal = $state<Canal>('master');
  let svg: SVGSVGElement | undefined = $state();
  let arrastrando = $state<number | null>(null);
  let fuera = $state(false);

  const def = $derived(CANALES.find((c) => c.id === canal) ?? CANALES[0]);
  const puntos = $derived(curvas[canal].length >= 2 ? curvas[canal] : DIAGONAL);

  /** Trazo de la curva: de la tabla del motor si la hay, si no, segmentos entre los puntos. */
  const trazo = $derived.by(() => {
    const n = 1024;
    if (luts && luts.length >= 5 * n && !esCurvaNeutra(puntos)) {
      const base = def.lut * n;
      const pts: string[] = [];
      for (let i = 0; i < n; i += 8) pts.push(`${(i / (n - 1)) * 255},${255 - luts[base + i] * 255}`);
      pts.push(`255,${255 - luts[base + n - 1] * 255}`);
      return pts.join(' ');
    }
    return puntos.map(([x, y]) => `${x},${255 - y}`).join(' ');
  });

  function aDatos(e: PointerEvent): { x: number; y: number; escala: number } {
    const r = svg!.getBoundingClientRect();
    const escala = r.width / 256;
    return { x: (e.clientX - r.left) / escala, y: 255 - (e.clientY - r.top) / escala, escala };
  }

  function bajar(e: PointerEvent) {
    if (!svg) return;
    svg.setPointerCapture(e.pointerId);
    const { x, y, escala } = aDatos(e);
    const radio = RADIO_TOQUE / escala;
    let cerca = -1;
    let mejor = Infinity;
    puntos.forEach(([px, py], i) => {
      const d = Math.hypot(px - x, py - y);
      if (d < radio && d < mejor) {
        mejor = d;
        cerca = i;
      }
    });
    if (cerca >= 0) {
      arrastrando = cerca;
      return;
    }
    // Un punto nuevo, si cabe.
    const xi = Math.round(Math.min(254, Math.max(1, x)));
    if (puntos.length >= MAX_PUNTOS || puntos.some(([px]) => Math.abs(px - xi) < 2)) return;
    const nuevos = [...puntos, [xi, clamp(Math.round(y))] as Punto].sort((a, b) => a[0] - b[0]);
    arrastrando = nuevos.findIndex(([px]) => px === xi);
    oncambio(canal, nuevos);
  }

  function mover(e: PointerEvent) {
    if (arrastrando === null) return;
    const { x, y } = aDatos(e);
    const i = arrastrando;
    const extremo = i === 0 || i === puntos.length - 1;
    // Sacar un punto interior del recuadro lo borra (se ve marcado mientras tanto).
    fuera = !extremo && (y < -24 || y > 279 || x < -24 || x > 279);
    const p = [...puntos];
    const minX = i === 0 ? 0 : p[i - 1][0] + 1;
    const maxX = i === p.length - 1 ? 255 : p[i + 1][0] - 1;
    const nx = extremo ? p[i][0] : Math.round(Math.min(maxX, Math.max(minX, x)));
    p[i] = [nx, clamp(Math.round(y))];
    oncambio(canal, p);
  }

  function soltar() {
    if (arrastrando === null) return;
    if (fuera) oncambio(canal, puntos.filter((_, i) => i !== arrastrando));
    arrastrando = null;
    fuera = false;
    onfin();
  }

  function clamp(v: number) {
    return Math.min(255, Math.max(0, v));
  }

  function restablecer() {
    oncambio(canal, DIAGONAL);
    onfin();
  }
</script>

<div class="canales" role="tablist" aria-label="Canal">
  {#each CANALES as c (c.id)}
    <button role="tab" aria-selected={c.id === canal} class:activo={c.id === canal} onclick={() => (canal = c.id)}>
      <span class="muestra" style="background: {c.color}"></span>{c.nombre}{#if !esCurvaNeutra(curvas[c.id])}<span class="punto"></span>{/if}
    </button>
  {/each}
</div>

<div class="caja">
  <svg
    bind:this={svg}
    viewBox="-6 -6 268 268"
    role="application"
    aria-label="Curva {def.nombre}: toca para añadir un punto, arrastra para moverlo, sácalo del recuadro para borrarlo"
    onpointerdown={bajar}
    onpointermove={mover}
    onpointerup={soltar}
    onpointercancel={soltar}
  >
    <rect x="0" y="0" width="255" height="255" class="fondo" />
    {#each [64, 128, 192] as g (g)}
      <line x1={g} y1="0" x2={g} y2="255" class="rejilla" />
      <line x1="0" y1={g} x2="255" y2={g} class="rejilla" />
    {/each}
    <line x1="0" y1="255" x2="255" y2="0" class="diagonal" />
    <polyline points={trazo} fill="none" stroke={def.color} stroke-width="2.5" />
    {#each puntos as [x, y], i (i)}
      <circle cx={x} cy={255 - y} r={arrastrando === i ? 9 : 7} class:borrar={arrastrando === i && fuera} fill={def.color} />
    {/each}
  </svg>
</div>
<div class="pie">
  <p class="ayuda">Toca para añadir · arrastra para mover · saca un punto del recuadro para borrarlo</p>
  <button class="cero" disabled={esCurvaNeutra(puntos)} onclick={restablecer}>Restablecer</button>
</div>

<style>
  .canales {
    display: flex;
    gap: 8px;
    padding: 4px 16px 8px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .canales button {
    flex: none;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 999px;
    background: var(--panel-2);
    color: var(--texto-2);
    display: flex;
    align-items: center;
    gap: 6px;
    position: relative;
  }
  .canales .activo {
    background: var(--texto);
    color: var(--fondo);
    font-weight: 600;
  }
  .muestra {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .punto {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--acento);
  }
  .pie {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 16px;
  }
  .cero {
    flex: none;
    color: var(--acento);
    min-height: 36px;
  }
  .caja {
    display: flex;
    justify-content: center;
    padding: 0 16px;
  }
  svg {
    width: min(100%, 240px);
    aspect-ratio: 1;
    touch-action: none;
    user-select: none;
  }
  .fondo {
    fill: var(--fondo);
    stroke: var(--borde);
  }
  .rejilla {
    stroke: var(--borde);
    stroke-width: 0.6;
  }
  .diagonal {
    stroke: var(--borde);
    stroke-dasharray: 4 4;
  }
  circle {
    stroke: var(--fondo);
    stroke-width: 2;
  }
  .borrar {
    opacity: 0.35;
  }
  .ayuda {
    flex: 1;
    color: var(--texto-2);
    font-size: 12px;
    margin: 6px 0 0;
  }
</style>
