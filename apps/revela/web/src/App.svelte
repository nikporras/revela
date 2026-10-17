<script lang="ts">
  import { onMount } from 'svelte';
  import { pedir } from './lib/motor';
  import type { FormatoExport, InfoFoto } from './lib/protocolo';
  import {
    LIMITES_MP,
    compartirODescargar,
    descargar,
    guardarLimiteMp,
    ladoPrevia,
    leerLimiteMp,
    mb,
    ms,
  } from './lib/plataforma';

  type Estado = 'cargando' | 'vacio' | 'abriendo' | 'lista' | 'exportando';

  interface Diagnostico {
    cargaInicial?: number;
    initWasm?: number;
    desdeCache?: boolean;
    archivo?: { nombre: string; tipo: string; bytes: number };
    abrir?: { leer: number; motor: number; previa: number; total: number };
    exportar?: { ms: number; bytes: number; formato: string; calidad: number };
    /** Memoria del wasm (solo crece) después de abrir+previa y después de exportar. */
    memoria?: { abrir?: number; exportar?: number };
  }

  let estado = $state<Estado>('cargando');
  let error = $state('');
  let aviso = $state('');
  let info = $state<InfoFoto | null>(null);
  let nombreBase = $state('foto');
  let diag = $state<Diagnostico>({});

  let hojaExportar = $state(false);
  let hojaInfo = $state(false);
  let formato = $state<FormatoExport>('jpg');
  let calidad = $state(90);
  let conservarUbicacion = $state(false);
  let limiteMp = $state(leerLimiteMp());
  let exportado = $state<File | null>(null);

  let lienzo: HTMLCanvasElement | undefined = $state();
  let entradaGaleria: HTMLInputElement | undefined = $state();
  let entradaCamara: HTMLInputElement | undefined = $state();

  onMount(async () => {
    try {
      const r = await pedir('iniciar');
      diag.cargaInicial = performance.now();
      diag.initWasm = r.valor.ms;
      diag.desdeCache = !!navigator.serviceWorker?.controller;
      estado = 'vacio';
    } catch (e) {
      error = `No se pudo cargar el motor: ${(e as Error).message}`;
      estado = 'vacio';
    }
  });

  async function abrirArchivo(ev: Event) {
    const entrada = ev.currentTarget as HTMLInputElement;
    const archivo = entrada.files?.[0];
    entrada.value = '';
    if (!archivo) return;
    error = '';
    aviso = '';
    exportado = null;
    estado = 'abriendo';
    const t0 = performance.now();
    try {
      diag.archivo = { nombre: archivo.name, tipo: archivo.type || '(sin tipo)', bytes: archivo.size };
      const bytes = await archivo.arrayBuffer();
      const tLeido = performance.now();
      const lado = ladoPrevia();
      const abierto = await pedir('abrir', archivo.name, bytes, limiteMp, lado);
      const previa = await pedir('previa', lado);
      dibujar(previa.valor);
      info = abierto.valor;
      nombreBase = archivo.name.replace(/\.[^.]+$/, '') || 'foto';
      diag.abrir = {
        leer: tLeido - t0,
        motor: abierto.ms,
        previa: previa.ms,
        total: performance.now() - t0,
      };
      diag.memoria = { abrir: previa.memoria };
      if (info.originalWidth !== info.width) {
        aviso = `Foto reducida de ${mpx(info.originalWidth, info.originalHeight)} a ${mpx(info.width, info.height)} (límite ${limiteMp} MP).`;
      }
      estado = 'lista';
    } catch (e) {
      error = `No se pudo abrir la foto: ${(e as Error).message}`;
      estado = info ? 'lista' : 'vacio';
    }
  }

  function dibujar(p: { width: number; height: number; pixels: Uint8ClampedArray }) {
    if (!lienzo) return;
    lienzo.width = p.width;
    lienzo.height = p.height;
    const ctx = lienzo.getContext('2d');
    ctx?.putImageData(new ImageData(new Uint8ClampedArray(p.pixels), p.width, p.height), 0, 0);
  }

  async function exportar() {
    error = '';
    estado = 'exportando';
    exportado = null;
    try {
      const r = await pedir('exportar', formato, calidad, conservarUbicacion);
      const tipo = formato === 'jpg' ? 'image/jpeg' : 'image/png';
      exportado = new File([r.valor as Uint8Array<ArrayBuffer>], `${nombreBase}-revela.${formato}`, { type: tipo });
      diag.exportar = { ms: r.ms, bytes: r.valor.byteLength, formato, calidad };
      diag.memoria = { ...diag.memoria, exportar: r.memoria };
    } catch (e) {
      error = `No se pudo exportar: ${(e as Error).message}`;
    }
    estado = 'lista';
  }

  async function compartir() {
    if (!exportado) return;
    const r = await compartirODescargar(exportado);
    if (r === 'descargado') aviso = 'Este navegador no puede compartir archivos: se descargó.';
  }

  function cambiarLimite(v: number) {
    limiteMp = v;
    guardarLimiteMp(v);
  }

  function mpx(w: number, h: number) {
    return `${((w * h) / 1e6).toFixed(1)} MP`;
  }

  function textoDiagnostico(): string {
    const d = diag;
    const l: string[] = [`Revela ${__REVELA_VERSION__}`, navigator.userAgent];
    if (d.cargaInicial != null)
      l.push(`Carga inicial: ${ms(d.cargaInicial)} (wasm ${ms(d.initWasm ?? 0)}, ${d.desdeCache ? 'desde caché' : 'desde red'})`);
    if (d.archivo) l.push(`Archivo: ${d.archivo.nombre} · ${d.archivo.tipo} · ${mb(d.archivo.bytes)}`);
    if (info)
      l.push(
        `Foto: ${info.format} · original ${info.originalWidth}×${info.originalHeight} · trabajo ${info.width}×${info.height}`,
      );
    if (d.abrir)
      l.push(
        `Abrir: ${ms(d.abrir.total)} (leer ${ms(d.abrir.leer)}, motor ${ms(d.abrir.motor)}, previa ${ms(d.abrir.previa)})`,
      );
    if (d.exportar)
      l.push(`Exportar ${d.exportar.formato.toUpperCase()} q${d.exportar.calidad}: ${ms(d.exportar.ms)} · ${mb(d.exportar.bytes)}`);
    if (d.memoria?.abrir)
      l.push(
        `Memoria del motor (pico): al abrir ${mb(d.memoria.abrir)}` +
          (d.memoria.exportar ? ` · al exportar ${mb(d.memoria.exportar)}` : ''),
      );
    if (info?.warnings.length) l.push(`Avisos: ${info.warnings.join(' | ')}`);
    return l.join('\n');
  }

  async function copiarDiagnostico() {
    try {
      await navigator.clipboard.writeText(textoDiagnostico());
      aviso = 'Diagnóstico copiado.';
    } catch {
      aviso = 'No se pudo copiar; mantén pulsado el texto para seleccionarlo.';
    }
  }

  const ocupado = $derived(estado === 'cargando' || estado === 'abriendo' || estado === 'exportando');
