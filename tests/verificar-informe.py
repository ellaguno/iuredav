#!/usr/bin/env python3
"""Comprueba que un informe de la sonda dice lo que tiene que decir.

Lo usa la integracion continua contra `tests/servidor-falso.py`. Verifica dos
afirmaciones distintas, segun como se hiciera la sonda:

* **Sin escritura** — que la sonda lea el `Allow:` del servidor y aun asi **no de
  por buenas** esas capacidades. Anunciado no es comprobado.
* **Con escritura** — que detecte la mentira: el doble anuncia DELETE, MOVE,
  MKCOL y PROPPATCH, y los cuatro fallan.

    python3 tests/verificar-informe.py informe.json
"""
import json
import sys

# Lo que hace de verdad el doble de `tests/servidor-falso.py`.
ESPERADO_CON_ESCRITURA = {
    "borrar":            ("rechazado", 403),
    "mkcol":             ("rechazado", 403),
    "mover":             ("roto", 502),
    "proppatch_modtime": ("rechazado", 403),
    "put_crear":         ("funciona", None),
}

VERBOS_ANUNCIADOS = ("DELETE", "MOVE", "MKCOL", "PROPPATCH")


def main(ruta: str) -> int:
    c = json.load(open(ruta, encoding="utf-8"))
    real, fallos = c["real"], []

    # El doble anuncia de todo; si no, la prueba no probaria nada.
    allow = [a.upper() for a in c["anunciado"]["allow"]]
    for verbo in VERBOS_ANUNCIADOS:
        if verbo not in allow:
            fallos.append(f"el doble deberia anunciar {verbo} en Allow:, y no lo hace")

    # La lectura funciona en ambos modos.
    for campo, etiqueta in (("get", "GET"), ("propfind_depth1", "PROPFIND Depth 1")):
        if real[campo]["estado"] != "funciona":
            fallos.append(f"{etiqueta} deberia funcionar contra el doble")
    if real["etag"]:
        fallos.append("el doble no da ETag; el informe dice que si")
    if sorted(real["raiz"]) != ["Casos", "General"]:
        fallos.append(f"la raiz deberia ser Casos y General, y es {real['raiz']}")

    if c["sonda_escritura"]:
        for campo, (estado, status) in ESPERADO_CON_ESCRITURA.items():
            v = real[campo]
            if v["estado"] != estado:
                fallos.append(f"{campo}: se esperaba '{estado}' y quedo '{v['estado']}'")
            elif status is not None and v.get("status") != status:
                fallos.append(f"{campo}: se esperaba {status} y llego {v.get('status')}")

        if real["put_sobrescribir"] != "crea_version":
            fallos.append(f"el doble versiona al sobrescribir; el informe dice "
                          f"'{real['put_sobrescribir']}'")
        if real["locks"]["cruza_procesos"] is not False:
            fallos.append("los locks del doble no cruzan procesos y deberia detectarse")
        veredicto = "la sonda detecto que el servidor anuncia cosas que no cumple"
    else:
        # Lo esencial: sin pedirlo, no se toca nada, por mucho que el `Allow:`
        # diga saber hacerlo.
        for campo in ESPERADO_CON_ESCRITURA:
            if real[campo]["estado"] != "sin_probar":
                fallos.append(f"{campo} se probo sin pedir --escritura: quedo en "
                              f"'{real[campo]['estado']}'")
        veredicto = "la sonda leyo el anuncio del servidor y no se lo creyo"

    for f in fallos:
        print(f"  FALLO: {f}", file=sys.stderr)
    if fallos:
        return 1
    print(f"  {veredicto}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "informe.json"))
