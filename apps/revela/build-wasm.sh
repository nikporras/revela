#!/usr/bin/env bash
# Compila revela-core a WebAssembly y deja el módulo en web/src/wasm/ para Vite.
#
#   apps/revela/build-wasm.sh            # release (perfil wasm-release del workspace)
#   PROFILE=dev apps/revela/build-wasm.sh
#
# Necesita wasm-bindgen (la misma versión que el crate en Cargo.lock) y, opcionalmente, wasm-opt.
# Si no están en el PATH, usa los que descargó trunk en ~/.cache/trunk.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
profile="${PROFILE:-wasm-release}"
out="$here/web/src/wasm"

find_tool() {
  command -v "$1" 2>/dev/null || find "$HOME/.cache/trunk" -type f -name "$1" 2>/dev/null | sort | tail -1
}
bindgen="$(find_tool wasm-bindgen)"
wasm_opt="$(find_tool wasm-opt)"
[ -n "$bindgen" ] || { echo "falta wasm-bindgen: cargo install wasm-bindgen-cli --version $(grep -A1 'name = "wasm-bindgen"$' "$root/Cargo.lock" | sed -n 's/version = "\(.*\)"/\1/p')" >&2; exit 1; }

# El perfil wasm-release de PhotoCraft compila los crates de píxeles para ocupar poco ("z"),
# porque su app web compone en la GPU. Revela procesa en la CPU, así que aquí van a velocidad.
# Se hace con --config para no tocar el Cargo.toml del workspace (upstream).
pixel_opt="${REVELA_PIXEL_OPT:-3}"
config=()
for pkg in photocraft-algo photocraft-compose photocraft-cms; do
  config+=(--config "profile.wasm-release.package.$pkg.opt-level=$pixel_opt")
done

cd "$root"
cargo build "${config[@]}" -p revela-core --lib --target wasm32-unknown-unknown --profile "$profile"
dir="$profile"; [ "$profile" = dev ] && dir=debug
rm -rf "$out"
"$bindgen" --target web --no-typescript --out-dir "$out" "target/wasm32-unknown-unknown/$dir/revela_core.wasm"
if [ -n "$wasm_opt" ] && [ "$profile" != dev ]; then
  "$wasm_opt" -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals \
    "$out/revela_core_bg.wasm" -o "$out/revela_core_bg.wasm"
fi
ls -l "$out"