</script>

<main>
  <header>
    <span class="marca">Revela</span>
    <button class="icono" aria-label="Información y diagnóstico" onclick={() => (hojaInfo = true)}>
      <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <circle cx="12" cy="12" r="9.5" /><path d="M12 11v6" /><circle cx="12" cy="7.5" r="0.6" fill="currentColor" />
      </svg>
    </button>
  </header>

  <section class="foto">
    <canvas bind:this={lienzo} class:oculto={!info}></canvas>
    {#if !info}
      <div class="vacio">
        {#if estado === 'cargando'}
          <p>Cargando el motor…</p>
        {:else}
          <p>Abre una foto para empezar.</p>
          <p class="sub">Todo se procesa en tu teléfono. Nada se sube a internet.</p>
        {/if}
      </div>
    {/if}
    {#if ocupado && estado !== 'cargando'}
      <div class="cargando">{estado === 'abriendo' ? 'Abriendo…' : 'Exportando…'}</div>
    {/if}
  </section>

  {#if error}<p class="mensaje error" role="alert">{error}</p>{/if}
  {#if aviso}<p class="mensaje" role="status">{aviso}</p>{/if}

  <nav class="barra">
    <button class="boton" disabled={ocupado} onclick={() => entradaGaleria?.click()}>Galería</button>
    <button class="boton" disabled={ocupado} onclick={() => entradaCamara?.click()}>Cámara</button>
    <button class="boton principal" disabled={ocupado || !info} onclick={() => (hojaExportar = true)}>Exportar</button>
  </nav>

  <input bind:this={entradaGaleria} type="file" accept="image/*" hidden onchange={abrirArchivo} />
  <input bind:this={entradaCamara} type="file" accept="image/*" capture="environment" hidden onchange={abrirArchivo} />
</main>

{#if hojaExportar}
  <div class="velo" role="presentation" onclick={() => (hojaExportar = false)}></div>
  <section class="hoja" aria-label="Exportar">
    <h2>Exportar</h2>
    <div class="segmentos" role="radiogroup" aria-label="Formato">
      {#each ['jpg', 'png'] as const as f (f)}
        <button role="radio" aria-checked={formato === f} class:activo={formato === f} onclick={() => { formato = f; exportado = null; }}>
          {f.toUpperCase()}
        </button>
      {/each}
    </div>
    {#if formato === 'jpg'}
      <label class="fila">
        <span>Calidad <b>{calidad}</b></span>
        <input type="range" min="40" max="100" step="1" bind:value={calidad} oninput={() => (exportado = null)} />
      </label>
    {/if}
    <label class="fila interruptor">
      <span>Conservar ubicación GPS</span>
      <input type="checkbox" bind:checked={conservarUbicacion} onchange={() => (exportado = null)} />
    </label>
    {#if info}
      <p class="sub">{info.width}×{info.height} px · sin miniatura EXIF{conservarUbicacion ? '' : ' · sin ubicación'}</p>
    {/if}
    {#if exportado}
      <p class="sub">Listo: {exportado.name} · {mb(exportado.size)}</p>
      <div class="acciones">
        <button class="boton" onclick={() => exportado && descargar(exportado)}>Guardar</button>
        <button class="boton principal" onclick={compartir}>Compartir</button>
      </div>
    {:else}
      <div class="acciones">
        <button class="boton principal" disabled={ocupado} onclick={exportar}>
          {estado === 'exportando' ? 'Exportando…' : 'Preparar archivo'}
        </button>
      </div>
    {/if}
  </section>
{/if}

{#if hojaInfo}
  <div class="velo" role="presentation" onclick={() => (hojaInfo = false)}></div>
  <section class="hoja" aria-label="Información">
    <h2>Revela</h2>
    <p>Editor de fotos gratuito. Todo ocurre en tu teléfono.</p>
    <p class="credito">
      Basado en <a href="https://github.com/storytold/photocraft" target="_blank" rel="noopener">PhotoCraft</a> (código abierto, MIT o Apache 2.0).
      <a href="https://github.com/nikporras/revela" target="_blank" rel="noopener">Código de Revela</a>.
    </p>
    <label class="fila">
      <span>Límite de tamaño al abrir</span>
      <select value={limiteMp} onchange={(e) => cambiarLimite(Number((e.currentTarget as HTMLSelectElement).value))}>
        {#each LIMITES_MP as v (v)}<option value={v}>{v} MP</option>{/each}
      </select>
    </label>
    <h3>Diagnóstico</h3>
    <pre class="diag">{textoDiagnostico()}</pre>
    <div class="acciones">
      <button class="boton" onclick={copiarDiagnostico}>Copiar diagnóstico</button>
    </div>
  </section>
{/if}

<style>
  main {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: env(safe-area-inset-top, 0px) 0 env(safe-area-inset-bottom, 0px);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
  }
  .marca {
    font-weight: 700;
    letter-spacing: 0.04em;
  }
  .icono {
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    color: var(--texto-2);
  }
  .foto {
    flex: 1;
    min-height: 0;
    position: relative;
    display: grid;
    place-items: center;
    padding: 8px 16px;
  }
  canvas {
    max-width: 100%;
    max-height: 100%;
    border-radius: 6px;
    image-rendering: auto;
  }
  .oculto {
    display: none;
  }
  .vacio {
    text-align: center;
    color: var(--texto-2);
  }
  .sub {
    color: var(--texto-2);
    font-size: 14px;
  }
  .cargando {
    position: absolute;
    inset: auto 50% 24px auto;
    transform: translateX(50%);
    background: var(--panel-2);
    padding: 8px 16px;
    border-radius: 999px;
  }
  .mensaje {
    margin: 0 16px 8px;
    font-size: 14px;
    color: var(--texto-2);
  }
  .error {
    color: var(--error);
  }
  .barra {
    display: flex;
    gap: 8px;
    padding: 8px 16px 12px;
  }
  .boton {
    flex: 1;
    min-height: var(--toque);
    border-radius: var(--radio);
    background: var(--panel-2);
    font-weight: 600;
  }
  .principal {
    background: var(--acento);
    color: var(--acento-texto);
  }
  .velo {
    position: fixed;
    inset: 0;
    background: #0008;
  }
  .hoja {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    max-height: 85%;
    overflow-y: auto;
    background: var(--panel);
    border-radius: 20px 20px 0 0;
    padding: 16px 16px calc(16px + env(safe-area-inset-bottom, 0px));
  }
  h2 {
    margin: 4px 0 16px;
    font-size: 20px;
  }
  h3 {
    margin: 20px 0 8px;
    font-size: 15px;
    color: var(--texto-2);
  }
  .segmentos {
    display: flex;
    background: var(--panel-2);
    border-radius: var(--radio);
    padding: 4px;
  }
  .segmentos button {
    flex: 1;
    min-height: 44px;
    border-radius: 10px;
    font-weight: 600;
  }
  .segmentos .activo {
    background: var(--borde);
  }
  .fila {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 16px 0;
  }
  .interruptor {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }
  .interruptor input {
    width: 24px;
    height: 24px;
    accent-color: var(--acento);
  }
  input[type='range'] {
    width: 100%;
    height: 32px;
    accent-color: var(--acento);
  }
  select {
    font: inherit;
    min-height: 44px;
    background: var(--panel-2);
    color: var(--texto);
    border: 1px solid var(--borde);
    border-radius: 10px;
    padding: 0 12px;
  }
  .acciones {
    display: flex;
    gap: 8px;
    margin-top: 16px;
  }
  .credito {
    font-size: 14px;
    color: var(--texto-2);
  }
  .credito a {
    color: var(--acento);
  }
  .diag {
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 12px;
    background: var(--fondo);
    padding: 12px;
    border-radius: 10px;
    user-select: text;
    -webkit-user-select: text;
  }
</style>
