#!/usr/bin/env python3
"""Extrae del CHANGELOG las notas de una version, para el cuerpo de la release.

    python3 scripts/notas-release.py 0.2.0

Si no hay seccion para esa version, devuelve un texto minimo en vez de fallar: es
preferible una release con notas escuetas a una publicacion abortada.
"""
import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
CHANGELOG = RAIZ / "CHANGELOG.md"

PIE = (
    "\n\n---\n\n"
    "**Linux** · `.deb`, `.rpm` o `.AppImage`. Necesita `fuse3`.\n"
    "**macOS** · un solo `.dmg` para Intel y Apple Silicon. No hace falta macFUSE.\n"
    "**Windows** · `.msi` o `.exe`. Necesita [WinFsp](https://winfsp.dev/rel/), "
    "que la aplicación te indica si falta.\n\n"
    "Los paquetes de macOS y Windows van sin firmar: macOS avisará de que la "
    "aplicación no está identificada y Windows mostrará SmartScreen."
)


def seccion(version: str) -> str | None:
    if not CHANGELOG.exists():
        return None
    texto = CHANGELOG.read_text(encoding="utf-8")
    # Una seccion va de "## [x.y.z]" hasta el siguiente "## " o el final.
    patron = re.compile(
        rf"^##\s*\[?{re.escape(version)}\]?.*?$\n(.*?)(?=^##\s|\Z)",
        re.M | re.S,
    )
    m = patron.search(texto)
    return m.group(1).strip() if m else None


def main() -> int:
    version = sys.argv[1] if len(sys.argv) > 1 else ""
    cuerpo = seccion(version) or f"Versión {version}."
    print(cuerpo + PIE)
    return 0


if __name__ == "__main__":
    sys.exit(main())
