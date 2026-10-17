#!/usr/bin/env python3
"""Traduce los presets de LightCraft a filtros de Revela (ajustes por píxel del Camera Raw de
PhotoCraft) y escribe web/src/lib/filtros.json.

    python3 apps/revela/scripts/traducir_filtros.py [presets.rs]

Sin argumento, descarga crates/engine/src/presets.rs de LightCraft en el commit fijado abajo.
LightCraft es MIT o Apache 2.0 (Copyright ArtCraft Team y colaboradores de LightCraft); los
filtros resultantes llevan la atribución en el JSON, en ATTRIBUTION.md y en la app.

Reglas:
- Exposición, contraste, vibrancia, saturación, curva paramétrica, HSL y gradación de color
  existen igual en Revela: se copian.
- Sombras/Luces/Blancos/Negros de LightCraft son locales; Revela los aproxima con la curva global
  (coeficientes en web/src/lib/tono.json, los mismos que los sliders de la app).
- Balance de blancos en Kelvin (relativo a 6500 K en JPEG): se convierte al slider de
  temperatura de PhotoCraft igualando el cambio rojo/azul de una adaptación Bradford.
- Blanco y negro: saturación −100 (la mezcla de canales B/N de LightCraft no existe aún).
- Claridad, textura, dehaze, grano y viñeta no existen en el MVP: si pesan (≥ 15, viñeta ≥ 25),
  el preset queda fuera; si no, se omiten y el filtro se marca como aproximado.
"""
import json
import math
import pathlib
import re
import sys
import urllib.request

COMMIT = "f34c7a8a89e0d1bade0a6889185f40acecc84a9f"
URL = f"https://raw.githubusercontent.com/storytold/lightcraft/{COMMIT}/crates/engine/src/presets.rs"
AQUI = pathlib.Path(__file__).resolve().parent
TONO = json.loads((AQUI.parent / "web/src/lib/tono.json").read_text())
SALIDA = AQUI.parent / "web/src/lib/filtros.json"

UMBRAL = {"clarity": 15, "texture": 15, "dehaze": 15, "grain": 15, "vignette": 25}
NOMBRES_EFECTO = {"clarity": "claridad", "texture": "textura", "dehaze": "dehaze", "grain": "grano", "vignette": "viñeta"}

NOMBRES = {
    "warm-glow": "Brillo cálido", "cool-morning": "Mañana fresca", "teal-orange": "Turquesa y naranja",
    "vivid-pop": "Vívido", "soft-pastel": "Pastel suave", "faded-matte": "Mate desvaído", "warm-film": "Película cálida",
    "muted-film": "Película apagada", "bw-high-contrast": "B/N alto contraste", "bw-soft": "B/N suave",
    "bw-selenium": "Virado selenio", "crisp-landscape": "Paisaje nítido", "golden-hour": "Hora dorada",
    "blue-hour": "Hora azul", "soft-skin": "Piel suave", "bright-airy": "Luminoso", "moody": "Sombrío",
    "cinematic": "Cine", "moody-portrait": "Retrato sombrío", "golden-glow": "Resplandor dorado",
    "clean-studio": "Estudio limpio", "crisp-vista": "Vista nítida", "deep-sky": "Cielo profundo",
    "lush-greens": "Verdes intensos", "desert-warmth": "Desierto", "misty-morning": "Mañana con niebla",
    "gritty-street": "Calle granulada", "neon-night": "Noche de neón", "concrete-cool": "Hormigón frío",
    "faded-urban": "Urbano desvaído", "fresh-bright": "Fresco y luminoso", "warm-table": "Mesa cálida",
    "dark-moody-food": "Oscuro", "autumn-gold": "Otoño dorado", "winter-blue": "Invierno azul",
    "spring-fresh": "Primavera", "summer-haze": "Bruma de verano", "instant-70s": "Instantánea de los 70",
    "cross-process": "Proceso cruzado", "bleach-bypass": "Blanqueo parcial", "bw-sepia": "Sepia",
}
GRUPOS = {"Color": "Color", "Film": "Película", "B&W": "Blanco y negro", "Landscape": "Paisaje", "Portrait": "Retrato",
          "Style": "Estilo", "Urban": "Urbano", "Food": "Comida", "Seasons": "Estaciones", "Vintage": "Retro"}
