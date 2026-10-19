<script lang="ts">
  // Viñeta (Efectos): oscurece o aclara los bordes del encuadre final, como en Lightroom.
  import Fila from './Fila.svelte';
  import { VINETA_DEFECTO, type Vineta } from '../lib/receta';

  let {
    vineta,
    oncambio,
    onfin,
  }: {
    vineta: Vineta;
    oncambio: (campo: keyof Vineta, v: number) => void;
    onfin: () => void;
  } = $props();

  const tocada = $derived((Object.keys(VINETA_DEFECTO) as (keyof Vineta)[]).some((k) => vineta[k] !== VINETA_DEFECTO[k]));

  function restablecer() {
    for (const k of Object.keys(VINETA_DEFECTO) as (keyof Vineta)[]) oncambio(k, VINETA_DEFECTO[k]);
    onfin();
  }
</script>

<div class="cabecera">
  <span>Viñeta</span>
  <button class="cero" disabled={!tocada} onclick={restablecer}>Restablecer</button>
</div>
<div class="filas">
  <Fila
    nombre="Cantidad"
    valor={vineta.cantidad}
    min={-100}
    max={100}
    pista="linear-gradient(to right, #111, #777 50%, #eee)"
    oncambio={(v) => oncambio('cantidad', v)}
    {onfin}
  />
  <!-- La forma solo se nota con cantidad. -->
  <div class="forma" class:apagada={vineta.cantidad === 0}>
    <Fila nombre="Punto medio" valor={vineta.punto} min={0} max={100} neutro={VINETA_DEFECTO.punto} formato={(v) => String(Math.round(v))} oncambio={(v) => oncambio('punto', v)} {onfin} />
    <Fila nombre="Redondez" valor={vineta.redondez} min={-100} max={100} oncambio={(v) => oncambio('redondez', v)} {onfin} />
    <Fila nombre="Difuminado" valor={vineta.difuminado} min={0} max={100} neutro={VINETA_DEFECTO.difuminado} formato={(v) => String(Math.round(v))} oncambio={(v) => oncambio('difuminado', v)} {onfin} />
    <!-- Solo actúa si la viñeta oscurece. -->
    <div class:apagada={vineta.cantidad > 0}>
      <Fila nombre="Luces" valor={vineta.luces} min={0} max={100} formato={(v) => String(Math.round(v))} oncambio={(v) => oncambio('luces', v)} {onfin} />
    </div>
  </div>
</div>

<style>
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
  .apagada {
    opacity: 0.45;
  }
</style>
