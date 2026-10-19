# Revela: arquitectura

> Última actualización: 2026-10-17 (Fase 4: ajustes, curvas, filtros y recorte).

Revela es una PWA móvil que usa el motor de PhotoCraft compilado a WebAssembly, con una
interfaz nueva en Svelte. Todo se procesa en el dispositivo.

## Piezas

```
apps/revela/
  Cargo.toml, src/      revela-core: puente Rust → wasm (wasm-bindgen). Sin egui ni GPU.
    lib.rs              Editor: abrir, vista previa, exportar (Rust normal, con pruebas)
    ajustes.rs          ajustes por píxel (Camera Raw de PhotoCraft) y parámetros del shader
    geometria.rs        girar, voltear, enderezar y recortar: una afín + bilineal
    metadata.rs         limpieza de EXIF al exportar (GPS, miniatura) y fecha de captura
    wasm.rs             la API que ve JavaScript
  tests/                pruebas de punta a punta y medición de memoria
  build-wasm.sh         compila el wasm a web/src/wasm/ (no se versiona)
  scripts/iconos.py     genera los iconos PNG
  web/                  interfaz: Svelte 5 + TypeScript + Vite + vite-plugin-pwa
    src/worker.ts       Web Worker donde vive el motor
    src/lib/motor.ts    cliente del worker (pedir(op, ...args) → promesa)
    src/lib/gpu.ts      previa en la GPU: shader WebGL2 (highp) de los ajustes
    src/lib/receta.ts   la receta (sliders, curvas, filtro, geometría) → ajustes de Camera Raw
    src/lib/recorte.ts  rectángulo de recorte: proporciones, sin esquinas vacías, girar/voltear
    src/lib/filtros.*   filtros (presets de LightCraft traducidos)
    src/lib/autoprueba.ts  prueba GPU = motor (CI y ⓘ › Probar GPU)
    src/componentes/    Sliders, Curvas, Filtros, PanelRecorte, MarcoRecorte
    src/App.svelte      la pantalla
    pruebas/igualdad.*  prueba de igualdad en Chromium headless (bloquea el despliegue)
.github/workflows/revela-pages.yml   pruebas, build y despliegue en GitHub Pages
```

`revela-core` depende de `engine`, `io`, `codecs` (con `heif`), `compose` y `doc`. No toca
ningún crate del motor (ver `docs/revela-cambios-motor.md`).

## Flujo

1. La interfaz lee el archivo y lo **transfiere** (sin copiar) al worker.
2. `Editor::open` decodifica con el motor. **La orientación EXIF la aplica el decodificador**:
   la foto queda derecha y al exportar se escribe Orientation = 1.
3. Si pasa del **límite de megapíxeles** (24 MP en Android, 16 MP en iOS, configurable en ⓘ),
   se reduce con bicúbico. Esa es la resolución de trabajo y la que se exporta.
4. Se crea una **copia de vista previa** del tamaño de la pantalla (máx. 2048 px, bilineal del
   motor, que ensancha el filtro al reducir). La pantalla y, en el MVP, los ajustes en vivo usan
   esta copia; la foto completa solo se procesa al exportar.
5. Exportar codifica la foto completa (JPG con calidad 40–100, o PNG) y la interfaz ofrece
   **Compartir** (Web Share API con archivos) o **Guardar** (descarga). Compartir es un toque
   aparte porque el navegador exige un gesto reciente del usuario.

Si el wasm falla (pánico o falta de memoria), el worker responde `fatal` y la interfaz crea
otro worker en el siguiente pedido, así que la página no se queda colgada.

## Ajustes en vivo: previa en la GPU, exportación en el motor

Medido antes de construir los sliders (wasm de un hilo, copia de previa de 1,5 MP): el filtro
Camera Raw del motor tarda 491 ms por cambio con ajustes simples y 4,7 s con sombras/luces
locales; exportar 12 MP con el comando entero gasta 770 MB. Por eso:

- **Ajustes = etapas por píxel del Camera Raw de PhotoCraft**: temperatura/tinte y exposición,
  contraste, vibrancia, saturación, curva paramétrica (Sombras/Luces globales, coeficientes en
  `web/src/lib/tono.json`), curvas de puntos Luz/R/G/B, HSL y gradación de color.
  `ajustes::sanear` descarta cualquier otro control, así que la exportación no puede usar algo
  que la previa no muestra.
