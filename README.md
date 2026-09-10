# IureDav

Monta tu instancia de Iurefficient como una unidad de tu equipo, al estilo de
Mountain Duck. Linux, macOS y Windows.

> **Estado: en desarrollo (Fase 0).** Ahora mismo funciona la sonda de
> diagnóstico. El montaje y la interfaz gráfica están en camino.

## Por qué existe la sonda

El servidor WebDAV de Iurefficient no es un WebDAV sobre archivos: es un proveedor
propio que expone un árbol virtual respaldado por la base de documentos. Y
**anuncia capacidades que no tiene**: su cabecera `Allow:` promete `DELETE`,
`COPY`, `MOVE` y `PROPPATCH`, pero al usarlos devuelve 403, 403, 502 y 403.

Eso no es un detalle cosmético. Un cliente que se cree ese anuncio planifica con
él y falla al ejecutar. Con rclone se ve así:

```
$ rclone backend features :webdav:          # sin protección
  Move     True        ← mentira
  DirMove  True        ← mentira
  Copy     True        ← mentira
  Purge    True        ← mentira

$ rclone moveto ...
ERROR : Server side directory move failed: DirMove MOVE call failed: 502 Bad Gateway
ERROR : Attempt 1/3 failed with 1 errors
```

Por eso IureDav **no lee el anuncio para decidir nada**. Prueba cada verbo, mide
lo que el servidor hace de verdad, y de esa medición deduce cómo montar la
unidad. Si mañana el servidor arregla `DELETE`, la sonda lo detecta y la
restricción desaparece sola.

## Probar la sonda

```bash
cargo build --workspace

IUREDAV_PASS='iurdav_...' cargo run --bin iuredav-probe -- \
  --url https://TU-INSTANCIA/webdav/ \
  --user tu@correo.com
```

Añade `--escritura` para probar también `PUT`, `MKCOL`, `MOVE`, `DELETE`,
`PROPPATCH` y `LOCK`.

> **Ojo con `--escritura`:** como el servidor rechaza `DELETE`, la sonda **no
> puede limpiar lo que crea**. Por eso escribe siempre en la misma ruta fija,
> `General/.iuredav-selftest.txt`, de modo que repetir el diagnóstico genere
> versiones de un único documento en lugar de acumular ficheros huérfanos.

La contraseña se pasa por `IUREDAV_PASS` a propósito: `argv` lo puede leer
cualquier otro proceso de la máquina.

### Sin tocar una instancia real

Hay un doble de pruebas que imita el comportamiento medido, mentira incluida:

```bash
python3 tests/servidor-falso.py 8099 &
IUREDAV_PASS='iurdav_falso' cargo run --bin iuredav-probe -- \
  --url http://127.0.0.1:8099/webdav/ --user prueba@ejemplo.com --escritura
```

Debe reportar 4 capacidades anunciadas que no existen, y terminar con código de
salida 1. Ese desacuerdo entre las dos columnas *es* la prueba de que funciona.

## Cómo está montado

| Pieza | Qué hace |
|---|---|
| `crates/iuredav-core/src/probe.rs` | Prueba cada verbo WebDAV y produce la medición. |
| `crates/iuredav-core/src/caps.rs` | Convierte la medición en opciones de rclone. |
| `crates/iuredav-core/src/errors.rs` | Traduce los fallos de rclone a español llano. |
| `crates/iuredav-core/src/rclone.rs` | Supervisa el sidecar que hace el montaje. |
| `crates/iuredav-cli` | `iuredav-probe`, la herramienta de diagnóstico. |

El montaje lo realiza [rclone](https://rclone.org) como proceso auxiliar,
controlado por su API remota. Los nombres y tipos de las opciones salen de
`rclone rc --loopback options/get`, que es la lista autoritativa: las duraciones
van en nanosegundos, `CacheMode` es un entero y la clave del trozo de lectura es
`ChunkSize`. Equivocarse en cualquiera de esas tres rompe el montaje en silencio.

### Cómo se monta en cada sistema

| Sistema | Mecanismo | Requisito |
|---|---|---|
| Linux | FUSE3 | paquete `fuse3` |
| macOS | servidor NFS local de rclone | **ninguno** — no hace falta macFUSE |
| Windows | WinFsp | lo instala el propio instalador |

## Desarrollo

```bash
cargo test --workspace     # la lógica de capacidades y el traductor de errores
cargo build --workspace
```

En Linux, la bandeja del sistema (Fase 1) necesita
`libayatana-appindicator3-dev`.

## Aviso sobre datos

Este repositorio es público. No incluyas nunca URLs de instancias, correos,
contraseñas `iurdav_…` ni informes de la sonda: los listados de `Casos/` llevan
nombres de expedientes reales. Los tests usan el servidor falso, nunca una
instancia real.

## Licencia

MIT — ver [LICENSE](LICENSE).
