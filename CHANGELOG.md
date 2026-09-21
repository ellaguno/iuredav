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

## [0.5.0]

### Añadido

- **Conectar con tu cuenta de Iurefficient.** En «Nueva conexión», el acceso
  recomendado ya no pide una contraseña de aplicación: escribes tu correo y tu
  contraseña de Iurefficient (y el código de dos pasos si lo tienes), IureDav
  inicia sesión, le pide a la instancia una contraseña de aplicación a nombre de
  este equipo y la guarda en el llavero del sistema. Tú nunca ves el `iurdav_…`
  ni tienes que abrir tu perfil. La contraseña de la cuenta no se guarda.
- **Llavero compartido con las demás apps.** La contraseña de aplicación y la
  sesión se guardan en la misma entrada que usan IureTranscribe e IureEditor
  (servicio `iurefficient`), así que conectar en una vale para todas. Sigue
  disponible «Ya tengo una contraseña de aplicación» para el caso manual.
- IureDav usa ahora el conector común
  [`iurefficient-connect`](https://github.com/ellaguno/iurefficient-connect).


- **La versión, al pie.** Debajo de los enlaces de Iurefficient, la ventana dice
  qué versión de IureDav es, si hay una más nueva, y enlaza a la página de
  releases en GitHub donde están todas.

## [0.4.0]

### Añadido

- **Avisa de versiones nuevas.** Al arrancar, y luego una vez al día mientras
  siga en la bandeja, consulta la última release publicada en GitHub. Si es más
  nueva, lo dice en la ventana y en el menú de la bandeja, con un enlace a la
  página de descarga. Solo avisa: no descarga ni instala nada, porque sin firma
  de código instalar desde dentro pierde la mitad de la gracia. Se puede apagar
  desde la tarjeta de preferencias, para quien no quiera que el programa hable
  con GitHub por su cuenta.

- **Pie de ventana con Iurefficient.** Una línea con lo que es Iurefficient y
  enlaces a la página principal y a la demo.

### Corregido

- **Ningún enlace abría el navegador, y «Abrir carpeta» no abría nada.** El
  complemento de apertura comprueba cada URL y cada ruta contra un ámbito que
  hay que declarar, y no estaba declarado: los botones fallaban en silencio.
  Las URL llevan ahora el ámbito por defecto (`http`, `https`, `mailto`, `tel`),
  y abrir la carpeta va por Rust, porque el punto de montaje lo elige el usuario
  y no se puede acotar de antemano.
- **En Wayland, los botones de cerrar, minimizar y maximizar no respondían.**
  No era cosa de IureDav: `tao` 0.35, la capa de ventanas de Tauri 2, dibuja en
  Wayland una barra de título propia dentro de una caja que se queda con todos
  los clics, y los botones nunca los reciben (tauri-apps/tauri#13440). `tao` 0.36
  lo arregla quitando esa barra, pero ninguna Tauri estable lo lleva aún, así que
  IureDav la retira por su cuenta antes de mostrar la ventana y deja que GTK ponga
  la suya. En X11 no cambia nada.

## [0.3.0]

### Añadido

- **Arrancar minimizado.** Cuando IureDav arranca con la sesión se queda en la
  bandeja sin abrir la ventana, que es lo que se espera de un agente que monta
  unidades: quien lo puso a arrancar solo no quiere verlo cada mañana. Viene
  activado de fábrica y se desactiva desde la misma tarjeta que el autoarranque.
  Los arranques a mano abren la ventana siempre, y sin bandeja tampoco se esconde
  nunca: una ventana oculta sin icono desde el que recuperarla es un programa al
  que no se puede llegar. La entrada de autoarranque se rehace en cada arranque,
  así que las creadas por versiones anteriores adoptan el comportamiento nuevo sin
  tener que tocar nada.

### Cambiado

- **Los iconos llevan las esquinas redondeadas**, al 22 % como los de macOS, y el
  de la bandeja del sistema es **redondo del todo**: ahí convive con los iconos
  del sistema, que son circulares en los tres escritorios.

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

[Sin publicar]: https://github.com/ellaguno/iuredav/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/ellaguno/iuredav/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ellaguno/iuredav/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ellaguno/iuredav/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ellaguno/iuredav/releases/tag/v0.1.0