- **Previa**: shader WebGL2 en `highp` (`gpu.ts`) que replica paso a paso `camera_raw::develop`.
  Todas las constantes y las curvas las calcula el motor (`gpuParams`): la curva paramétrica sale
  de pasar una rampa de grises por `develop`, sin copiar código. Un cambio de slider tarda
  ~1 ms (mediana; 14 ms máx.) incluido el viaje al worker. Sin WebGL2, la previa la calcula el
  motor a 900 px.
- **Exportación**: el motor compone la foto por franjas de 256 filas (`compose::render_bands`) y
  revela cada una con `develop`. Las etapas por píxel dan 0/255 de diferencia enteras o por
  franjas (medido). Escribe RGB directo (RGBA solo para un PNG con transparencia): exportar 12 MP
  no sube la memoria sobre lo que ya usa abrir (189 MB) y tarda ~4,3 s.
- **Igualdad comprobada**: `pruebas/igualdad.mjs` compara shader y motor en Chromium headless
  (SwiftShader) sobre un cielo suave, una rampa de grises, todos los colores y ruido, con 53
  recetas (extremos de cada etapa, combinaciones al azar y los 25 filtros): máx. 1/255 y los
  degradados conservan sus niveles (sin bandas). Corre en el CI y **bloquea el despliegue** si
  falla (comprobado rompiendo el shader a propósito). La misma prueba se lanza en el teléfono
  desde ⓘ › Diagnóstico › Probar GPU, con su GPU real.
- **Receta**: sliders + curvas + filtro con intensidad (`receta.ts`). El filtro se escala por su
  intensidad y los sliders se suman encima; las curvas del usuario mandan. Deshacer/rehacer
  guarda la receta entera (100 pasos).

## Recortar, enderezar, girar y voltear

- **Una sola afín** (`geometria.rs`, `Plano`): cada píxel de salida → un punto de la foto de
  origen. Orden: giros de 90°, volteos, enderezado (±45°, alrededor del centro) y recorte
  `[cx, cy, ancho, alto]` en fracciones del marco girado (igual a cualquier resolución).
- **El mismo muestreo bilineal en `f32`** en el shader y en el núcleo (centros de píxel en
  `i + ½`, vecinos sujetos al borde, fuera de la foto transparente). La matriz la calcula el
  núcleo y la GPU la usa tal cual.
- **Exportación por bloques de 256×256**: cada bloque pide al motor solo su zona de la foto
  (`compose::render` de la huella) y la remuestrea y revela. No se usa `image.crop` con ángulo
  porque gira la foto entera (mucha memoria a 12 MP). Girar 90° y voltear dan exactamente lo
  mismo que los comandos del motor (prueba `quarter_turns_and_flips_match...`), y la exportación
  por bloques coincide con la previa a la misma resolución (sin costuras).
- **Sin esquinas vacías** (`recorte.ts`): el recorte siempre cabe en la foto enderezada. Al
  enderezar o cambiar la proporción, si el usuario no movió el recorte, se usa el rectángulo
  máximo con esa proporción; si lo movió, solo se encoge lo necesario. Mover y redimensionar se
  frenan en el borde (búsqueda binaria hacia el último recorte válido).
- **Enderezar con una línea**: el motor no detecta el horizonte solo; `straightenLayer` usa la
  regla del usuario. Revela pide trazar una línea y usa la misma regla del motor
  (`analysis_cmds::straighten_angle`). Un enderezado automático (detección de líneas) queda
  para más adelante.
- La prueba de igualdad incluye 6 encuadres cruzados con 5 recetas (332 comparaciones, máx.
  1/255, sin píxeles de borde distintos).

## Pestañas y color

