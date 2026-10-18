<script lang="ts">
  // Gradación de color: una rueda por zona (sombras, medios, luces, global). El ángulo es el
  // tono y la distancia al centro la saturación; debajo, la luminancia de la zona y la mezcla y
  // el balance entre zonas.
  import Fila from './Fila.svelte';
  import { MEZCLA_DEFECTO, esRuedaNeutra, type Gradacion, type IdRueda, type Rueda } from '../lib/receta';

  let {
    gradacion,
    onrueda,
    onmezcla,
    onbalance,
    onfin,
  }: {
    gradacion: Gradacion;
    onrueda: (id: IdRueda, r: Rueda) => void;
    onmezcla: (v: number) => void;
    onbalance: (v: number) => void;
    onfin: () => void;
  } = $props();

  const ZONAS: { id: IdRueda; nombre: string }[] = [
    { id: 'sombras', nombre: 'Sombras' },
    { id: 'medios', nombre: 'Medios' },
    { id: 'luces', nombre: 'Luces' },
    { id: 'global', nombre: 'Global' },
  ];

  let zona = $state<IdRueda>('sombras');
  const r = $derived(gradacion.ruedas[zona]);
  let rueda: HTMLDivElement | undefined = $state();
  let arrastrando = false;

  /** Posición del punto en la rueda (fracciones desde el centro; y hacia arriba). */
  const px = $derived((r.sat / 100) * Math.cos((r.tono * Math.PI) / 180));
  const py = $derived((r.sat / 100) * Math.sin((r.tono * Math.PI) / 180));

  function color(x: Rueda): string {
    return x.sat === 0 ? 'var(--panel-2)' : `hsl(${x.tono} ${Math.max(25, x.sat)}% 55%)`;
  }

  function mover(e: PointerEvent) {
    const b = rueda!.getBoundingClientRect();
    const dx = (e.clientX - b.left) / (b.width / 2) - 1;
    const dy = 1 - (e.clientY - b.top) / (b.height / 2);
    const sat = Math.min(1, Math.hypot(dx, dy));
    // Cerca del centro, sin color (es fácil volver a cero).
    if (sat < 0.06) return onrueda(zona, { ...r, sat: 0 });
    const tono = ((Math.atan2(dy, dx) * 180) / Math.PI + 360) % 360;
    onrueda(zona, { ...r, tono: Math.round(tono), sat: Math.round(sat * 100) });
  }

  function bajar(e: PointerEvent) {
    rueda?.setPointerCapture(e.pointerId);
    arrastrando = true;
    mover(e);
  }

  function soltar() {
    if (!arrastrando) return;
    arrastrando = false;
    onfin();
  }

  function restablecer() {
    onrueda(zona, { tono: 0, sat: 0, lum: 0 });
    onfin();
  }
</script>

<div class="zonas" role="radiogroup" aria-label="Zona">
  {#each ZONAS as z (z.id)}
    <button role="radio" aria-checked={zona === z.id} class:activo={zona === z.id} onclick={() => (zona = z.id)}>
      <span class="muestra" style:background={color(gradacion.ruedas[z.id])}></span>{z.nombre}
    </button>
  {/each}
</div>

<div class="cuerpo">
  <div
    bind:this={rueda}
    class="rueda"
    role="slider"
    tabindex="0"
    aria-label="Tono y saturación de {ZONAS.find((z) => z.id === zona)?.nombre}"
    aria-valuetext="tono {r.tono}°, saturación {r.sat}"
    aria-valuenow={r.sat}
    onpointerdown={bajar}
    onpointermove={(e) => arrastrando && mover(e)}
    onpointerup={soltar}
    onpointercancel={soltar}
  >
    <span class="punto" style:left="{50 + px * 50}%" style:top="{50 - py * 50}%" style:background={color(r)}></span>
  </div>
  <div class="lado">
    <div class="valores">
      <span>{r.sat === 0 ? 'Sin color' : `${r.tono}° · ${r.sat}`}</span>
      <button class="cero" disabled={esRuedaNeutra(r)} onclick={restablecer}>Restablecer</button>
    </div>
    <Fila nombre="Luminancia" valor={r.lum} min={-100} max={100} oncambio={(v) => onrueda(zona, { ...r, lum: v })} {onfin} />
    <Fila nombre="Mezcla" valor={gradacion.mezcla} min={0} max={100} neutro={MEZCLA_DEFECTO} formato={(v) => String(Math.round(v))} oncambio={onmezcla} {onfin} />
    <Fila nombre="Balance" valor={gradacion.balance} min={-100} max={100} oncambio={onbalance} {onfin} />
  </div>
</div>

<style>
  .zonas {
    display: flex;
    gap: 6px;
    padding: 2px 16px 6px;
  }
  .zonas button {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 34px;
    border-radius: 999px;
    background: var(--panel-2);
    color: var(--texto-2);
    font-size: 13px;
  }
  .zonas .activo {
    background: var(--texto);
    color: var(--fondo);
    font-weight: 600;
  }
  .muestra {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: 0 0 0 1px var(--borde);
  }
  .cuerpo {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 0 16px;
  }
  .rueda {
    flex: none;
    position: relative;
    width: 116px;
    height: 116px;
    border-radius: 50%;
    /* Tono 0 (rojo) a la derecha, creciendo en sentido contrario al reloj, como el motor. */
    background:
      radial-gradient(closest-side, #8a8a8a 0%, #8a8a8a00 100%),
      conic-gradient(from 90deg, #f00, #f0f, #00f, #0ff, #0f0, #ff0, #f00);
    touch-action: none;
    box-shadow: 0 0 0 1px var(--borde);
  }
  .rueda::after {
    /* El centro: sin color. */
    content: '';
    position: absolute;
    left: 50%;
    top: 50%;
    width: 6px;
    height: 6px;
    margin: -3px;
    border-radius: 50%;
    background: #fff8;
  }
  .punto {
    position: absolute;
    width: 22px;
    height: 22px;
    margin: -11px;
    border-radius: 50%;
    border: 3px solid #fff;
    box-shadow: 0 1px 4px #0008;
    pointer-events: none;
  }
  .lado {
    flex: 1;
    min-width: 0;
  }
  /* Al lado de la rueda hay menos sitio: nombres y valores más estrechos. */
  .lado :global(.nombre) {
    width: 82px;
    font-size: 13px;
  }
  .lado :global(.fila b) {
    width: 32px;
  }
  .lado :global(.fila) {
    gap: 6px;
  }
  .valores {
    display: flex;
    align-items: center;
    font-size: 13px;
    color: var(--texto-2);
    font-variant-numeric: tabular-nums;
  }
  .cero {
    margin-left: auto;
    color: var(--acento);
    font-size: 14px;
    min-height: 32px;
  }
</style>