BANDAS = ["red", "orange", "yellow", "green", "aqua", "blue", "purple", "magenta"]


def leer_presets(texto):
    cuerpo = texto[texto.index("pub fn builtin()"):]
    cuerpo = cuerpo[:cuerpo.index("\n}\n")]
    items = re.findall(r'p\(\s*"([^"]*)",\s*"([^"]*)",\s*"([^"]*)",\s*json!\((\{.*?\})\),?\s*\)', cuerpo, re.S)
    return [(g, i, n, json.loads(js)) for g, i, n, js in items]


# --- Kelvin → slider de temperatura de PhotoCraft -------------------------------------------

def planck_xy(t):
    """Cromaticidad del cuerpo negro (aproximación de Kim et al., 1667–25000 K)."""
    t = min(max(t, 1667.0), 25000.0)
    if t <= 4000:
        x = -0.2661239e9 / t**3 - 0.2343589e6 / t**2 + 0.8776956e3 / t + 0.179910
    else:
        x = -3.0258469e9 / t**3 + 2.1070379e6 / t**2 + 0.2226347e3 / t + 0.240390
    if t <= 2222:
        y = -1.1063814 * x**3 - 1.34811020 * x**2 + 2.18555832 * x - 0.20219683
    elif t <= 4000:
        y = -0.9549476 * x**3 - 1.37418593 * x**2 + 2.09137015 * x - 0.16748867
    else:
        y = 3.0817580 * x**3 - 5.87338670 * x**2 + 3.75112997 * x - 0.37001483
    return x, y


