# Revela

Editor de fotos móvil, gratuito y de código abierto, construido sobre el motor de PhotoCraft.
Repo: `github.com/nikporras/revela` (fork de `storytold/photocraft`).

## Para qué sirve

1. **Fotos personales** tomadas con el celular: retoques rápidos y bonitos.
2. **Fotos de producto para redes sociales** (por ejemplo, la marca Joker): fondo limpio, texto, formatos listos para Instagram y TikTok.

## Plataforma

- **PWA** instalable en Android y iPhone, desplegada en **GitHub Pages**.
- Funciona **sin conexión** y **todo se procesa en el dispositivo**. Sin nube, sin cuentas, sin servicios de pago.
- Dispositivo principal de pruebas: Samsung Galaxy S25 Ultra (Chrome). Probar también en Safari de iOS cuando sea posible.

## Enfoque técnico

- **Reutilizar el motor de PhotoCraft** (modelo de documento + registro de comandos) compilado a **WebAssembly**.
- **Interfaz nueva, táctil y móvil**. No usar la interfaz de escritorio (egui) de PhotoCraft para el móvil.
- Crear la app nueva en su propia carpeta (por ejemplo `apps/revela/`). **Tocar lo mínimo posible los crates del motor**, para poder seguir trayendo actualizaciones de PhotoCraft.
- Si hace falta cambiar el motor, hacerlo en cambios pequeños y aislados, y anotarlo en `docs/revela-cambios-motor.md`.
- Mantener el remoto `upstream` apuntando a `storytold/photocraft`.

### Rendimiento en móvil (crítico)

- Editar sobre una **vista previa reducida** y aplicar a resolución completa solo al exportar.
- Poner un límite de megapíxeles configurable para evitar que el navegador se quede sin memoria (Safari de iOS es el más estricto).
- Procesar en **Web Workers** para que la interfaz nunca se congele.
- Tener un camino de respaldo si el navegador no soporta WebGPU (el motor tiene compositor por CPU).

## Funciones

### Versión 1 (MVP)

- Abrir foto desde galería o cámara.
- Recortar, enderezar, rotar y voltear.
- Ajustes: exposición/brillo, contraste, sombras/luces, saturación, vibrancia, temperatura, curvas.
- Filtros preestablecidos (pocos y buenos).
- Deshacer y rehacer.
- Exportar JPG y PNG, con calidad ajustable, y compartir directo (Web Share API).

### Versión 2 (producto y redes)

- **Quitar fondo** en el dispositivo (usar la selección de sujeto del motor) y poner fondo de color o degradado.
- **Texto** sobre la foto: fuente, tamaño, color, sombra.
- **Plantillas para redes**: 1:1 (1080x1080), 4:5 (1080x1350), 9:16 (1080x1920).
- Edición en lote de varias fotos con los mismos ajustes (útil para catálogos de producto).

### Después

- Marca de agua o logo propio.
- Guardar ajustes favoritos como preset.

## Diseño de la interfaz

- Pensada para usar con una mano: controles abajo, foto arriba.
- Botones grandes, sliders cómodos, gestos de pellizco para zoom y arrastre.
- Mantener pulsado para comparar antes y después.
- Tema oscuro por defecto. Interfaz en **español**.

## Licencia y marca (obligatorio)

- PhotoCraft es MIT o Apache 2.0: **conservar** `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` y `ATTRIBUTION.md`.
- **Quitar el nombre, wordmark y logos de ArtCraft** (`docs/brand/` y cualquier uso en la app Revela), como exige su licencia de marca.
- No usar marcas de Adobe ni "Craft" en el nombre o la interfaz de Revela.
- Incluir un crédito visible: "Basado en PhotoCraft (código abierto)".

## Entorno

- Windows 11 con **WSL2 Ubuntu**. Repo en `~/revela`.
- PC: Ryzen 9 5900X, 64 GB RAM, RTX 4070 Ti SUPER.

## Cómo trabajar conmigo

- Háblame en **español**.
- Avanza **un paso a la vez** y dime qué hiciste en pocas líneas.
- **Pídeme permiso antes de usar `sudo`**, borrar archivos o hacer cambios grandes en el motor.
- Commits pequeños y claros. Trabaja en ramas y fusiona a `main` cuando algo funcione.
- Cuando algo esté listo para probar, despliega en GitHub Pages y dame el enlace para abrirlo en el celular.
- Prefiero archivos completos y listos para usar, no fragmentos sueltos.

