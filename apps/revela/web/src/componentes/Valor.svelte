<script lang="ts">
  // El número de un slider: al tocarlo se puede escribir el valor exacto (Intro o tocar fuera
  // lo aplica, Esc lo cancela). El teclado decimal de Android no siempre tiene "−": para los
  // valores que pueden ser negativos hay un botón ±.
  let {
    texto,
    valor,
    min,
    max,
    paso = 1,
    nombre,
    cambiado = false,
    oncambio,
    onfin,
  }: {
    /** Cómo se ve el valor ("+0.50 EV"). */
    texto: string;
    valor: number;
    min: number;
    max: number;
    paso?: number;
    /** Para el lector de pantalla ("Exposición"). */
    nombre: string;
    cambiado?: boolean;
    oncambio: (v: number) => void;
    onfin: () => void;
  } = $props();

  let editando = $state(false);
  let entrada: HTMLInputElement | undefined = $state();

  function empezar() {
    editando = true;
    queueMicrotask(() => {
      entrada?.focus();
      entrada?.select();
    });
  }

  function aplicar() {
    if (!editando || !entrada) return;
    editando = false;
    const v = Number(entrada.value.replace(',', '.').replace('−', '-').trim());
    if (!entrada.value.trim() || !Number.isFinite(v)) return;
    // Dentro de los límites y al paso del slider.
    const decimales = (String(paso).split('.')[1] ?? '').length;
    const c = Number((Math.round((Math.min(max, Math.max(min, v)) - min) / paso) * paso + min).toFixed(decimales));
    if (c !== valor) {
      oncambio(c);
      onfin();
    }
  }

  function signo(e: MouseEvent) {
    e.preventDefault();
    if (!entrada) return;
    const t = entrada.value.trim();
    entrada.value = t.startsWith('-') ? t.slice(1) : `-${t}`;
    entrada.focus();
  }
</script>

{#if editando}
  <span class="editor">
    {#if min < 0}
      <!-- mousedown: que el campo no pierda el foco (y se aplique) al tocarlo. -->
      <button class="signo" aria-label="Cambiar el signo" onmousedown={signo}>±</button>
    {/if}
    <input
      bind:this={entrada}
      type="text"
      inputmode="decimal"
      enterkeyhint="done"
      aria-label="{nombre}: escribe el valor"
      value={String(valor)}
      onkeydown={(e) => {
        if (e.key === 'Enter') aplicar();
        if (e.key === 'Escape') editando = false;
      }}
      onblur={aplicar}
    />
  </span>
{:else}
  <button class="valor" class:cambiado aria-label="{nombre}: {texto}. Toca para escribir el valor" onclick={empezar}>{texto}</button>
{/if}

<style>
  .valor {
    flex: none;
    min-width: 44px;
    min-height: 36px;
    padding: 0 2px;
    text-align: right;
    font: inherit;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: inherit;
    border-bottom: 1px dashed transparent;
  }
  .cambiado {
    color: var(--texto);
  }
  .valor:active {
    border-bottom-color: var(--texto-2);
  }
  .editor {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  input {
    width: 64px;
    min-height: 34px;
    padding: 0 6px;
    border-radius: 8px;
    border: 1px solid var(--acento);
    background: var(--panel-2);
    color: var(--texto);
    font: inherit;
    font-weight: 700;
    text-align: right;
  }
  .signo {
    min-width: 34px;
    min-height: 34px;
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--texto);
    font-weight: 700;
  }
</style>
