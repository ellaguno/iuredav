#!/usr/bin/env python3
"""Genera los iconos de la aplicacion a partir de assets/iuredav_icon.png.

    python3 scripts/generar-iconos.py

La imagen de origen lleva la marca arriba y el logotipo "WebDAVs" debajo. A 16 y
24 pixeles —bandeja del sistema, barra de tareas, pestana del navegador— ese texto
es una mancha ilegible, y ademas roba un tercio de la altura, con lo que la marca
queda apretada.

Por eso los tamanos pequenos usan **solo la marca** y los grandes la imagen
completa. Es lo que hacen las aplicaciones del sistema, y los formatos de icono lo
contemplan: un .ico y un .icns pueden llevar arte distinto en cada resolucion.
"""
import struct
from collections import Counter
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageDraw

RAIZ = Path(__file__).resolve().parent.parent
ORIGEN = RAIZ / "assets" / "iuredav_icon.png"
DESTINO = RAIZ / "src-tauri" / "icons"
# La cabecera de la ventana tambien lleva la marca, para que coincida con el icono
# del sistema en vez de ser un dibujo aparte.
MARCA_UI = RAIZ / "src" / "assets" / "marca.png"

# Margen alrededor del dibujo. Sin algo de aire el icono se ve pegado al borde.
AIRE = 0.06
# Por debajo de esto, el logotipo no se lee: se usa solo la marca.
UMBRAL_MARCA = 64
# Radio de las esquinas, en proporcion al lado. 0.22 es lo que usan los iconos de
# macOS y los adaptativos de Android: por debajo de 0.15 no se aprecia y por
# encima de 0.30 el dibujo empieza a perder las esquinas.
RADIO = 0.22


def color_de_fondo(img: Image.Image) -> tuple:
    """El color que domina el borde. La imagen de origen no es transparente."""
    w, h = img.size
    c = Counter()
    for x in range(w):
        c[img.getpixel((x, 0))[:3]] += 1
        c[img.getpixel((x, h - 1))[:3]] += 1
    for y in range(h):
        c[img.getpixel((0, y))[:3]] += 1
        c[img.getpixel((w - 1, y))[:3]] += 1
    return c.most_common(1)[0][0]


def tinta_por_fila(img: Image.Image, fondo: tuple, umbral: int = 30) -> list[int]:
    w, h = img.size
    px = img.load()
    return [
        sum(1 for x in range(w) if sum(abs(a - b) for a, b in zip(px[x, y][:3], fondo)) > umbral)
        for y in range(h)
    ]


def separar_logotipo(img: Image.Image, fondo: tuple) -> Image.Image:
    """Devuelve solo la marca, cortando por el hueco que la separa del texto.

    Se busca la franja sin tinta en la mitad inferior. Si no la hay —porque
    alguien cambie el arte de origen— se devuelve la imagen entera, que es peor
    pero nunca corta por un sitio arbitrario.
    """
    h = img.size[1]
    perfil = tinta_por_fila(img, fondo)

    inicio, fin = int(h * 0.45), int(h * 0.85)
    hueco_ini = None
    mejor = None
    for y in range(inicio, fin):
        if perfil[y] <= 2:
            if hueco_ini is None:
                hueco_ini = y
        elif hueco_ini is not None:
            if mejor is None or (y - hueco_ini) > (mejor[1] - mejor[0]):
                mejor = (hueco_ini, y)
            hueco_ini = None

    if mejor is None:
        return img
    corte = (mejor[0] + mejor[1]) // 2
    return img.crop((0, 0, img.size[0], corte))


def recortar(img: Image.Image, fondo: tuple, tolerancia: int = 12) -> Image.Image:
    """Quita el margen uniforme que rodea al dibujo."""
    w, h = img.size
    px = img.load()

    def vacia(coords):
        return all(sum(abs(a - b) for a, b in zip(px[x, y][:3], fondo)) < tolerancia
                   for x, y in coords)

    arriba, abajo, izq, der = 0, h - 1, 0, w - 1
    while arriba < abajo and vacia([(x, arriba) for x in range(0, w, 2)]):
        arriba += 1
    while abajo > arriba and vacia([(x, abajo) for x in range(0, w, 2)]):
        abajo -= 1
    while izq < der and vacia([(izq, y) for y in range(arriba, abajo, 2)]):
        izq += 1
    while der > izq and vacia([(der, y) for y in range(arriba, abajo, 2)]):
        der -= 1
    return img.crop((izq, arriba, der + 1, abajo + 1))