## Fases

1. **Preparación:** leer `AGENTS.md`, `docs/` y `book/`; instalar Rust, el target `wasm32` y las herramientas de build web; compilar la versión web actual para confirmar que funciona.
2. **Plan:** proponer arquitectura de Revela (cómo se conecta la interfaz con el motor WASM, qué framework de interfaz, cómo se despliega). Esperar mi aprobación.
3. **Esqueleto:** PWA vacía instalable en GitHub Pages, que abre una foto y la exporta.
4. **MVP:** funciones de la versión 1.
5. **Producto y redes:** funciones de la versión 2.
6. **Pulido:** rendimiento, pruebas en Android e iOS, iconos y nombre de Revela.

## Estado actual (17 de octubre de 2026)

En línea: https://nikporras.github.io/revela/ (versión `34d0947`). Cada push a `main` pasa las pruebas, incluida la de igualdad entre la previa en la GPU y la exportación, y si pasan se despliega.

### Hecho

- **Fases 1 a 3:** preparación, plan aprobado y esqueleto PWA. Ver `docs/revela-arquitectura.md`.
- **Fase 4 (MVP, Versión 1), completa:**
  - Abrir desde galería o cámara, respetando la orientación EXIF.
  - Recortar con 7 proporciones, enderezar sin esquinas vacías (rueda o "Con línea"), girar 90° y voltear.
  - Ajustes, curvas y 25 filtros (traducidos de LightCraft, con atribución).
  - Deshacer y rehacer, y mantener pulsado para comparar.
  - Exportar JPG o PNG (calidad 90 por defecto, con barra de progreso), compartir y guardar en carpeta. Sin GPS por defecto.
  - Zoom: pellizco, arrastre con un dedo, doble toque entre ajustar a pantalla y 100 %, y zona nítida a resolución completa. Los sliders tienen prioridad sobre la zona nítida.
  - Con el botón Cámara, la exportación se nombra por fecha y aparece un aviso de calidad que se puede cerrar.
- **Probado en el S25 Ultra:**
  - Ajustes en vivo: 2 ms de mediana.
  - "Probar GPU": diferencia máxima 1/255.
  - Recortar y exportar, guardar en carpeta, compartir y cámara.
  - Memoria al exportar igual que al abrir (unos 180 MB).

- **Zoom en el S25 (18 de octubre):** pellizco fluido, zona nítida al soltar, sliders bien con zoom y asas de recorte bien con zoom.

### Falta probar

- **Botón Cámara:** el nombre del archivo exportado y el aviso con X (no debe volver a salir una vez cerrado).
- **Safari de iOS:** todavía sin probar.

### Lo siguiente: herramientas de color y efectos (plan aprobado el 18 de octubre)

Pestañas: **Recortar · Filtros · Editar**. Editar tiene las secciones **Luz, Color, Efectos y Curvas** (como Lightroom móvil).

1. **Color:** HSL (8 colores × tono, saturación y luminancia) y gradación (sombras, medios, luces, global, mezcla y balance). Junto con la nueva estructura de pestañas.
2. **Viñeta** (Efectos), sobre el encuadre final. Copiar la fórmula del motor en Revela con coordenadas globales y comprobarla contra el motor.
3. **Grano** (Efectos). El ruido lo calcula el motor como textura; es aproximado a tamaño de pantalla y exacto al 100 %, con la nota "Míralo al 100 % (doble toque)".
4. **Volver a traducir los filtros de LightCraft:** vuelven 6 de los 16 excluidos (los que dependen de viñeta y grano).
5. **Nitidez** (Efectos). Bloques con margen al exportar y varias pasadas en la GPU. Aproximada a tamaño de pantalla y exacta al 100 %, con la misma nota.
6. **Bloque posterior:** textura primero, luego claridad y dehaze, para recuperar los otros 10 filtros.

### Después: Fase 5 (Versión 2)

1. **Quitar fondo** en el dispositivo, con fondo de color o degradado. La API ya está diseñada para poder cambiar después a un modelo de IA (ver la arquitectura).
2. **Texto** sobre la foto: fuente, tamaño, color y sombra. Fuentes OFL servidas por el motor (decisión en la arquitectura).
3. **Plantillas para redes:** 1:1 (1080×1080), 4:5 (1080×1350) y 9:16 (1080×1920).
4. Edición en lote (después de las tres anteriores).