- **Pestañas:** Recortar · Filtros · Editar. Editar tiene las secciones Luz (exposición, contraste, luces y sombras), Color, Efectos y Curvas, como Lightroom móvil.
- **Color:** temperatura, tinte, vibrancia y saturación, más dos paneles:
  - **Mezcla:** los 8 colores del HSL del motor; para cada uno, tono, saturación y luminancia, con las barras pintadas con el color real (el tono va de un color vecino al otro).
    - **Sobre la foto:**
      - Mantener pulsado abre una lupa (15×15 px de la previa ampliados) para apuntar; al soltar se fija ese color.
      - Se detecta el color medio de una zona de 7×7 px, recuadrada en la lupa.
      - Con saturación < 0,05 (o casi negro) es neutro y no se fija. Por debajo de 0,15 se avisa: "Casi neutro: el ajuste tendrá poco efecto".
      - Arriba, en grande, el nombre que le daría una persona (`nombreVisible`: "Marrón", "Beige", "Gris frío", "Oliva", "Azul cielo"…). Debajo, en pequeño, qué color de la mezcla lo ajusta ("se ajusta con Naranja"; dos si el segundo pesa ≥ 25 %). Si coinciden ("Azul"), una sola línea.
      - Los círculos de esos colores se resaltan con un anillo de acento en la fila de colores.
      - Un toque corto fija el color del punto.
      - Arrastrar en vertical ajusta el color fijado. Se elige qué cambia: tono, saturación o luminancia.
    - El color se mira sin la mezcla ni la gradación (`Revelador.muestra`), que es lo que ve el HSL del motor.
    - El cambio se reparte entre los colores vecinos con los pesos del motor (`pesosBandas`), normalizados para que el punto tocado cambie justo lo arrastrado.
  - **Gradación:** una rueda por zona (sombras, medios, luces, global), con luminancia, mezcla y balance.
- **Cómo se combinan con el filtro elegido** (`aCameraRaw` en `receta.ts`):
  - El HSL se suma banda a banda.
  - Las ruedas se suman como colores (vectores de tono y saturación), así que dos tintes opuestos se anulan.
  - La mezcla del usuario manda si la ha cambiado (por defecto, 50 como el motor).
  - El balance se suma.
- Son etapas por píxel que ya estaban en el shader y en la exportación, y la prueba de igualdad ya las cubría (también en extremos).

## Viñeta (Efectos)

- El motor centra la viñeta en el trozo que recibe; al exportar por franjas o bloques saldría una por trozo. Revela usa **la misma fórmula** del motor (`apps/revela/src/efectos.rs`) con coordenadas del encuadre final.
- Va después del revelado y antes de recortar a 0..1, como en el motor: `develop` se llama con `float = true` y la viñeta se aplica encima.
- Pruebas: por bloques coincide con el `develop` del motor sobre la imagen entera (< 1e-5), y la exportación por franjas con viñeta coincide con `filter.cameraRaw` (≤ 1/255).
- En la GPU es una etapa más del shader. Las constantes salen de Rust (`gpuParams.vineta`); la posición la pone la interfaz con un `MarcoVineta` (dónde está el encuadre final en el lienzo). Así sirve igual en la previa, en Recortar (centrada en el recorte, aunque se vea el marco entero) y en la zona nítida del zoom.
- La prueba de igualdad del CI incluye 5 recetas con viñeta (los tres estilos del motor), también con encuadres.
- Controles: Cantidad, Punto medio, Redondez, Difuminado y Luces (solo cuando oscurece). La cantidad se suma a la del filtro; la forma, si el usuario pone viñeta, es la suya.

## Sliders

Todos los sliders de edición (Luz, Color, Mezcla, Gradación, Viñeta, Enderezar e intensidad del filtro) usan `use:iman` (`lib/iman.ts`):
- **Arrastre relativo:** el valor se mueve lo que se mueve el dedo; tocar no salta. El dedo lo recibe el contenedor del `<input>`; el `<input>` queda para el teclado y la accesibilidad.
- **Ajuste fino:** cuanto más sube el dedo por encima del slider, más despacio va (½ a partir de 40 px, ¼ a partir de 100 px, 1/10 a partir de 170 px), con un rótulo junto al dedo.
- **Imán:** al cruzar el neutro se detiene en él, con una vibración corta, y hay que seguir 12 px para salir. No atrae desde cerca.
- **Doble toque:** vuelve al neutro.
- **Valor exacto:** al tocar el número se puede escribir (`Valor.svelte`). Se ajusta a los límites y al paso del slider, acepta coma decimal y tiene un botón ± porque el teclado decimal de Android no siempre trae "−".

## La edición no se pierde

- La foto abierta y la receta (con su historial de deshacer) se guardan en IndexedDB (`src/lib/edicion.ts`, base `revela-edicion`). Solo una edición, la última.
- La foto se escribe una vez al abrirla; la receta, en cada paso de deshacer y al pasar la app a segundo plano.
- Al abrir Revela, si hay una edición guardada se vuelve a abrir antes de enseñar la pantalla inicial, con el aviso "Se recuperó tu edición" y el botón **Descartar** (vuelve a la pantalla inicial y la borra).
- Exportar no la borra; abrir otra foto la sustituye; la papelera la borra (si se sigue editando, se guarda de nuevo).
- Se pide almacenamiento persistente (`navigator.storage.persist`) para que el navegador no la borre si le falta sitio.

