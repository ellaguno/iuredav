#!/usr/bin/env python3
"""Descarga los binarios de rclone que se empaquetan con la aplicacion.

Se fija una version concreta a proposito: el comportamiento de los flags del VFS
cambia entre versiones, y no queremos que la unidad se comporte distinto segun lo
que el usuario tenga instalado.

Tauri espera los binarios externos nombrados con el triple del objetivo, asi que
se guardan como `src-tauri/binaries/iuredav-rclone-<triple>[.exe]`.

El nombre no es "rclone" a proposito: el .deb instala los binarios externos en
/usr/bin, y ahi chocaria con el paquete `rclone` de la distribucion. dpkg se
niega a sobrescribir un fichero de otro paquete, asi que la instalacion fallaria
en cualquier maquina que ya tenga rclone.

    python3 scripts/descargar-rclone.py              # solo el de esta maquina
    python3 scripts/descargar-rclone.py --todos      # los de las tres plataformas
"""
import argparse
import io
import os
import platform
import stat
import sys
import urllib.request
import zipfile
from pathlib import Path

VERSION = "1.75.1"
# Nombre propio para no colisionar con el rclone del sistema al instalar.
NOMBRE = "iuredav-rclone"
BASE = f"https://downloads.rclone.org/v{VERSION}"
DESTINO = Path(__file__).resolve().parent.parent / "src-tauri" / "binaries"

# triple de Rust -> (nombre del zip de rclone, sufijo del ejecutable)
OBJETIVOS = {
    "x86_64-unknown-linux-gnu":  (f"rclone-v{VERSION}-linux-amd64",   ""),
    "aarch64-unknown-linux-gnu": (f"rclone-v{VERSION}-linux-arm64",   ""),
    "x86_64-apple-darwin":       (f"rclone-v{VERSION}-osx-amd64",     ""),
    "aarch64-apple-darwin":      (f"rclone-v{VERSION}-osx-arm64",     ""),
    "x86_64-pc-windows-msvc":    (f"rclone-v{VERSION}-windows-amd64", ".exe"),
}


def triple_local() -> str:
    m = platform.machine().lower()
    arch = "aarch64" if m in ("arm64", "aarch64") else "x86_64"
    if sys.platform.startswith("linux"):
        return f"{arch}-unknown-linux-gnu"
    if sys.platform == "darwin":
        return f"{arch}-apple-darwin"
    if sys.platform.startswith("win"):
        return "x86_64-pc-windows-msvc"
    raise SystemExit(f"plataforma no contemplada: {sys.platform}")


def descargar(triple: str) -> Path:
    carpeta, sufijo = OBJETIVOS[triple]
    salida = DESTINO / f"{NOMBRE}-{triple}{sufijo}"
    if salida.exists():
        print(f"  ya estaba: {salida.name}")
        return salida

    url = f"{BASE}/{carpeta}.zip"
    print(f"  descargando {url}")
    with urllib.request.urlopen(url, timeout=180) as r:
        datos = r.read()

    with zipfile.ZipFile(io.BytesIO(datos)) as z:
        nombre = f"{carpeta}/rclone{sufijo}"
        salida.write_bytes(z.read(nombre))

    if not sufijo:  # en Unix hay que devolverle el bit de ejecucion
        salida.chmod(salida.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    print(f"  escrito: {salida.name} ({salida.stat().st_size // 1024 // 1024} MB)")
    return salida


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--todos", action="store_true", help="descarga los de todas las plataformas")
    ap.add_argument("--objetivo", help="un triple concreto")
    args = ap.parse_args()

    DESTINO.mkdir(parents=True, exist_ok=True)
    if args.todos:
        objetivos = list(OBJETIVOS)
    elif args.objetivo:
        objetivos = [args.objetivo]
    else:
        objetivos = [triple_local()]

    print(f"rclone v{VERSION} -> {DESTINO}")
    for t in objetivos:
        if t not in OBJETIVOS:
            raise SystemExit(f"objetivo desconocido: {t}")
        descargar(t)


if __name__ == "__main__":
    main()
