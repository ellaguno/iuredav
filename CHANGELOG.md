# Registro de cambios

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y las
versiones, [SemVer](https://semver.org/lang/es/).

Para publicar una versión:

```bash
python3 scripts/version.py 0.2.0     # cambia la versión en los tres ficheros
# escribe aquí la sección de 0.2.0
git commit -am "Versión 0.2.0"
git tag -a v0.2.0 -m "IureDav 0.2.0" && git push --follow-tags
```

La etiqueta dispara la construcción para las tres plataformas y publica una release
con los instaladores. Si la etiqueta y la versión del código no coinciden, el
proceso se detiene antes de construir nada.

## [Sin publicar]

### Corregido

- **«No se pudo crear» cuando lo que había era un montaje sin cerrar.** Si la
  aplicación muere sin desmontar —un cierre de sesión basta—, la entrada se queda
  en la tabla del sistema sin nadie detrás y esa carpeta ya no se puede abrir ni
  volver a montar. Encima el diagnóstico apuntaba al sitio equivocado:
  `Path::exists()` hace un `stat`, el `stat` falla con `ENOTCONN` y responde
  «no existe», así que IureDav intentaba crear una carpeta que sí estaba ahí.
  Ahora se distingue ese caso, se suelta el montaje huérfano, y si no se puede
  —porque una terminal tenga abierta esa carpeta— se dice eso y qué hacer.
- **Los montajes huérfanos se limpian al arrancar**, en vez de esperar a que el
  usuario los descubra fallando al montar.

## [0.2.0]

Desde la ventana ya se puede comprobar si el servidor acepta subidas —hasta ahora
el modo edición no podía surtir efecto— y los límites de la unidad han dejado de
depender de que el servidor mienta sobre ellos.

### Añadido

- **Volver a comprobar un servidor desde la ventana.** La medición se guardaba en
  el perfil y no caducaba nunca, así que un servidor que arreglara `DELETE` seguía
  mutilado para siempre. Ahora la pantalla de detalle la repite cuando se le pida.
- **Atajo para generar la contraseña de aplicación.** En una conexión de
  Iurefficient, un botón junto al campo abre el perfil de *tu* instancia —la
  dirección sale de lo que acabas de escribir, no de una escrita a mano— donde
  está «Otros» → «Contraseñas de acceso WebDAV» → «Generar».
- **Comprobar las subidas desde la ventana.** El formulario sondea solo la lectura
  a propósito —para no dejar rastro en un servidor que quizá ni se guarde—, pero
  eso dejaba `PUT` en «sin probar», y con eso el montaje se fuerza a solo lectura:
  el modo edición no podía surtir efecto nunca. Al activarlo ahora se ofrece medir
  las subidas, avisando antes del archivo de diagnóstico que eso deja.

### Corregido

- **El aviso de límites desaparecía si el servidor dejaba de mentir.** «No permite
  eliminar documentos, crear carpetas ni mover o renombrar» colgaba de las
  discrepancias, es decir, de que el servidor prometiera en `Allow:` algo que
  luego fallaba. Un servidor honesto sobre sus límites se quedaba sin aviso
  aunque siguiera rechazando esas operaciones. Ahora los límites son su propio
  concepto: entran los que se comprobaron y los que el propio servidor declara no
  ofrecer, así que hay aviso sin necesidad de escribir nada en el servidor. Lo
  que promete y no se ha comprobado sigue sin contar, ni a favor ni en contra.
- **La sonda decía que los bloqueos eran fiables sin haberlos medido.** El segundo
  `LOCK` —el que comprueba si un bloqueo lo ven los demás procesos— iba contra la
  ruta fija de Iurefficient en vez de la del perfil. Con cualquier otro servidor
  esa ruta no existe, el `LOCK` no podía tener éxito, y de ahí salía un «fiables»
  que no se había comprobado.

## [0.1.0]

Primera versión. Monta un servidor WebDAV como una unidad del equipo, con un perfil
optimizado para Iurefficient.

### Añadido

- **Sonda de capacidades.** Descubre lo que el servidor sabe hacer de verdad
  probando cada operación, en vez de fiarse de lo que anuncia en `Allow:`. De esa
  medición salen las opciones de montaje: si el servidor no cumple lo que promete,
  se le retiran esas capacidades a rclone para que no planifique con ellas y falle
  a mitad.
- **Montaje** con caché local. Solo lectura salvo que la sonda confirme la
  escritura y el usuario la active a mano.
- **Interfaz en español** que traduce los límites del servidor a algo accionable:
  «no permite eliminar documentos, crear carpetas ni mover o renombrar», en vez de
  códigos y verbos HTTP.
- **Bandeja del sistema** con montaje por conexión, y autoarranque. Cerrar la
  ventana no desmonta.
- **Carpetas disponibles sin conexión.**
- **Dos tipos de servidor**: Iurefficient y WebDAV genérico (Nextcloud, ownCloud,
  Synology, Seafile…), con autenticación básica sobre HTTPS.
- **Instaladores** para Linux (`.deb`, `.rpm`, `.AppImage`), macOS (un `.dmg`
  universal) y Windows (`.msi`, `.exe`), con rclone 1.75.1 dentro.

### Se sabe que

- macOS y Windows se construyen y empaquetan en integración continua, pero
  **nadie los ha ejecutado todavía** en una máquina real. Linux sí está probado de
  punta a punta.
- Los paquetes de macOS y Windows van **sin firmar**.
- Las carpetas sin conexión son una aproximación: rclone no tiene un «anclar»
  nativo, y si la caché se llena el desalojo por antigüedad puede expulsar
  contenido marcado.

[Sin publicar]: https://github.com/ellaguno/iuredav/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/ellaguno/iuredav/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ellaguno/iuredav/releases/tag/v0.1.0
