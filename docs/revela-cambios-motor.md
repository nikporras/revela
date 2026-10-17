# Revela: cambios en el motor de PhotoCraft

Registro de cada cambio que Revela hace fuera de `apps/revela/` y de sus propios archivos, para
poder seguir trayendo actualizaciones de `upstream` (`storytold/photocraft`).

Regla: cambios pequeños y aislados, uno por entrada, con el motivo.

## Cambios en crates del motor

Ninguno por ahora.

## Otros archivos compartidos

| Fecha | Archivo | Cambio | Motivo |
| --- | --- | --- | --- |
| 2026-10-17 | `.github/workflows/revela-pages.yml` | Nuevo | Pruebas, build y despliegue de Revela en GitHub Pages. |

Los workflows heredados de PhotoCraft (`ci.yml`, `release.yml`, etc.) **no se borran**: se
desactivan en GitHub (Actions › workflow › Disable workflow), así las fusiones con `upstream`
no generan conflictos.

## Ajustes de compilación sin tocar archivos del motor

- `apps/revela/build-wasm.sh` compila `photocraft-algo`, `photocraft-compose` y
  `photocraft-cms` con `opt-level=3` mediante `cargo --config`, en lugar de editar el perfil
  `wasm-release` del `Cargo.toml` raíz.
- El crate `revela-core` entra al workspace por el glob `apps/*` que ya existía.
