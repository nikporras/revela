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

## Primera tarea

Fases 1 y 2. No escribas código de la app todavía: instala, compila la versión web de PhotoCraft y preséntame el plan.
