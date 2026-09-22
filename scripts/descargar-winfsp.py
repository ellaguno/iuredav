#!/usr/bin/env python3
"""Descarga el instalador de WinFsp que se empaqueta dentro del instalador de Windows.

WinFsp es lo que permite a Windows mostrar la unidad (rclone monta sobre el). Se
incluye su MSI oficial, sin modificar, y el instalador de IureDav lo ejecuta en
silencio si la maquina no lo tiene (ver `src-tauri/windows/hooks.nsh`). La
licencia de WinFsp (GPLv3 con excepcion para software libre) permite distribuir
el instalador oficial tal cual junto a software con licencia libre, como IureDav.

Se fija la version a proposito, igual que con rclone: es un driver del nucleo y
no queremos que cambie sin que lo probemos.

    python3 scripts/descargar-winfsp.py
"""
import hashlib
import urllib.request
from pathlib import Path

VERSION = "2.1.25156"
URL = f"https://github.com/winfsp/winfsp/releases/download/v2.1/winfsp-{VERSION}.msi"
# Tamano publicado en la release; si no coincide, algo raro pasa y no se empaqueta.
TAMANO = 2191360
DESTINO = Path(__file__).resolve().parent.parent / "src-tauri" / "windows" / "winfsp.msi"


def main() -> None:
    if DESTINO.exists() and DESTINO.stat().st_size == TAMANO:
        print(f"  ya estaba: {DESTINO}")
        return
    print(f"  descargando {URL}")
    with urllib.request.urlopen(URL, timeout=180) as r:
        datos = r.read()
    if len(datos) != TAMANO:
        raise SystemExit(f"winfsp.msi: tamano inesperado {len(datos)} (se esperaban {TAMANO} bytes)")
    DESTINO.parent.mkdir(parents=True, exist_ok=True)
    DESTINO.write_bytes(datos)
    print(f"  escrito: {DESTINO} ({len(datos) // 1024} KB, sha256 {hashlib.sha256(datos).hexdigest()[:16]}…)")


if __name__ == "__main__":
    main()