## Actualizaciones

- El service worker está en modo `prompt` (`src/lib/actualizar.ts`): la versión nueva se descarga sola pero espera.
- Sale el aviso "Nueva versión de Revela disponible · Actualizar" y la página se recarga solo cuando el usuario lo toca, para no perder una edición a medias. La edición en curso se recupera al recargar.
- También se busca al volver a la app y a mano en ⓘ (Buscar actualización / Actualizar ahora).
- La activación (mensaje `SKIP_WAITING` y recarga en `controllerchange`) se hace a mano: la de `registerSW` solo recarga si fue ella quien encontró la versión.

## Barra de arriba

Con una foto abierta no hay botones abajo: arriba van los iconos de abrir (galería y cámara), deshacer, quitar todo, rehacer, información y exportar. Los botones grandes solo están en la pantalla inicial.

## Zoom

- **Gestos** (`App.svelte`, matemática en `web/src/lib/zoom.ts`):
  - Pellizco con dos dedos (zoom alrededor del punto medio, y arrastre a la vez).
  - Arrastre con un dedo cuando hay zoom.
  - Doble toque para alternar entre ajustar a pantalla y 100 % (un píxel de la foto por píxel físico de la pantalla). Si la foto es más pequeña que la pantalla, se acerca al doble.
  - El máximo es el doble del 100 %.
- **En Recortar** funciona igual:
  - Las asas y la línea de enderezar mandan.
  - Sin zoom, el dedo dentro del recorte lo mueve; con zoom, mueve la foto.
  - Un segundo dedo cancela lo que hacía el primero.
- **Mientras dura el gesto** la previa reducida solo se agranda con CSS, que es instantáneo.
- **Zona nítida:** al soltar, si la previa se queda corta, el motor dibuja la zona visible a resolución completa:
  - Usa `Editor::detalle`, que es la misma geometría y el mismo bilineal que la exportación, por bloques, sin ajustes y como mucho a la resolución de la pantalla.
  - La GPU le pone los ajustes encima. Por eso mover un slider con zoom no vuelve a pedirla.
  - Se pide **por franjas de 128 filas**, una petición cada vez. Si un slider se mueve mientras tanto, su petición pasa delante de la siguiente franja (espera como mucho una), la zona a medias se descarta y se vuelve a pedir al soltar el slider.
  - Una prueba comprueba que coincide con la exportación (≤ 1/255).
- **Sin WebGL2** hay zoom, pero sin zona nítida.

## Filtros

Los presets de LightCraft (commit `f34c7a8a`) traducidos por `scripts/traducir_filtros.py`:
25 de 41 (7 completos, 18 aproximados). Los locales de tono se aproximan con la curva global;
el balance de blancos en Kelvin se convierte al slider de temperatura igualando el cambio
rojo/azul de una adaptación Bradford; B/N = saturación −100. Quedan fuera los que dependen de
claridad, textura, dehaze o grano ≥ 15, o viñeta ≥ 25 (la lista sale del script). Atribución en
`filtros.json`, `ATTRIBUTION.md`, `NOTICE` y la pantalla ⓘ.

## Para más adelante: ajustes locales sin costuras

Sombras/luces, blancos/negros y claridad **locales** (como Lightroom) mirarían a los vecinos, y
por franjas dejarían costuras (medido: hasta 103/255). Idea: calcular una **guía de luminancia de
baja resolución de la foto completa** (unos 512 px, con el filtro guiado / Laplaciano local del
motor) una sola vez, y que la previa y **todas** las franjas de la exportación lean esa misma
guía, escalada a su posición. Así cada píxel depende solo de sí mismo y de la guía compartida:
sin costuras, y la GPU puede usar la guía como textura. Mismo método de verificación (≤ 1/255 en
el CI). Blancos/negros adaptativos ya usan estadísticas de una copia de ≤ 256 px, así que encajan.

## Privacidad de los metadatos

El motor conserva el EXIF original. `metadata.rs` lo limpia antes de exportar:

- **GPS**: se borra por defecto (la IFD GPS se pone a cero y se quita su puntero). Hay un
  interruptor "Conservar ubicación GPS".
- **Miniatura EXIF (IFD1)**: se borra siempre, porque muestra la foto sin editar (por ejemplo,
  lo que se recortó).
