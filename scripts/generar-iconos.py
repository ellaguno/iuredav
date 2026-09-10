#!/usr/bin/env python3
"""Genera los iconos de la aplicacion a partir de una unica marca dibujada aqui.

Se dibuja en codigo, y no como un binario suelto en el repositorio, para que la
marca se pueda ajustar y regenerar sin depender de un editor grafico.

    python3 scripts/generar-iconos.py
"""
from PIL import Image, ImageDraw
from pathlib import Path

DESTINO = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
L = 1024                      # se dibuja grande y se reduce, para bordes suaves
FONDO = (23, 62, 92)          # azul profundo
CLARO = (255, 255, 255)
ACENTO = (86, 197, 178)       # verde agua: el punto de "conectado"


def marca() -> Image.Image:
    """Una nube sobre una unidad de disco: lo que hace la aplicacion, en un glifo."""
    img = Image.new("RGBA", (L, L), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    d.rounded_rectangle([0, 0, L, L], radius=int(L * 0.22), fill=FONDO)

    # Nube: tres circulos que comparten exactamente la misma linea de base, mas un
    # rectangulo que la cierra. Si las bases no coinciden al pixel, el glifo deja un
    # escalon y deja de leerse como una nube.
    cy = L * 0.36
    base = cy + L * 0.13
    for cx, r in ((0.355, 0.115), (0.50, 0.155), (0.645, 0.105)):
        d.ellipse([L*cx - L*r, base - 2*L*r, L*cx + L*r, base], fill=CLARO)
    d.rectangle([L*0.355, base - L*0.10, L*0.645, base], fill=CLARO)

    # Unidad: una barra con su piloto encendido.
    by0, by1 = L*0.605, L*0.735
    d.rounded_rectangle([L*0.20, by0, L*0.80, by1], radius=int(L*0.032), fill=CLARO)
    d.ellipse([L*0.685, by0+L*0.038, L*0.745, by0+L*0.098], fill=ACENTO)

    # Segunda barra, mas corta: sugiere varias carpetas montadas.
    d.rounded_rectangle([L*0.285, L*0.775, L*0.715, L*0.845], radius=int(L*0.026),
                        fill=(*CLARO, 145))
    return img


def main() -> None:
    DESTINO.mkdir(parents=True, exist_ok=True)
    base = marca()

    salidas = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
        "StoreLogo.png": 50,
    }
    for nombre, px in salidas.items():
        base.resize((px, px), Image.LANCZOS).save(DESTINO / nombre)

    # Windows quiere varias resoluciones dentro del mismo .ico.
    base.resize((256, 256), Image.LANCZOS).save(
        DESTINO / "icon.ico",
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    print(f"{len(salidas) + 1} iconos escritos en {DESTINO}")


if __name__ == "__main__":
    main()
