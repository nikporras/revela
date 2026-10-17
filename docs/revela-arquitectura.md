# Revela: arquitectura

> Última actualización: 2026-10-17 (Fase 4: ajustes en vivo, curvas y filtros).

Revela es una PWA móvil que usa el motor de PhotoCraft compilado a WebAssembly, con una
interfaz nueva en Svelte. Todo se procesa en el dispositivo.

## Piezas

```
apps/revela/
  Cargo.toml, src/      revela-core: puente Rust → wasm (wasm-bindgen). Sin egui ni GPU.
    lib.rs              Editor: abrir, vista previa, exportar (Rust normal, con pruebas)
    ajustes.rs          ajustes por píxel (Camera Raw de PhotoCraft) y parámetros del shader
    metadata.rs         limpieza de EXIF al exportar (GPS, miniatura) y fecha de captura
    wasm.rs             la API que ve JavaScript
  tests/                pruebas de punta a punta y medición de memoria
  build-wasm.sh         compila el wasm a web/src/wasm/ (no se versiona)
  scripts/iconos.py     genera los iconos PNG
  web/                  interfaz: Svelte 5 + TypeScript + Vite + vite-plugin-pwa
    src/worker.ts       Web Worker donde vive el motor
    src/lib/motor.ts    cliente del worker (pedir(op, ...args) → promesa)
    src/lib/gpu.ts      previa en la GPU: shader WebGL2 (highp) de los ajustes
    src/lib/receta.ts   la receta (sliders, curvas, filtro) → ajustes de Camera Raw
    src/lib/filtros.*   filtros (presets de LightCraft traducidos)
    src/lib/autoprueba.ts  prueba GPU = motor (CI y ⓘ › Probar GPU)
    src/componentes/    Sliders, Curvas, Filtros
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
