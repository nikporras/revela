<script lang="ts">
  import { onMount } from 'svelte';
  import Sliders from './componentes/Sliders.svelte';
  import Curvas from './componentes/Curvas.svelte';
  import Filtros from './componentes/Filtros.svelte';
  import { Motor, pedir, ultimaGana } from './lib/motor';
  import { carpetaRecordada, guardarEnCarpeta, nombreBase as elegirNombreBase, soportaCarpeta } from './lib/guardar';
  import { Revelador, hayWebGL2, type ParametrosGpu } from './lib/gpu';
  import { FILTROS, recetasDeFiltros, type Filtro } from './lib/filtros';
  import { aCameraRaw, esVacia, recetaVacia, type Canal, type IdSlider, type Punto, type Receta } from './lib/receta';
  import { probar, recetasDePrueba, TOLERANCIA, type MotorDePrueba } from './lib/autoprueba';
  import type { FormatoExport, InfoFoto, Previa } from './lib/protocolo';
  import {
    LIMITES_MP,
    compartir as compartirArchivo,
    puedeCompartir,
    descargar,
    guardarLimiteMp,
    ladoPrevia,
    leerLimiteMp,
    mb,
    ms,
  } from './lib/plataforma';

  type Estado = 'cargando' | 'vacio' | 'abriendo' | 'lista' | 'exportando';
  type Pestana = 'filtros' | 'ajustes' | 'curvas';

  interface Diagnostico {
    cargaInicial?: number;
    initWasm?: number;
    desdeCache?: boolean;
    archivo?: { nombre: string; tipo: string; bytes: number; origen: string };
    abrir?: { leer: number; motor: number; previa: number; total: number };
    exportar?: { ms: number; bytes: number; formato: string; calidad: number };
    /** Memoria del wasm (solo crece) después de abrir+previa y después de exportar. */
    memoria?: { abrir?: number; exportar?: number };
    compartir?: string;
    guardar?: string;
    /** Tiempo por cambio de slider (desde que se mueve hasta que está dibujado), últimos 20. */
    vivo?: number[];
    gpu?: string;
    autoprueba?: string;
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
  /** Resultado de Compartir/Guardar, visible dentro de la hoja de exportar. */
  let avisoHoja = $state('');
  const sePuedeCompartir = $derived(exportado ? puedeCompartir(exportado) : false);

  // --- Receta, historial y previa ---------------------------------------------------------

  let receta = $state<Receta>(recetaVacia());
  let historial = $state<Receta[]>([recetaVacia()]);
  let posicion = $state(0);
  let pestana = $state<Pestana>('ajustes');
  let comparando = $state(false);
  /** Curvas calculadas por el motor (para dibujarlas en el editor). */
  let luts = $state<Float32Array | null>(null);
  let claveFoto = $state(0);

  let lienzoGpu: HTMLCanvasElement | undefined = $state();
  let lienzo2d: HTMLCanvasElement | undefined = $state();
  let entradaGaleria: HTMLInputElement | undefined = $state();
  let entradaCamara: HTMLInputElement | undefined = $state();

  /** WebGL2 disponible: previa en la GPU. Si no, la previa la calcula el motor (más lenta). */
  let usarGpu = $state(hayWebGL2());
  let revelador: Revelador | null = null;
  let ultimosParams: ParametrosGpu | null = null;
  /** Previa sin ajustes (para "mantener pulsado para comparar" sin GPU). */
  let base: Previa | null = null;
  /** Sin GPU la previa del motor se calcula más pequeña para que el slider responda. */
  const LADO_CPU = 900;

  const ajustesJson = () => JSON.stringify(aCameraRaw($state.snapshot(receta) as Receta, FILTROS));

  let tCambio = 0;
  function medirVivo() {
    if (!tCambio) return;
    const v = [...(diag.vivo ?? []), performance.now() - tCambio].slice(-20);
    diag.vivo = v;
    tCambio = 0;
  }

  const actualizarGpu = ultimaGana(
    async (json: string) => (await pedir('gpuParams', json)).valor,
    (p) => {
      ultimosParams = p;
      luts = p.luts;
      if (!comparando) revelador?.dibujar(p);
      medirVivo();
    },
    (e) => (error = `No se pudo aplicar el ajuste: ${e.message}`),
  );

  const actualizarCpu = ultimaGana(
    async (json: string) => (await pedir('previaMotor', LADO_CPU, json)).valor,
    (p) => {
      if (!comparando) pintar2d(p);
      medirVivo();
    },
    (e) => (error = `No se pudo aplicar el ajuste: ${e.message}`),
  );

  function aplicar() {
    if (!info) return;
    tCambio = performance.now();
    const json = ajustesJson();
    if (usarGpu) actualizarGpu(json);
    else actualizarCpu(json);
  }

  function pintar2d(p: Previa) {
    if (!lienzo2d) return;
    lienzo2d.width = p.width;
    lienzo2d.height = p.height;
    lienzo2d.getContext('2d')?.putImageData(new ImageData(new Uint8ClampedArray(p.pixels), p.width, p.height), 0, 0);
  }

  /** Un paso de deshacer (al soltar un slider, elegir un filtro…). */
  function confirmar() {
    const actual = $state.snapshot(receta) as Receta;
    if (JSON.stringify(actual) === JSON.stringify(historial[posicion])) return;
    historial = [...historial.slice(0, posicion + 1), actual].slice(-100);
    posicion = historial.length - 1;
    exportado = null;
  }

  function irA(p: number) {
    if (p < 0 || p >= historial.length) return;
    posicion = p;
    receta = structuredClone($state.snapshot(historial[p]) as Receta);
    exportado = null;
    aplicar();
  }

  function cambiarSlider(id: IdSlider, v: number) {
    receta.sliders[id] = v;
    aplicar();
  }

  function cambiarCurva(canal: Canal, puntos: Punto[]) {
    receta.curvas[canal] = puntos;
    aplicar();
  }

  function cambiarFiltro(f: { id: string; intensidad: number } | null) {
    receta.filtro = f;
    aplicar();
  }

  function empezarDeNuevo() {
    receta = recetaVacia();
    confirmar();
    aplicar();
  }

  // Mantener pulsado para comparar con el original.
  let temporizador: ReturnType<typeof setTimeout> | undefined;
  function pulsar() {
    if (!info) return;
    clearTimeout(temporizador);
    temporizador = setTimeout(() => {
      comparando = true;
      if (usarGpu) revelador?.dibujar(null);
      else if (base) pintar2d(base);
    }, 150);
  }
  function soltar() {
    clearTimeout(temporizador);
    if (!comparando) return;
    comparando = false;
    if (usarGpu) revelador?.dibujar(ultimosParams);
    else aplicar();
  }

  // Miniaturas de los filtros: la foto pequeña con cada filtro, en un revelador aparte.
  let mini: Revelador | null = null;
  let baseMini = $state.raw<Previa | null>(null);
  async function dibujarMiniatura(lienzo: HTMLCanvasElement, f: Filtro | null) {
    if (!baseMini) return;
    const json = JSON.stringify(f ? f.ajustes : {});
    let px: Uint8Array | Uint8ClampedArray;
    let w = baseMini.width;
    let h = baseMini.height;
    if (mini) {
      mini.dibujar((await pedir('gpuParams', json)).valor);
      px = mini.leer();
    } else {
      const p = (await pedir('previaMotor', 160, json)).valor;
      ({ width: w, height: h, pixels: px } = p);
    }
    lienzo.width = w;
    lienzo.height = h;
    lienzo.getContext('2d')?.putImageData(new ImageData(new Uint8ClampedArray(px), w, h), 0, 0);
  }

  onMount(async () => {
    if (usarGpu && lienzoGpu) {
      try {
        revelador = new Revelador(lienzoGpu);
        diag.gpu = revelador.descripcion();
        mini = new Revelador(document.createElement('canvas'));
      } catch (e) {
        usarGpu = false;
        diag.gpu = `sin GPU: ${(e as Error).message}`;
      }
    } else {
      diag.gpu = 'sin WebGL2: la previa la calcula el motor';
    }
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
      diag.archivo = { nombre: archivo.name, tipo: archivo.type || '(sin tipo)', bytes: archivo.size, origen: entrada.hasAttribute('capture') ? 'botón Cámara' : 'botón Galería' };
      const bytes = await archivo.arrayBuffer();
      const tLeido = performance.now();
      const lado = ladoPrevia();
      const abierto = await pedir('abrir', archivo.name, bytes, limiteMp, lado);
      const previa = await pedir('previa', usarGpu ? lado : LADO_CPU);
      base = previa.valor;
      if (usarGpu) revelador?.cargar(base.width, base.height, base.pixels);
      baseMini = (await pedir('previa', 160)).valor;
      mini?.cargar(baseMini.width, baseMini.height, baseMini.pixels);
      info = abierto.valor;
      nombreBase = elegirNombreBase(archivo.name, info.takenAt);
      // Foto nueva: receta e historial nuevos.
      receta = recetaVacia();
      historial = [recetaVacia()];
      posicion = 0;
      claveFoto++;
      ultimosParams = null;
      if (usarGpu) revelador?.dibujar(null);
      else pintar2d(base);
      diag.abrir = { leer: tLeido - t0, motor: abierto.ms, previa: previa.ms, total: performance.now() - t0 };
      diag.memoria = { abrir: previa.memoria };
      diag.vivo = [];
      if (info.originalWidth !== info.width) {
        aviso = `Foto reducida de ${mpx(info.originalWidth, info.originalHeight)} a ${mpx(info.width, info.height)} (límite ${limiteMp} MP).`;
      }
      estado = 'lista';
    } catch (e) {
      error = `No se pudo abrir la foto: ${(e as Error).message}`;
      estado = info ? 'lista' : 'vacio';
    }
  }

  async function exportar() {
    error = '';
    estado = 'exportando';
    exportado = null;
    avisoHoja = '';
    try {
      const r = await pedir('exportar', formato, calidad, conservarUbicacion, ajustesJson());
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
    avisoHoja = '';
    const r = await compartirArchivo(exportado);
    diag.compartir = r.estado === 'error' ? `error (${r.detalle})` : r.estado;
    if (r.estado === 'no-soportado') avisoHoja = 'Este navegador no puede compartir archivos. Usa Guardar.';
    else if (r.estado === 'error') avisoHoja = `No se pudo compartir: ${r.detalle}`;
  }

  /** Carpeta donde guarda "Guardar" (si el navegador permite elegirla). */
  let carpeta = $state<string | undefined>();
  onMount(async () => {
    if (soportaCarpeta) carpeta = await carpetaRecordada();
  });

  async function guardar(elegir = false) {
    if (!exportado) return;
    avisoHoja = '';
    if (!soportaCarpeta) {
      descargar(exportado);
      avisoHoja = `Guardado en Descargas: ${exportado.name}`;
      return;
    }
    const r = await guardarEnCarpeta(exportado, elegir);
    diag.guardar = r.estado === 'error' ? `error (${r.detalle})` : r.estado;
    if (r.estado === 'guardado') {
      carpeta = r.carpeta;
      avisoHoja = `Guardado en ${r.carpeta}: ${r.nombre}`;
    } else if (r.estado === 'error') {
      // Sin carpeta posible, se descarga para no perder la foto.
      descargar(exportado);
      avisoHoja = `No se pudo guardar en la carpeta (${r.detalle}). Se guardó en Descargas.`;
    }
  }

  // --- Autoprueba de la GPU en el propio teléfono ------------------------------------------

  let probando = $state('');
  async function probarGpu() {
    if (!usarGpu) {
      diag.autoprueba = 'sin WebGL2: la previa y la exportación usan el motor (son iguales por construcción)';
      return;
    }
    // Un motor aparte, para no tocar la foto que se está editando.
    const m = new Motor('motor-autoprueba');
    const motorPrueba: MotorDePrueba = {
      abrir: async (nombre, bytes) => void (await m.pedir('abrir', nombre, bytes.slice().buffer, 100, 512)),
      previa: async (lado) => (await m.pedir('previa', lado)).valor,
      previaMotor: async (lado, json) => (await m.pedir('previaMotor', lado, json)).valor,
      gpuParams: async (json) => (await m.pedir('gpuParams', json)).valor,
    };
    try {
      probando = 'Preparando…';
      const rev = new Revelador(document.createElement('canvas'));
      const inf = await probar(motorPrueba, rev, recetasDePrueba(recetasDeFiltros()), (t) => (probando = t));
      diag.autoprueba =
        `${inf.ok ? '✓ igual' : '✗ DISTINTA'}: máx ${inf.peor.max}/255 (límite ${TOLERANCIA}) en ${inf.resultados.length} comparaciones` +
        `${inf.bandas ? ' · ✗ BANDAS en degradados' : ' · sin bandas'} · ${ms(inf.ms)}` +
        (inf.ok ? '' : ` · peor: ${inf.peor.imagen}, ${inf.peor.receta}`);
    } catch (e) {
      diag.autoprueba = `error: ${(e as Error).message}`;
    } finally {
      probando = '';
      m.reiniciar();
    }
  }

  function cambiarLimite(v: number) {
    limiteMp = v;
    guardarLimiteMp(v);
  }

  function mpx(w: number, h: number) {
    return `${((w * h) / 1e6).toFixed(1)} MP`;
  }

  function mediana(v: number[]): number {
    const s = [...v].sort((a, b) => a - b);
    return s[Math.floor(s.length / 2)] ?? 0;
  }

  function textoDiagnostico(): string {
    const d = diag;
    const instalada = matchMedia('(display-mode: standalone)').matches;
    const l: string[] = [`Revela ${__REVELA_VERSION__}${instalada ? ' (instalada)' : ' (navegador)'}`, navigator.userAgent];
    if (d.cargaInicial != null)
      l.push(`Carga inicial: ${ms(d.cargaInicial)} (wasm ${ms(d.initWasm ?? 0)}, ${d.desdeCache ? 'desde caché' : 'desde red'})`);
    if (d.gpu) l.push(`GPU: ${d.gpu}`);
    if (d.archivo) l.push(`Archivo (${d.archivo.origen}): ${d.archivo.nombre} · ${d.archivo.tipo} · ${mb(d.archivo.bytes)}`);
    if (info) l.push(`Foto: ${info.format} · original ${info.originalWidth}×${info.originalHeight} · trabajo ${info.width}×${info.height}`);
    if (d.abrir) l.push(`Abrir: ${ms(d.abrir.total)} (leer ${ms(d.abrir.leer)}, motor ${ms(d.abrir.motor)}, previa ${ms(d.abrir.previa)})`);
    if (d.vivo?.length)
      l.push(`Ajuste en vivo (${usarGpu ? 'GPU' : 'motor'}): mediana ${ms(mediana(d.vivo))}, máx ${ms(Math.max(...d.vivo))} (${d.vivo.length} cambios)`);
    if (d.exportar) l.push(`Exportar ${d.exportar.formato.toUpperCase()} q${d.exportar.calidad}: ${ms(d.exportar.ms)} · ${mb(d.exportar.bytes)}`);
    if (d.compartir) l.push(`Compartir: ${d.compartir}`);
    l.push(`Guardar en carpeta: ${soportaCarpeta ? (d.guardar ?? 'disponible') : 'no disponible (va a Descargas)'}`);
    if (d.memoria?.abrir)
      l.push(`Memoria del motor (pico): al abrir ${mb(d.memoria.abrir)}` + (d.memoria.exportar ? ` · al exportar ${mb(d.memoria.exportar)}` : ''));
    if (d.autoprueba) l.push(`Prueba GPU = exportación: ${d.autoprueba}`);
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
  const editada = $derived(!esVacia(receta));
</script>

<main>
  <header>
    <span class="marca">Revela</span>
    <div class="herramientas">
      {#if info}
        <button class="icono" aria-label="Deshacer" disabled={posicion === 0} onclick={() => irA(posicion - 1)}>
          <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 14 4 9l5-5" /><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11" /></svg>
        </button>
        <button class="icono" aria-label="Rehacer" disabled={posicion >= historial.length - 1} onclick={() => irA(posicion + 1)}>
          <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 14 5-5-5-5" /><path d="M20 9H9.5a5.5 5.5 0 0 0 0 11H13" /></svg>
        </button>
      {/if}
      <button class="icono" aria-label="Información y diagnóstico" onclick={() => (hojaInfo = true)}>
        <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <circle cx="12" cy="12" r="9.5" /><path d="M12 11v6" /><circle cx="12" cy="7.5" r="0.6" fill="currentColor" />
        </svg>
      </button>
    </div>
  </header>

  <section
    class="foto"
    aria-label="Foto (mantén pulsado para ver el original)"
    onpointerdown={pulsar}
    onpointerup={soltar}
    onpointercancel={soltar}
    onpointerleave={soltar}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <canvas bind:this={lienzoGpu} class:oculto={!info || !usarGpu}></canvas>
    <canvas bind:this={lienzo2d} class:oculto={!info || usarGpu}></canvas>
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
    {#if comparando}<div class="etiqueta">Original</div>{/if}
    {#if ocupado && estado !== 'cargando'}
      <div class="cargando">{estado === 'abriendo' ? 'Abriendo…' : 'Exportando…'}</div>
    {/if}
  </section>

  {#if error}<p class="mensaje error" role="alert">{error}</p>{/if}
  {#if aviso}<p class="mensaje" role="status">{aviso}</p>{/if}

  {#if info}
    <section class="panel">
      <div class="pestanas" role="tablist">
        {#each [['filtros', 'Filtros'], ['ajustes', 'Ajustes'], ['curvas', 'Curvas']] as [id, nombre] (id)}
          <button role="tab" aria-selected={pestana === id} class:activa={pestana === id} onclick={() => (pestana = id as Pestana)}>{nombre}</button>
        {/each}
        <button class="reiniciar" disabled={!editada} onclick={empezarDeNuevo}>Quitar todo</button>
      </div>
      <div class="contenido">
        {#if pestana === 'ajustes'}
          <Sliders valores={receta.sliders} oncambio={cambiarSlider} onfin={confirmar} />
        {:else if pestana === 'curvas'}
          <Curvas curvas={receta.curvas} {luts} oncambio={cambiarCurva} onfin={confirmar} />
        {:else}
          <Filtros filtro={receta.filtro} {claveFoto} dibujarMiniatura={baseMini ? dibujarMiniatura : null} oncambio={cambiarFiltro} onfin={confirmar} />
        {/if}
      </div>
    </section>
  {/if}

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
      <p class="sub">{info.width}×{info.height} px · {editada ? 'con tus ajustes' : 'sin ajustes'} · sin miniatura EXIF{conservarUbicacion ? '' : ' · sin ubicación'}</p>
    {/if}
    {#if exportado}
      <p class="sub">Listo: {exportado.name} · {mb(exportado.size)}</p>
      {#if avisoHoja}<p class="sub aviso-hoja" role="status">{avisoHoja}</p>{/if}
      <div class="acciones">
        <button class="boton" class:principal={!sePuedeCompartir} onclick={() => guardar()}>Guardar</button>
        {#if sePuedeCompartir}
          <button class="boton principal" onclick={compartir}>Compartir</button>
        {/if}
      </div>
      {#if soportaCarpeta}
        <p class="sub carpeta">
          {#if carpeta}
            Guarda en la carpeta <b>{carpeta}</b> ·
            <button class="enlace" onclick={() => guardar(true)}>Cambiar carpeta</button>
          {:else}
            La primera vez elige o crea la carpeta <b>Pictures/Revela</b>: la galería la muestra como álbum.
          {/if}
        </p>
      {/if}
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
      Filtros adaptados de los presets de <a href="https://github.com/storytold/lightcraft" target="_blank" rel="noopener">LightCraft</a> (código abierto, MIT o Apache 2.0).
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
    {#if probando}<p class="sub">Probando: {probando}</p>{/if}
    <div class="acciones">
      <button class="boton" disabled={!!probando} onclick={probarGpu}>Probar GPU</button>
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
    padding: 4px 8px 4px 16px;
  }
  .herramientas {
    display: flex;
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
    padding: 4px 16px;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
    -webkit-touch-callout: none;
  }
  canvas {
    max-width: 100%;
    max-height: 100%;
    border-radius: 6px;
    grid-area: 1 / 1;
  }
  .oculto {
    display: none;
  }
  .vacio {
    text-align: center;
    color: var(--texto-2);
    grid-area: 1 / 1;
  }
  .etiqueta {
    position: absolute;
    top: 14px;
    left: 50%;
    transform: translateX(-50%);
    background: #000a;
    padding: 4px 12px;
    border-radius: 999px;
    font-size: 13px;
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
  .panel {
    background: var(--panel);
    border-radius: 18px 18px 0 0;
    padding-top: 6px;
  }
  .pestanas {
    display: flex;
    gap: 4px;
    padding: 0 12px 4px;
  }
  .pestanas button {
    min-height: 40px;
    padding: 0 12px;
    color: var(--texto-2);
    font-weight: 600;
    border-bottom: 2px solid transparent;
  }
  .pestanas .activa {
    color: var(--texto);
    border-bottom-color: var(--acento);
  }
  .reiniciar {
    margin-left: auto;
    font-weight: 400 !important;
    font-size: 14px;
  }
  .contenido {
    min-height: 120px;
  }
  .barra {
    display: flex;
    gap: 8px;
    padding: 8px 16px 12px;
    background: var(--panel);
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
  .carpeta {
    margin-top: 12px;
  }
  .enlace {
    color: var(--acento);
    text-decoration: underline;
    padding: 8px 0;
  }
  .aviso-hoja {
    color: var(--texto);
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
