<script lang="ts">
  import { onMount } from 'svelte';
  import Sliders from './componentes/Sliders.svelte';
  import Curvas from './componentes/Curvas.svelte';
  import Filtros from './componentes/Filtros.svelte';
  import PanelRecorte from './componentes/PanelRecorte.svelte';
  import MarcoRecorte from './componentes/MarcoRecorte.svelte';
  import {
    COMPLETO,
    ANGULO_MAX,
    aspecto,
    aspectoNormal,
    encajar,
    geometriaJson,
    geometriaVacia,
    girar,
    marco,
    reencuadrar,
    voltearH,
    voltearV,
    type Proporcion,
    type Rect,
  } from './lib/recorte';
  import { Motor, motor, pedir, ultimaGana } from './lib/motor';
  import { AJUSTADA, acercar, esAjustada, limitar, mezclar, type Medidas, type Vista } from './lib/zoom';
  import { carpetaRecordada, guardarEnCarpeta, nombreBase as elegirNombreBase, soportaCarpeta } from './lib/guardar';
  import { Revelador, hayWebGL2, type ParametrosGpu, type PlanoGpu } from './lib/gpu';
  import { FILTROS, recetasDeFiltros, type Filtro } from './lib/filtros';
  import { aCameraRaw, esVacia, recetaVacia, type Canal, type IdSlider, type Punto, type Receta } from './lib/receta';
  import { probar, recetasDePrueba, TOLERANCIA, type MotorDePrueba } from './lib/autoprueba';
  import type { FormatoExport, InfoFoto, Previa } from './lib/protocolo';
  import {
    LIMITES_MP,
    avisoCamaraCerrado,
    cerrarAvisoCamara,
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
  type Pestana = 'recortar' | 'filtros' | 'ajustes' | 'curvas';

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
  /** Aviso de calidad al abrir con Cámara; una vez cerrado, no vuelve a salir. */
  let avisoCamara = $state(false);
  let avisoCamaraVisto = avisoCamaraCerrado();
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
  /** Avance de la exportación (0..1). */
  let progreso = $state(0);
  /** Tamaño de la foto exportada con el encuadre actual. */
  let tamExport = $state<[number, number] | null>(null);

  async function abrirExportar() {
    hojaExportar = true;
    try {
      tamExport = (await pedir('tamanoExport', geometriaJson($state.snapshot(receta.geometria)))).valor;
    } catch {
      tamExport = null;
    }
  }

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
  let ultimoPlano: PlanoGpu | null = null;
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
    async (json: string, geo: string) => (await pedir('previaGpu', json, geo, revelador?.ancho ?? 1, revelador?.alto ?? 1)).valor,
    ({ params, plano }) => {
      ultimosParams = params;
      ultimoPlano = plano;
      luts = params.luts;
      if (!comparando) {
        revelador?.dibujar(params, plano);
        if (cajaDetalle) detalle?.dibujar(params, null);
      }
      medirVivo();
      medirCaja();
    },
    (e) => (error = `No se pudo aplicar el ajuste: ${e.message}`),
  );

  const actualizarCpu = ultimaGana(
    async (json: string, geo: string) => (await pedir('previaMotor', LADO_CPU, json, geo)).valor,
    (p) => {
      if (!comparando) pintar2d(p);
      medirVivo();
      medirCaja();
    },
    (e) => (error = `No se pudo aplicar el ajuste: ${e.message}`),
  );

  /** Encuadre de la previa: mientras se recorta se ve el marco entero (con el recorte encima). */
  const geometriaPrevia = () => geometriaJson($state.snapshot(receta.geometria), pestana === 'recortar');

  function aplicar() {
    if (!info) return;
    tCambio = performance.now();
    const json = ajustesJson();
    // El slider manda: si la zona nítida se estaba calculando (o iba a pedirse), se abandona y
    // se vuelve a pedir al soltar (`confirmar`). Con otro encuadre, la que hay ya no vale.
    if (calculandoDetalle || tDetalle !== undefined) {
      cancelarDetalle();
      detallePendiente = true;
    }
    if (cajaDetalle && geoDetalle !== geometriaPrevia()) {
      ocultarDetalle();
      detallePendiente = true;
    }
    if (usarGpu) actualizarGpu(json, geometriaPrevia());
    else actualizarCpu(json, geometriaPrevia());
  }

  // --- Recortar, enderezar, girar y voltear ------------------------------------------------

  let modoLinea = $state(false);
  let seccionFoto: HTMLElement | undefined = $state();
  /** Dónde está dibujada la foto dentro de su sección (para poner el marco de recorte encima). */
  let caja = $state({ x: 0, y: 0, w: 0, h: 0 });

  /** Tamaño en píxeles de lo dibujado: si cambia (otro recorte, un giro…), el zoom vuelve a ajustar. */
  let tamLienzo = '';

  function medirCaja() {
    const l = usarGpu ? lienzoGpu : lienzo2d;
    if (!l || !seccionFoto) return;
    const tam = `${l.width}x${l.height}`;
    if (tam !== tamLienzo) {
      tamLienzo = tam;
      if (!esAjustada(vista)) {
        vista = AJUSTADA;
        ocultarDetalle();
      }
    }
    const a = l.getBoundingClientRect();
    const b = seccionFoto.getBoundingClientRect();
    caja = { x: a.left - b.left, y: a.top - b.top, w: a.width, h: a.height };
  }

  onMount(() => {
    const ro = new ResizeObserver(() => {
      const m = medidas();
      if (m) vista = limitar(vista, m);
      medirCaja();
      if (!esAjustada(vista)) programarDetalle();
    });
    if (seccionFoto) ro.observe(seccionFoto);
    return () => ro.disconnect();
  });

  /** Tamaño del marco (la foto girada) a resolución de trabajo. */
  function tamMarco(): [number, number] {
    return info ? marco(receta.geometria, info.width, info.height) : [1, 1];
  }

  function aspectoFijo(): number | null {
    const [fw, fh] = tamMarco();
    const a = aspecto(receta.geometria.proporcion, fw, fh);
    return a === null ? null : aspectoNormal(a, fw, fh);
  }

  function reencajar() {
    const [fw, fh] = tamMarco();
    receta.geometria.recorte = reencuadrar($state.snapshot(receta.geometria), fw, fh);
  }

  function cambiarProporcion(p: Proporcion) {
    receta.geometria.proporcion = p;
    receta.geometria.manual = false;
    reencajar();
    confirmar();
  }

  function cambiarAngulo(a: number) {
    receta.geometria.angulo = Math.max(-ANGULO_MAX, Math.min(ANGULO_MAX, a));
    reencajar();
    aplicar();
  }

  function transformar(f: (g: typeof receta.geometria) => typeof receta.geometria) {
    receta.geometria = f($state.snapshot(receta.geometria));
    reencajar();
    aplicar();
    confirmar();
  }

  function cambiarRecorte(deseado: Rect) {
    const [fw, fh] = tamMarco();
    const g = receta.geometria;
    receta.geometria.recorte = encajar(deseado, $state.snapshot(g.recorte) ?? COMPLETO, g.angulo, fw, fh);
    receta.geometria.manual = true;
  }

  async function enderezarConLinea(x0: number, y0: number, x1: number, y1: number) {
    modoLinea = false;
    const [fw, fh] = tamMarco();
    try {
      const d = (await pedir('anguloLinea', x0 * fw, y0 * fh, x1 * fw, y1 * fh)).valor;
      cambiarAngulo(receta.geometria.angulo + d);
      confirmar();
    } catch (e) {
      error = `No se pudo enderezar: ${(e as Error).message}`;
    }
  }

  function restablecerGeometria() {
    receta.geometria = geometriaVacia();
    modoLinea = false;
    aplicar();
    confirmar();
  }

  function cambiarPestana(p: Pestana) {
    const antes = pestana;
    pestana = p;
    modoLinea = false;
    // Entrar o salir de Recortar cambia lo que se dibuja (marco entero / recortado).
    if ((antes === 'recortar') !== (p === 'recortar')) aplicar();
  }

  function pintar2d(p: Previa) {
    if (!lienzo2d) return;
    lienzo2d.width = p.width;
    lienzo2d.height = p.height;
    lienzo2d.getContext('2d')?.putImageData(new ImageData(new Uint8ClampedArray(p.pixels), p.width, p.height), 0, 0);
  }

  /** Un paso de deshacer (al soltar un slider, elegir un filtro…). */
  function confirmar() {
    reanudarDetalle();
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
    reanudarDetalle();
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
    // Mientras se recorta, el dedo mueve el recorte: no se compara.
    if (!info || pestana === 'recortar') return;
    clearTimeout(temporizador);
    // Con zoom el dedo también arrastra: se espera un poco más para no confundirlo.
    temporizador = setTimeout(() => {
      comparando = true;
      if (usarGpu) {
        revelador?.dibujar(null, ultimoPlano);
        if (cajaDetalle) detalle?.dibujar(null, null);
      } else if (base) pintar2d(base);
    }, esAjustada(vista) ? 150 : 300);
  }
  function soltar() {
    clearTimeout(temporizador);
    if (!comparando) return;
    comparando = false;
    if (usarGpu) {
      revelador?.dibujar(ultimosParams, ultimoPlano);
      if (cajaDetalle) detalle?.dibujar(ultimosParams, null);
    } else aplicar();
  }

  // --- Zoom: pellizco, arrastre con un dedo y doble toque -----------------------------------
  //
  // La foto ajustada se agranda con CSS (rápido, pero se ve la previa reducida). Al soltar, si la
  // previa se queda corta, el motor dibuja la zona visible a resolución completa (sin ajustes) y
  // la GPU le pone los ajustes encima: la zona nítida.

  let vista = $state<Vista>(AJUSTADA);
  const transformacion = $derived(esAjustada(vista) ? undefined : `translate(${vista.x}px, ${vista.y}px) scale(${vista.z})`);
  let marcoRecorte: { cancelar: () => void } | undefined = $state();
  const punteros = new Map<number, { x: number; y: number }>();
  type Gesto =
    | { tipo: 'pellizco'; d0: number; m0: { x: number; y: number }; v0: Vista }
    | { tipo: 'arrastre'; p0: { x: number; y: number }; v0: Vista };
  let gesto: Gesto | null = null;
  /** El dedo actual (para el doble toque): dónde y cuándo bajó, y si se ha movido. */
  let toque: { t: number; x: number; y: number; movido: boolean; asa: boolean } | null = null;
  let ultimoToque: { t: number; x: number; y: number } | null = null;

  const lienzoVisible = () => (usarGpu ? lienzoGpu : lienzo2d);

  /** Medidas para el zoom, y el zoom al que un píxel de la foto es un píxel de la pantalla. */
  function medidas(): (Medidas & { z100: number }) | null {
    const l = lienzoVisible();
    if (!l || !seccionFoto || !info || !l.offsetWidth) return null;
    const tex = usarGpu ? (revelador?.ancho ?? 0) : (base?.width ?? 0);
    if (!tex) return null;
    // La previa es la foto reducida: tantos píxeles de la foto completa por píxel de la previa.
    const completo = (l.width * info.width) / tex;
    const z100 = completo / (l.offsetWidth * (devicePixelRatio || 1));
    return { fotoW: l.offsetWidth, fotoH: l.offsetHeight, areaW: seccionFoto.clientWidth, areaH: seccionFoto.clientHeight, zMax: Math.max(2 * z100, 2), z100 };
  }

  function capturar(id: number) {
    try {
      seccionFoto?.setPointerCapture(id);
    } catch {
      // Un puntero que ya no está activo: no pasa nada.
    }
  }

  function pos(e: PointerEvent) {
    const r = seccionFoto!.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  function empezarPellizco() {
    const [a, b] = [...punteros.values()];
    gesto = { tipo: 'pellizco', d0: Math.hypot(a.x - b.x, a.y - b.y) || 1, m0: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }, v0: vista };
    ocultarDetalle();
  }

  function abajoZoom(e: PointerEvent) {
    if (!info || !seccionFoto) return;
    cancelAnimationFrame(animacion);
    const p = pos(e);
    punteros.set(e.pointerId, p);
    if (punteros.size >= 2) {
      // Segundo dedo: pellizco. Lo que hacía el primero (mover el recorte, trazar la línea,
      // comparar) se abandona.
      e.stopPropagation();
      capturar(e.pointerId);
      if (punteros.size === 2) {
        marcoRecorte?.cancelar();
        clearTimeout(temporizador);
        soltar();
        toque = null;
        empezarPellizco();
      }
      return;
    }
    const asa = !!(e.target as HTMLElement).dataset?.asa;
    toque = { t: e.timeStamp, x: p.x, y: p.y, movido: false, asa };
    // Las asas del recorte y la línea de enderezar son del marco de recorte.
    if (asa || modoLinea) return;
    if (pestana === 'recortar') {
      // Sin zoom, el dedo mueve el recorte; con zoom, mueve la foto.
      if (esAjustada(vista)) return;
      e.stopPropagation();
    }
    capturar(e.pointerId);
    if (!esAjustada(vista)) gesto = { tipo: 'arrastre', p0: p, v0: vista };
    pulsar();
  }

  function moverZoom(e: PointerEvent) {
    if (!punteros.has(e.pointerId)) return;
    const p = pos(e);
    punteros.set(e.pointerId, p);
    if (toque && !toque.movido && Math.hypot(p.x - toque.x, p.y - toque.y) > 8) {
      toque.movido = true;
      clearTimeout(temporizador);
    }
    const m = medidas();
    if (!gesto || !m) return;
    e.stopPropagation();
    if (gesto.tipo === 'pellizco') {
      const [a, b] = [...punteros.values()];
      const d = Math.hypot(a.x - b.x, a.y - b.y);
      const c = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
      vista = acercar(gesto.v0, (gesto.v0.z * d) / gesto.d0, gesto.m0.x, gesto.m0.y, m, c.x - gesto.m0.x, c.y - gesto.m0.y);
    } else {
      if (!toque?.movido) return;
      if (comparando) return;
      if (cajaDetalle) ocultarDetalle();
      vista = limitar({ ...gesto.v0, x: gesto.v0.x + p.x - gesto.p0.x, y: gesto.v0.y + p.y - gesto.p0.y }, m);
    }
  }

  function arribaZoom(e: PointerEvent) {
    if (!punteros.delete(e.pointerId)) return;
    const habia = gesto;
    if (gesto) e.stopPropagation();
    if (gesto?.tipo === 'pellizco') {
      if (vista.z < 1.03) vista = AJUSTADA;
      // Queda un dedo: sigue arrastrando con él.
      const resto = [...punteros.values()][0];
      gesto = resto && !esAjustada(vista) ? { tipo: 'arrastre', p0: resto, v0: vista } : null;
      if (resto) toque = { t: 0, x: resto.x, y: resto.y, movido: true, asa: false };
    }
    if (punteros.size > 0) return;
    gesto = null;
    soltar();
    if (habia) programarDetalle();
    // Doble toque: alterna entre ajustar a pantalla y 100 %.
    const t = toque;
    toque = null;
    if (!t || t.movido || t.asa || modoLinea || e.type !== 'pointerup' || e.timeStamp - t.t > 300) {
      ultimoToque = null;
      return;
    }
    if (ultimoToque && t.t - ultimoToque.t < 350 && Math.hypot(t.x - ultimoToque.x, t.y - ultimoToque.y) < 40) {
      ultimoToque = null;
      dobleToque(t.x, t.y);
    } else ultimoToque = { t: e.timeStamp, x: t.x, y: t.y };
  }

  function dobleToque(x: number, y: number) {
    const m = medidas();
    if (!m) return;
    // Si la foto es más pequeña que la pantalla, el "100 %" sería alejarse: se acerca al doble.
    animarA(esAjustada(vista) ? acercar(vista, m.z100 > 1.05 ? m.z100 : 2, x, y, m) : AJUSTADA);
  }

  let animacion = 0;
  function animarA(destino: Vista) {
    const desde = vista;
    const t0 = performance.now();
    ocultarDetalle();
    cancelAnimationFrame(animacion);
    const paso = (t: number) => {
      const k = Math.min(1, (t - t0) / 220);
      vista = mezclar(desde, destino, k);
      if (k < 1) animacion = requestAnimationFrame(paso);
      else programarDetalle();
    };
    animacion = requestAnimationFrame(paso);
  }

  // El marco de recorte sigue a la foto con zoom.
  $effect(() => {
    void [vista.z, vista.x, vista.y];
    medirCaja();
  });

  // --- Zona nítida del zoom ---

  let lienzoDetalle: HTMLCanvasElement | undefined = $state();
  let detalle: Revelador | null = null;
  /** Dónde se ve la zona nítida (en la sección); `null`: oculta. */
  let cajaDetalle = $state<{ x: number; y: number; w: number; h: number } | null>(null);
  /** Encuadre con el que se calculó (si cambia, hay que pedirla otra vez). */
  let geoDetalle = '';
  let tDetalle: ReturnType<typeof setTimeout> | undefined = undefined;

  /** Cada cálculo de la zona nítida tiene su turno; cambiarlo abandona el que esté en marcha. */
  let turnoDetalle = 0;
  let calculandoDetalle = false;
  /** Se abandonó por un slider: se vuelve a pedir al soltarlo. */
  let detallePendiente = false;
  /** Filas por petición: entre una franja y otra, el worker atiende antes a los sliders. */
  const FILAS_DETALLE = 128;

  function cancelarDetalle() {
    clearTimeout(tDetalle);
    tDetalle = undefined;
    turnoDetalle++;
    calculandoDetalle = false;
  }

  function ocultarDetalle() {
    cancelarDetalle();
    cajaDetalle = null;
  }

  function programarDetalle() {
    clearTimeout(tDetalle);
    tDetalle = setTimeout(() => {
      tDetalle = undefined;
      pedirDetalle();
    }, 150);
  }

  function reanudarDetalle() {
    if (!detallePendiente) return;
    detallePendiente = false;
    if (!esAjustada(vista)) programarDetalle();
  }

  /** Lo que determina la zona nítida: si cambia mientras se calcula, se descarta. */
  function claveVista() {
    const l = lienzoVisible();
    return `${geometriaPrevia()}|${vista.z}|${vista.x}|${vista.y}|${l?.offsetWidth}x${l?.offsetHeight}|${l?.width}`;
  }

  /** Pide la zona nítida franja a franja; si mientras tanto cambia el turno, la abandona. */
  async function traerDetalle(geo: string, ventana: [number, number, number, number], ancho: number, alto: number, clave: string, caja: { x: number; y: number; w: number; h: number }) {
    const turno = ++turnoDetalle;
    calculandoDetalle = true;
    let px: Uint8Array | null = null;
    let tam: [number, number] = [0, 0];
    try {
      for (let fila = 0; !px || fila < tam[1]; fila += FILAS_DETALLE) {
        const f = (await pedir('detalle', geo, ventana, ancho, alto, fila, fila + FILAS_DETALLE)).valor;
        if (turno !== turnoDetalle) return;
        if (!px) {
          tam = [f.ancho, f.alto];
          px = new Uint8Array(f.ancho * f.alto * 4);
        }
        px.set(f.pixels, f.fila0 * f.ancho * 4);
      }
    } catch (e) {
      // No es grave: se queda la previa ampliada.
      if (turno === turnoDetalle) console.warn('zona nítida:', (e as Error).message);
      return;
    } finally {
      if (turno === turnoDetalle) calculandoDetalle = false;
    }
    if (!detalle || clave !== claveVista() || gesto) return;
    detalle.cargar(tam[0], tam[1], px);
    detalle.dibujar(comparando ? null : ultimosParams, null);
    geoDetalle = geo;
    cajaDetalle = caja;
  }

  function pedirDetalle() {
    const l = lienzoVisible();
    if (!usarGpu || !info || !l || !seccionFoto || esAjustada(vista) || gesto) return ocultarDetalle();
    if (!detalle && lienzoDetalle) {
      try {
        detalle = new Revelador(lienzoDetalle);
      } catch {
        return;
      }
    }
    if (!detalle) return;
    const dpr = devicePixelRatio || 1;
    const r = l.getBoundingClientRect();
    // Si la previa ya da un píxel por cada píxel de la pantalla, no hace falta.
    if (r.width * dpr <= l.width * 1.15) return ocultarDetalle();
    const s = seccionFoto.getBoundingClientRect();
    const x0 = Math.max(r.left, s.left);
    const y0 = Math.max(r.top, s.top);
    const x1 = Math.min(r.right, s.right);
    const y1 = Math.min(r.bottom, s.bottom);
    if (x1 - x0 < 1 || y1 - y0 < 1) return;
    const ventana: [number, number, number, number] = [(x0 - r.left) / r.width, (y0 - r.top) / r.height, (x1 - r.left) / r.width, (y1 - r.top) / r.height];
    const caja = { x: x0 - s.left, y: y0 - s.top, w: x1 - x0, h: y1 - y0 };
    traerDetalle(geometriaPrevia(), ventana, Math.round(caja.w * dpr), Math.round(caja.h * dpr), claveVista(), caja);
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
      const p = (await pedir('previaMotor', 160, json, '{}')).valor;
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
      const deCamara = entrada.hasAttribute('capture');
      nombreBase = elegirNombreBase(archivo.name, info.takenAt, deCamara);
      // Foto nueva: receta e historial nuevos.
      receta = recetaVacia();
      historial = [recetaVacia()];
      posicion = 0;
      claveFoto++;
      ultimosParams = null;
      ultimoPlano = null;
      modoLinea = false;
      vista = AJUSTADA;
      ocultarDetalle();
      if (usarGpu) revelador?.dibujar(null);
      else pintar2d(base);
      diag.abrir = { leer: tLeido - t0, motor: abierto.ms, previa: previa.ms, total: performance.now() - t0 };
      diag.memoria = { abrir: previa.memoria };
      diag.vivo = [];
      if (info.originalWidth !== info.width) {
        aviso = `Foto reducida de ${mpx(info.originalWidth, info.originalHeight)} a ${mpx(info.width, info.height)} (límite ${limiteMp} MP).`;
      }
      // El navegador entrega la foto de la cámara comprimida (≈ 1,4 MB a 12 MP en Chrome).
      avisoCamara = deCamara && !avisoCamaraVisto;
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
      progreso = 0;
      const geo = geometriaJson($state.snapshot(receta.geometria));
      const r = await motor.pedirConProgreso('exportar', (f) => (progreso = f), formato, calidad, conservarUbicacion, ajustesJson(), geo);
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
      previaMotor: async (lado, json, geo) => (await m.pedir('previaMotor', lado, json, geo)).valor,
      previaGpu: async (json, geo, w, h) => (await m.pedir('previaGpu', json, geo, w, h)).valor,
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
        <button class="icono" aria-label="Quitar todos los cambios" title="Quitar todos los cambios" disabled={!editada} onclick={empezarDeNuevo}>
          <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h16" /><path d="M10 11v6M14 11v6" /><path d="M6 7l1 13h10l1-13" /><path d="M9 7V4h6v3" /></svg>
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
    bind:this={seccionFoto}
    class="foto"
    aria-label="Foto (pellizca o toca dos veces para hacer zoom; mantén pulsado para ver el original)"
    onpointerdowncapture={abajoZoom}
    onpointermovecapture={moverZoom}
    onpointerupcapture={arribaZoom}
    onpointercancelcapture={arribaZoom}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <canvas bind:this={lienzoGpu} class:oculto={!info || !usarGpu} style:transform={transformacion}></canvas>
    <canvas bind:this={lienzo2d} class:oculto={!info || usarGpu} style:transform={transformacion}></canvas>
    <canvas
      bind:this={lienzoDetalle}
      class="detalle"
      class:oculto={!info || !cajaDetalle}
      style="left: {cajaDetalle?.x ?? 0}px; top: {cajaDetalle?.y ?? 0}px; width: {cajaDetalle?.w ?? 0}px; height: {cajaDetalle?.h ?? 0}px"
    ></canvas>
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
    {#if info && pestana === 'recortar' && !comparando && caja.w > 0}
      <div class="sobre" style="left: {caja.x}px; top: {caja.y}px; width: {caja.w}px; height: {caja.h}px">
        <MarcoRecorte
          bind:this={marcoRecorte}
          recorte={receta.geometria.recorte ?? COMPLETO}
          aspecto={aspectoFijo()}
          {modoLinea}
          oncambio={cambiarRecorte}
          onfin={confirmar}
          onlinea={enderezarConLinea}
        />
      </div>
    {/if}
    {#if comparando}<div class="etiqueta">Original</div>{/if}
    {#if ocupado && estado !== 'cargando'}
      <div class="cargando">{estado === 'abriendo' ? 'Abriendo…' : 'Exportando…'}</div>
    {/if}
  </section>

  {#if error}<p class="mensaje error" role="alert">{error}</p>{/if}
  {#if aviso}<p class="mensaje" role="status">{aviso}</p>{/if}
  {#if avisoCamara}
    <div class="mensaje cerrable" role="status">
      <p>Para máxima calidad, toma la foto con la cámara del teléfono y ábrela desde Galería.</p>
      <button
        class="cerrar"
        aria-label="Cerrar el aviso"
        onclick={() => {
          avisoCamara = false;
          avisoCamaraVisto = true;
          cerrarAvisoCamara();
        }}
      >
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg>
      </button>
    </div>
  {/if}

  {#if info}
    <section class="panel">
      <div class="pestanas" role="tablist">
        {#each [['recortar', 'Recortar'], ['filtros', 'Filtros'], ['ajustes', 'Ajustes'], ['curvas', 'Curvas']] as [id, nombre] (id)}
          <button role="tab" aria-selected={pestana === id} class:activa={pestana === id} onclick={() => cambiarPestana(id as Pestana)}>{nombre}</button>
        {/each}
      </div>
      <div class="contenido">
        {#if pestana === 'recortar'}
          <PanelRecorte
            geometria={receta.geometria}
            {modoLinea}
            onproporcion={cambiarProporcion}
            onangulo={cambiarAngulo}
            onfin={confirmar}
            ongirar={() => transformar(girar)}
            onvoltearH={() => transformar(voltearH)}
            onvoltearV={() => transformar(voltearV)}
            onlinea={() => (modoLinea = !modoLinea)}
            onrestablecer={restablecerGeometria}
          />
        {:else if pestana === 'ajustes'}
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
    <button class="boton principal" disabled={ocupado || !info} onclick={abrirExportar}>Exportar</button>
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
      <p class="sub">{tamExport ? `${tamExport[0]}×${tamExport[1]}` : `${info.width}×${info.height}`} px · {editada ? 'con tus ajustes' : 'sin ajustes'} · sin miniatura EXIF{conservarUbicacion ? '' : ' · sin ubicación'}</p>
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
      {#if estado === 'exportando'}
        <div class="progreso" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(progreso * 100)}>
          <div class="barra-progreso" style="width: {progreso * 100}%"></div>
        </div>
        <p class="sub">Exportando… {Math.round(progreso * 100)} %</p>
      {:else}
        <div class="acciones">
          <button class="boton principal" disabled={ocupado} onclick={exportar}>Preparar archivo</button>
        </div>
      {/if}
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
    overflow: hidden;
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
  .detalle {
    position: absolute;
    /* Sin área de cuadrícula: así se coloca respecto a la sección, como el marco de recorte. */
    grid-area: auto;
    border-radius: 0;
    max-width: none;
    max-height: none;
    pointer-events: none;
  }
  .vacio {
    text-align: center;
    color: var(--texto-2);
    grid-area: 1 / 1;
  }
  .sobre {
    position: absolute;
  }
  .progreso {
    height: 8px;
    border-radius: 4px;
    background: var(--panel-2);
    overflow: hidden;
    margin-top: 16px;
  }
  .barra-progreso {
    height: 100%;
    background: var(--acento);
    transition: width 0.15s linear;
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
  .cerrable {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .cerrable p {
    margin: 0;
    flex: 1;
  }
  .cerrar {
    flex: none;
    width: 40px;
    height: 40px;
    margin: -8px -8px -8px 0;
    display: grid;
    place-items: center;
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
    justify-content: space-around;
    padding: 0 8px 4px;
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
