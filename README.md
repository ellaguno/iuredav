# IureDav

[![CI](https://github.com/ellaguno/iuredav/actions/workflows/ci.yml/badge.svg)](https://github.com/ellaguno/iuredav/actions/workflows/ci.yml)

Monta tu instancia de Iurefficient como una unidad de tu equipo, al estilo de
Mountain Duck. Linux, macOS y Windows.

> **Estado: en desarrollo.** Funcionan la sonda, el montaje, la interfaz gráfica
> y los instaladores de las tres plataformas. Falta la bandeja del sistema y el
> anclado de carpetas para trabajar sin conexión.
>
> Probado de verdad solo en Linux. Windows y macOS **compilan y se empaquetan en
> integración continua**, pero nadie los ha ejecutado todavía en una máquina real.

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

## Uso

```bash
cargo build --workspace

# 1. Ver qué sabe hacer de verdad tu servidor, y guardar la conexión
IUREDAV_PASS='iurdav_...' ./target/debug/iuredav probe \
  --url https://TU-INSTANCIA/webdav/ \
  --user tu@correo.com \
  --guardar-como trabajo

# 2. Montarlo (Ctrl+C para desmontar)
./target/debug/iuredav mount trabajo

# 3. Gestionar conexiones
./target/debug/iuredav perfiles
./target/debug/iuredav olvidar trabajo
```

La contraseña se guarda en el llavero del sistema, nunca en el fichero de
perfiles. Añade `--escritura` a `probe` para comprobar también `PUT`, `MKCOL`,
`MOVE`, `DELETE`, `PROPPATCH` y `LOCK`.

**El montaje es de solo lectura mientras la sonda no confirme que el servidor
acepta escrituras**, y aun entonces hay que pedirlo con `--escritura`. Contra
Iurefficient eso es lo correcto: borrar y renombrar no funcionan, y cada guardado
crea una versión nueva del documento.

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
IUREDAV_PASS='iurdav_falso' ./target/debug/iuredav probe \
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
| `crates/iuredav-core/src/perfiles.rs` | Conexiones guardadas, sin secretos dentro. |
| `crates/iuredav-core/src/secretos.rs` | Contraseñas en el llavero del sistema. |
| `crates/iuredav-cli` | El binario `iuredav`. |

El montaje lo realiza [rclone](https://rclone.org) como proceso auxiliar,
controlado por su API remota. Ni la contraseña de WebDAV ni las credenciales de
esa API pasan por la línea de órdenes: `/proc/PID/cmdline` lo puede leer
cualquier usuario de la máquina (permisos 444), mientras que el entorno solo su
dueño (400). Los nombres y tipos de las opciones salen de
`rclone rc --loopback options/get`, que es la lista autoritativa: las duraciones
van en nanosegundos, `CacheMode` es un entero y la clave del trozo de lectura es
`ChunkSize`. Equivocarse en cualquiera de esas tres rompe el montaje en silencio.

### Cómo se monta en cada sistema

| Sistema | Mecanismo | Hace falta instalar | Dónde aparece |
|---|---|---|---|
| Linux | FUSE 3 | `fuse3` (lo pide el `.deb`) | `~/Iurefficient` |
| macOS | servidor NFS local de rclone | **nada** | `~/Iurefficient` |
| Windows | WinFsp | [WinFsp](https://winfsp.dev/rel/) | unidad `I:` |

En macOS no hace falta macFUSE. rclone levanta un servidor NFS local y el sistema
lo monta, así que nadie tiene que instalar una extensión del núcleo ni autorizarla
en Preferencias del Sistema — que es la mayor fricción de instalación de este tipo
de programas.

El mecanismo **no está escrito a mano**: al montar se le pregunta a rclone qué
mecanismos tiene (`mount/types`) y se elige el primero de una lista de preferencia
por plataforma. Los nombres cambian entre versiones — `nfsmount` no existe en
rclone 1.60 y sí en 1.75— y pedir uno que no está da un error que no orienta.

IureDav comprueba estos requisitos **antes** de intentar montar, y si falta alguno
explica cuál y cómo conseguirlo, en vez de dejar que rclone falle con un mensaje
del sistema.

## La aplicación de escritorio

```bash
npm install
npm run tauri dev
```

La interfaz traduce los límites del servidor a algo accionable: en vez de
«no permite delete, mkcol, move» dice **«no permite eliminar documentos, crear
carpetas ni mover o renombrar; esas operaciones se hacen desde Iurefficient»**.
La pantalla *Ver qué sabe hacer este servidor* enfrenta, fila a fila, lo que el
servidor anuncia con lo que cumple.

## Construir los instaladores

```bash
python3 scripts/descargar-rclone.py    # el rclone que se empaqueta
npm ci
npm run tauri build
```

Produce `.deb`, `.rpm` y `.AppImage` en Linux; `.dmg` en macOS; `.msi` y `.exe` en
Windows. La integración continua los construye para las cuatro combinaciones
(Linux x86-64, macOS Intel, macOS Apple Silicon, Windows x86-64) y los deja como
artefactos de cada ejecución.

rclone viaja dentro del paquete con el nombre **`iuredav-rclone`**, no `rclone`.
Los binarios externos acaban en `/usr/bin`, y ahí `rclone` a secas chocaría con el
paquete de la distribución: dpkg se niega a sobrescribir un fichero de otro
paquete, así que la instalación fallaría en cualquier equipo que ya lo tenga.

Los paquetes de macOS y Windows van **sin firmar**. macOS avisará de que la
aplicación no está identificada, y Windows mostrará SmartScreen. Firmarlos
requiere una cuenta de Apple Developer y un certificado de firma de código.

## Desarrollo

```bash
cargo test --workspace     # capacidades, traductor de errores, perfiles
cargo build --workspace
npx tsc --noEmit           # el frontend, en modo estricto
python3 scripts/generar-iconos.py   # regenera los iconos desde el código
```

En Linux, la bandeja del sistema necesita un paquete que aún no está instalado:

```bash
sudo apt install libayatana-appindicator3-dev
```

## Aviso sobre datos

Este repositorio es público. No incluyas nunca URLs de instancias, correos,
contraseñas `iurdav_…` ni informes de la sonda: los listados de `Casos/` llevan
nombres de expedientes reales. Los tests usan el servidor falso, nunca una
instancia real.

## Licencia

MIT — ver [LICENSE](LICENSE).