def mat(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def inv(m):
    (a, b, c), (d, e, f), (g, h, i) = m
    det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    return [[(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det],
            [(f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det],
            [(d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det]]


BRADFORD = [[0.8951, 0.2664, -0.1614], [-0.7502, 1.7135, 0.0367], [0.0389, -0.0685, 1.0296]]
SRGB_A_XYZ = [[0.4124, 0.3576, 0.1805], [0.2126, 0.7152, 0.0722], [0.0193, 0.1192, 0.9505]]


def temperatura_photocraft(kelvin):
    """Slider de temperatura (−100…100) que da el mismo cambio rojo/azul que adaptar la foto de
    6500 K a `kelvin` (lo que hace LightCraft con un JPEG)."""
    def lms(t):
        x, y = planck_xy(t)
        xyz = [x / y, 1.0, (1 - x - y) / y]
        return [sum(BRADFORD[i][k] * xyz[k] for k in range(3)) for i in range(3)]
    a, b = lms(kelvin), lms(6500.0)
    d = [[b[0] / a[0], 0, 0], [0, b[1] / a[1], 0], [0, 0, b[2] / a[2]]]
    m = mat(inv(SRGB_A_XYZ), mat(inv(BRADFORD), mat(d, mat(BRADFORD, SRGB_A_XYZ))))
    g = [sum(m[i]) for i in range(3)]  # un gris neutro
    r = g[0] / g[2]
    return max(-100.0, min(100.0, (r - 1) / (0.35 * (r + 1)) * 100))


# --- Traducción --------------------------------------------------------------------------

def sumar(out, clave, v):
    out[clave] = max(-100.0, min(100.0, out.get(clave, 0.0) + v))


def traducir(s):
    out, omitidos, aprox = {}, {}, []
    luz = s.get("light", {})
    if "exposure" in luz:
        out["exposure"] = luz["exposure"]
    if "contrast" in luz:
        out["contrast"] = luz["contrast"]
    for lc, rv in (("shadows", "sombras"), ("highlights", "luces"), ("whites", "blancos"), ("blacks", "negros")):
        if luz.get(lc):
            for clave, k in TONO[rv].items():
                sumar(out, clave, luz[lc] * k)
            aprox.append({"shadows": "sombras", "highlights": "luces", "whites": "blancos", "blacks": "negros"}[lc] + " globales")
    curva = s.get("curve", {})
    for lc, clave in (("highlights", "curveHighlights"), ("lights", "curveLights"), ("darks", "curveDarks"), ("shadows", "curveShadows")):
        if curva.get(lc):
            sumar(out, clave, curva[lc])
    if any(k in curva for k in ("split_shadows", "split_mid", "split_highlights")):
        out["curveSplits"] = [curva.get("split_shadows", 25.0), curva.get("split_mid", 50.0), curva.get("split_highlights", 75.0)]
    color = s.get("color", {})
    for lc, clave in (("vibrance", "vibrance"), ("saturation", "saturation")):
        if color.get(lc):
            out[clave] = color[lc]
    wb = s.get("wb", {})
    if wb.get("mode") == "custom":
        out["temperature"] = round(temperatura_photocraft(wb.get("temp", 6500.0)), 1)
        if wb.get("tint"):
            out["tint"] = round(wb["tint"] * 100 / 150, 1)
        aprox.append("temperatura convertida de Kelvin")
    mezcla = s.get("mixer", {})
    if mezcla:
        for campo, clave in (("hue", "hslHue"), ("sat", "hslSat"), ("lum", "hslLum")):
            out[clave] = [mezcla.get(b, {}).get(campo, 0.0) for b in BANDAS]
    grad = s.get("grading", {})
    for lc, clave in (("shadows", "gradeShadows"), ("midtones", "gradeMidtones"), ("highlights", "gradeHighlights"), ("global", "gradeGlobal")):
        if lc in grad:
            out[clave] = grad[lc]
    if "blending" in grad:
        out["gradeBlending"] = grad["blending"]
    if "balance" in grad:
        out["gradeBalance"] = grad["balance"]
    if s.get("treatment") == "bw":
        out["saturation"] = -100.0
        out.pop("vibrance", None)
        if s.get("bw_mix"):
            aprox.append("sin mezcla de canales B/N")
    for k, v in s.get("effects", {}).items():
        if v:
            omitidos[k] = v
    if s.get("grain", {}).get("amount"):
        omitidos["grain"] = s["grain"]["amount"]
    if s.get("vignette", {}).get("amount"):
        omitidos["vignette"] = s["vignette"]["amount"]
    return out, omitidos, aprox


def main():
    texto = pathlib.Path(sys.argv[1]).read_text() if len(sys.argv) > 1 else urllib.request.urlopen(
        urllib.request.Request(URL, headers={"User-Agent": "Revela-dev"})).read().decode()
    filtros, excluidos = [], []
    for grupo, ident, nombre, s in leer_presets(texto):
        ajustes, omitidos, aprox = traducir(s)
        pesados = [f"{NOMBRES_EFECTO[k]} {v:+g}" for k, v in omitidos.items() if abs(v) >= UMBRAL[k]]
        base = {"id": ident, "nombre": NOMBRES[ident], "original": nombre, "grupo": GRUPOS[grupo]}
        if pesados:
            excluidos.append({**base, "motivo": "depende de " + ", ".join(pesados)})
            continue
        nota = aprox + [f"sin {NOMBRES_EFECTO[k]} ({v:+g})" for k, v in omitidos.items()]
        filtros.append({**base, "tipo": "aproximado" if nota else "completo", "nota": "; ".join(nota), "ajustes": ajustes})
    salida = {
        "_fuente": f"Presets de LightCraft (https://github.com/storytold/lightcraft, commit {COMMIT}, crates/engine/src/presets.rs), "
                   "MIT o Apache 2.0, Copyright ArtCraft Team y colaboradores de LightCraft. Traducidos a los ajustes de "
                   "PhotoCraft por apps/revela/scripts/traducir_filtros.py; no editar a mano.",
        "filtros": filtros,
        "excluidos": excluidos,
    }
    SALIDA.write_text(json.dumps(salida, ensure_ascii=False, indent=1) + "\n")
    print(f"{len(filtros)} filtros ({sum(f['tipo'] == 'completo' for f in filtros)} completos), {len(excluidos)} excluidos → {SALIDA}")
    for f in filtros:
        print(f"  {'✓' if f['tipo'] == 'completo' else '≈'} {f['nombre']:<24} {f['nota']}")
    for e in excluidos:
        print(f"  ✗ {e['nombre']:<24} {e['motivo']}")


if __name__ == "__main__":
    main()