def cuadrar(img: Image.Image, fondo: tuple) -> Image.Image:
    """Centra el dibujo en un lienzo cuadrado, con algo de aire alrededor."""
    w, h = img.size
    lado = int(max(w, h) * (1 + 2 * AIRE))
    lienzo = Image.new("RGBA", (lado, lado), (*fondo, 255))
    lienzo.paste(img, ((lado - w) // 2, (lado - h) // 2), img)
    return lienzo


def redondear(img: Image.Image, radio_rel: float = RADIO) -> Image.Image:
    """Recorta las esquinas. Con `radio_rel = 0.5` sale un circulo.

    La mascara se dibuja a 4x y se reduce despues: `ImageDraw` no suaviza bordes,
    y sin ese paso la curva sale escalonada justo en los tamanos pequenos, que es
    donde mas se nota.

    Se aplica **al tamano final**, nunca antes de reescalar: redondear y luego
    reducir emborrona el borde y deja un halo del color de fondo.
    """
    lado = img.size[0]
    escala = 4
    mascara = Image.new("L", (lado * escala, lado * escala), 0)
    ImageDraw.Draw(mascara).rounded_rectangle(
        (0, 0, lado * escala - 1, lado * escala - 1),
        radius=int(lado * escala * radio_rel),
        fill=255,
    )

    salida = img.convert("RGBA")
    salida.putalpha(mascara.resize((lado, lado), Image.LANCZOS))
    return salida


def a_tamano(arte: Image.Image, px: int, radio_rel: float = RADIO) -> Image.Image:
    """Reescala y redondea, en ese orden."""
    return redondear(arte.resize((px, px), Image.LANCZOS), radio_rel)


def escribir_ico(ruta: Path, imagenes: dict[int, Image.Image]) -> None:
    """Escribe un .ico con arte distinto en cada resolucion.

    Pillow solo sabe reescalar una unica imagen a varios tamanos, y aqui hace
    falta que los pequenos lleven la marca sola. El formato es simple: una
    cabecera, una entrada por tamano y los PNG concatenados.
    """
    tamanos = sorted(imagenes)
    blobs = []
    for t in tamanos:
        b = BytesIO()
        a_tamano(imagenes[t], t).save(b, format="PNG")
        blobs.append(b.getvalue())

    cabecera = struct.pack("<HHH", 0, 1, len(tamanos))
    desplazamiento = len(cabecera) + 16 * len(tamanos)
    entradas = b""
    for t, blob in zip(tamanos, blobs):
        entradas += struct.pack(
            "<BBBBHHII",
            0 if t >= 256 else t,   # 0 significa 256
            0 if t >= 256 else t,
            0, 0, 1, 32, len(blob), desplazamiento,
        )
        desplazamiento += len(blob)

    ruta.write_bytes(cabecera + entradas + b"".join(blobs))


TAMANOS = {
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

# Los mosaicos de la Tienda de Windows se quedan cuadrados. Ahi la forma la pone
# el sistema sobre un fondo de color propio, asi que unas esquinas transparentes
# no se leerian como un icono redondeado sino como un recorte mal hecho.
CUADRADOS = {n for n in TAMANOS if n.startswith("Square")} | {"StoreLogo.png"}

# Icono de la bandeja del sistema y de las notificaciones. Va aparte y **redondo**:
# ahi convive con los iconos del sistema, que en los tres escritorios son
# circulares, y un cuadrado con las esquinas redondeadas canta al lado. Lleva solo
# la marca, porque se dibuja a 22 px.
BANDEJA = "bandeja.png"


def main() -> None:
    if not ORIGEN.exists():
        raise SystemExit(f"no existe {ORIGEN}")

    DESTINO.mkdir(parents=True, exist_ok=True)
    original = Image.open(ORIGEN).convert("RGBA")
    fondo = color_de_fondo(original)

    completo = cuadrar(recortar(original, fondo), fondo)
    marca = cuadrar(recortar(separar_logotipo(original, fondo), fondo), fondo)

    print(f"  origen {original.size[0]}x{original.size[1]}, fondo {fondo}")
    print(f"  completo {completo.size[0]}px · marca sola {marca.size[0]}px")

    for nombre, px in TAMANOS.items():
        arte = marca if px < UMBRAL_MARCA else completo
        if nombre in CUADRADOS:
            arte.resize((px, px), Image.LANCZOS).save(DESTINO / nombre)
        else:
            a_tamano(arte, px).save(DESTINO / nombre)

    # Redondo del todo. 128 px y no 22: el tamano real depende del escritorio y
    # del factor de escala, y reducir una imagen buena sale mejor que agrandar una
    # justa.
    a_tamano(marca, 128, radio_rel=0.5).save(DESTINO / BANDEJA)

    escribir_ico(DESTINO / "icon.ico", {
        16: marca, 32: marca, 48: marca,
        64: completo, 128: completo, 256: completo,
    })

    # La cabecera de la ventana lleva las mismas esquinas que el icono del
    # sistema, asi que el redondeo viene en el PNG y no de una regla de CSS.
    MARCA_UI.parent.mkdir(parents=True, exist_ok=True)
    a_tamano(marca, 128).save(MARCA_UI)

    pequenos = [n for n, px in TAMANOS.items() if px < UMBRAL_MARCA]
    print(f"  {len(TAMANOS) + 2} iconos escritos en {DESTINO}")
    print(f"  esquinas redondeadas al {RADIO:.0%}; {BANDEJA} es un circulo")
    print(f"  cuadrados a proposito (mosaicos de Windows): {len(CUADRADOS)}")
    print(f"  usan solo la marca (el logotipo no se leeria): {', '.join(sorted(pequenos))}")
    print(f"  marca para la cabecera de la ventana: {MARCA_UI.relative_to(RAIZ)}")


if __name__ == "__main__":
    main()
