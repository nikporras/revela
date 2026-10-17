#!/usr/bin/env python3
"""Genera los iconos PNG de Revela (provisionales) sin dependencias: un anillo ámbar, como un
diafragma, con un punto en el centro, sobre fondo oscuro.

    python3 apps/revela/scripts/iconos.py apps/revela/web/public/icons
"""
import math
import pathlib
import struct
import sys
import zlib

FONDO = (0x0E, 0x0F, 0x12)
ACENTO = (0xF2, 0xB8, 0x4B)


def png(path, w, h, rgba):
    raw = b"".join(b"\0" + bytes(rgba[y * w * 4:(y + 1) * w * 4]) for y in range(h))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    path.write_bytes(data)


def cobertura(d):
    """Antialias analítico: d es la distancia con signo al borde, en píxeles (negativa dentro)."""
    return min(1.0, max(0.0, 0.5 - d))


def icono(n, esquinas, escala):
    """`esquinas`: radio relativo del cuadrado de fondo (0 = a sangre). `escala`: tamaño del dibujo."""
    out = bytearray(n * n * 4)
    c = n / 2
    r_ext, r_int, r_pto = 0.34 * n * escala, 0.22 * n * escala, 0.075 * n * escala
    rc = esquinas * n
    for y in range(n):
        for x in range(n):
            px, py = x + 0.5 - c, y + 0.5 - c
            # Fondo: cuadrado redondeado.
            if rc > 0:
                qx, qy = abs(px) - (c - rc), abs(py) - (c - rc)
                d = math.hypot(max(qx, 0), max(qy, 0)) + min(max(qx, qy), 0) - rc
                a_fondo = cobertura(d)
            else:
                a_fondo = 1.0
            r = math.hypot(px, py)
            anillo = min(cobertura(r - r_ext), cobertura(r_int - r))
            punto = cobertura(r - r_pto)
            k = max(anillo, punto)
            col = [round(FONDO[i] * (1 - k) + ACENTO[i] * k) for i in range(3)]
            i = (y * n + x) * 4
            out[i:i + 4] = bytes(col + [round(255 * a_fondo)])
    return out


def main():
    dest = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    dest.mkdir(parents=True, exist_ok=True)
    for nombre, n, esquinas, escala in [
        ("icon-192.png", 192, 0.22, 1.0),
        ("icon-512.png", 512, 0.22, 1.0),
        # Maskable: a sangre, con el dibujo dentro de la zona segura (80 % central).
        ("icon-maskable-512.png", 512, 0.0, 0.8),
        # iOS redondea las esquinas por su cuenta.
        ("apple-touch-icon.png", 180, 0.0, 0.9),
    ]:
        png(dest / nombre, n, n, icono(n, esquinas, escala))
        print(dest / nombre)


if __name__ == "__main__":
    main()
