#!/usr/bin/env python3
"""Lee o cambia la version del proyecto en los tres sitios donde vive.

    python3 scripts/version.py            # la ensena y comprueba que coincidan
    python3 scripts/version.py 0.2.0      # la cambia en los tres

La version esta repetida porque cada herramienta tiene la suya: Cargo para los
binarios, npm para el frontend y tauri.conf.json para los instaladores. Esa
ultima es la que acaba en el nombre del fichero y en las propiedades del MSI, asi
que si se desincronizan sale un instalador que dice una version y lleva otra. De
ahi que esto exista y que la integracion continua lo compruebe.
"""
import json
import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
SEMVER = re.compile(r"^\d+\.\d+\.\d+$")


def leer() -> dict[str, str]:
    cargo = (RAIZ / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r'^\[workspace\.package\][^\[]*?^version\s*=\s*"([^"]+)"',
                  cargo, re.M | re.S)
    return {
        "Cargo.toml": m.group(1) if m else "?",
        "package.json": json.loads((RAIZ / "package.json").read_text(encoding="utf-8"))["version"],
        "src-tauri/tauri.conf.json": json.loads(
            (RAIZ / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))["version"],
    }


def escribir(nueva: str) -> None:
    p = RAIZ / "Cargo.toml"
    s = p.read_text(encoding="utf-8")
    s = re.sub(r'(^\[workspace\.package\][^\[]*?^version\s*=\s*")[^"]+(")',
               rf"\g<1>{nueva}\g<2>", s, count=1, flags=re.M | re.S)
    p.write_text(s, encoding="utf-8")

    for ruta in ("package.json", "src-tauri/tauri.conf.json"):
        p = RAIZ / ruta
        d = json.loads(p.read_text(encoding="utf-8"))
        d["version"] = nueva
        p.write_text(json.dumps(d, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def main() -> int:
    actual = leer()

    if len(sys.argv) == 1:
        for k, v in actual.items():
            print(f"  {v:<10} {k}")
        distintas = set(actual.values())
        if len(distintas) != 1:
            print(f"\n  LAS VERSIONES NO COINCIDEN: {sorted(distintas)}", file=sys.stderr)
            print("  Corrigelo con: python3 scripts/version.py <version>", file=sys.stderr)
            return 1
        print(f"\n  version del proyecto: {distintas.pop()}")
        return 0

    nueva = sys.argv[1].lstrip("v")
    if not SEMVER.match(nueva):
        print(f"  '{nueva}' no tiene forma MAYOR.MENOR.PARCHE", file=sys.stderr)
        return 2

    escribir(nueva)
    print(f"  version puesta a {nueva} en los tres ficheros")
    print("  siguiente paso:")
    print(f"    git commit -am 'Version {nueva}' && git tag v{nueva} && git push --follow-tags")
    return 0


if __name__ == "__main__":
    sys.exit(main())
