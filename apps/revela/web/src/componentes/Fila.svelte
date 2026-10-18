<script lang="ts">
  // Un slider compacto en una sola línea: nombre, barra y valor. Para paneles con varios
  // sliders a la vista (mezcla de color, gradación).
  let {
    nombre,
    valor,
    min,
    max,
    paso = 1,
    /** Fondo de la barra (un degradado que muestra lo que hace); por defecto, una línea. */
    pista = '',
    /** Dónde está el valor neutro (marca en la barra); `null`: sin marca. */
    neutro = 0,
    formato,
    oncambio,
    onfin,
  }: {
    nombre: string;
    valor: number;
    min: number;
    max: number;
    paso?: number;
    pista?: string;
    neutro?: number | null;
    formato?: (v: number) => string;
    oncambio: (v: number) => void;
    onfin: () => void;
  } = $props();

  const texto = $derived(formato ? formato(valor) : `${valor > 0 && min < 0 ? '+' : ''}${Math.round(valor)}`);
  const marca = $derived(neutro === null ? null : ((neutro - min) / (max - min)) * 100);
</script>

<label class="fila">
  <span class="nombre">{nombre}</span>
  <span class="barra" class:con-pista={!!pista} style:--pista={pista || null}>
    {#if marca !== null}<span class="marca" style:left="{marca}%"></span>{/if}
    <input
      type="range"
      aria-label={nombre}
      {min}
      {max}
      step={paso}
      value={valor}
      oninput={(e) => oncambio(Number((e.currentTarget as HTMLInputElement).value))}
      onchange={onfin}
    />
  </span>
  <b class:cambiado={neutro !== null && valor !== neutro}>{texto}</b>
</label>

<style>
  .fila {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
  }
  .nombre {
    flex: none;
    width: 84px;
    font-size: 14px;
    color: var(--texto-2);
  }
  .barra {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
    height: 40px;
  }
  .barra::before {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    height: 4px;
    border-radius: 2px;
    background: var(--borde);
  }
  .con-pista::before {
    height: 6px;
    border-radius: 3px;
    background: var(--pista);
  }
  .marca {
    position: absolute;
    width: 2px;
    height: 12px;
    margin-left: -1px;
    background: var(--texto-2);
    border-radius: 1px;
  }
  input[type='range'] {
    position: relative;
    width: 100%;
    height: 40px;
    margin: 0;
    background: transparent;
    -webkit-appearance: none;
    appearance: none;
  }
  input[type='range']::-webkit-slider-runnable-track {
    height: 4px;
    background: transparent;
  }
  input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 24px;
    height: 24px;
    margin-top: -10px;
    border-radius: 50%;
    background: var(--texto);
    border: 3px solid var(--fondo);
    box-shadow: 0 0 0 1px var(--borde);
  }
  input[type='range']::-moz-range-track {
    background: transparent;
  }
  input[type='range']::-moz-range-thumb {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--texto);
  }
  b {
    flex: none;
    width: 44px;
    text-align: right;
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    color: var(--texto-2);
  }
  .cambiado {
    color: var(--texto);
  }
</style>