- **XMP y texto libre**: no se exportan (pueden repetir ubicación e historial).
- Si el EXIF no se puede analizar con seguridad, se descarta entero.

## HEIC de iPhone

- El wasm incluye el decodificador HEIC del motor (`photocraft-heif`, Rust puro). Probado en
  Chromium con las muestras del corpus (`cargo xtask corpus --heif`): rejilla de teselas estilo
  iPhone y HEIC con EXIF.
- Safari en iOS, al elegir desde Fotos con `accept="image/*"`, normalmente **convierte el HEIC a
  JPEG** antes de entregarlo. Desde la app Archivos llega el HEIC original. Ambos caminos abren.
- **Pendiente de confirmar en un iPhone real**: el panel ⓘ › Diagnóstico muestra el tipo que
  entregó el navegador (`image/jpeg` o `image/heic`) y el formato que detectó el motor.

## Fuentes para la herramienta de texto (decisión)

- Se usa el mecanismo de **fuentes servidas** que ya tiene el motor (`photocraft_text::served`):
  un `fonts/manifest.txt` junto a `index.html`, en el formato de craft-fonts
  (`familia | estilo | archivo | scripts | licencia | archivo de licencia | sha256 | fuente`).
  El worker descarga cada fuente y la registra con `FontDb::register_font_data`. No hace falta
  cambiar el motor.
- **Selección curada de fuentes OFL** pensada para redes (por ejemplo Inter, Montserrat,
  Playfair Display, Bebas Neue, Pacifico; se cierra en la Fase 5). Los archivos **no se
  versionan**: el workflow los descarga de `google/fonts` en un commit fijo y verifica su
  sha256, con cada `OFL.txt` al lado.
- El service worker las **precarga** con el resto de la app, para que el texto funcione sin
  conexión desde la primera visita.
- La interfaz usa **los mismos archivos** con `@font-face` para el selector de fuentes, así que lo
  que se ve en el menú es exactamente lo que dibuja el motor.

## Quitar fondo: API intercambiable (diseño, Fase 5)

La selección de sujeto se pide por **método**, y el resultado siempre es una **máscara alfa de
8 bits** del tamaño de trabajo de la foto. Así se puede cambiar el método sin tocar el resto:

```ts
type MetodoSujeto = 'clasico' | 'modelo';
pedir('mascaraSujeto', metodo)        // → { width, height, alpha: Uint8Array }
pedir('usarMascara', alpha, w, h)     // máscara calculada fuera del motor
```

- `clasico`: `select.subject` del motor (saliencia + GrabCut, sin IA). Primera versión.
- `modelo`: un modelo de segmentación ligero ejecutado en el dispositivo (por ejemplo con
  ONNX Runtime Web en el mismo worker, WebGPU con respaldo WASM). Calcula la máscara sobre la
  vista previa y la escala al tamaño de trabajo; se entrega con `usarMascara`.
- La receta de edición guarda `fondo: { metodo, color | degradado }`, no la máscara: al exportar
  se recalcula a resolución completa con el mismo método.

## Mediciones (esqueleto)

Foto de 11,9 MP (3747×3178, JPEG de 6 MB, dominio público). Chromium headless con perfil de
Pixel 7, **en el PC de desarrollo (Ryzen 9 5900X)**. En el teléfono serán más lentas; el panel
ⓘ › Diagnóstico muestra las mismas cifras en el dispositivo.

| Medida | Valor |
| --- | --- |
| wasm (sin comprimir / gzip) | 16,8 MB / 7,0 MB |
| Carga inicial, primera visita / desde caché | 159 ms / 146 ms |
| Abrir (leer + decodificar + copia de previa + dibujar) | 0,95 s |
| Exportar JPG calidad 90 a resolución completa | 0,39 s (1,9 MB) |
| Pico de memoria del wasm, abrir + exportar | 189 MB |

Medición nativa por etapa (`tests/memoria.rs`, un hilo): abrir +122 MB, previa +55 MB,
exportar +66 MB sobre lo que ya había.

Cambios que dieron estas cifras: la previa sale de la copia reducida y no de la foto completa
(el pico pasó de 320 a 189 MB y la previa de 459 a 120 ms), y `algo`, `compose` y `cms` se
compilan a velocidad (`opt-level=3`) con `--config` en `build-wasm.sh`, sin tocar el
`Cargo.toml` de PhotoCraft.
