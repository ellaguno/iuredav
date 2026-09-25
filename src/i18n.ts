/**
 * Idioma de la interfaz: inglés por defecto y español si el sistema está en
 * español o el usuario lo elige. El idioma lo decide Rust (`ui_language`), que es
 * quien conoce el ajuste y el idioma del sistema; aquí nunca se mira el navegador.
 *
 * `en` es la fuente de verdad: `es` tiene que tener exactamente las mismas claves,
 * y el compilador lo exige.
 */
import { useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";

export type Idioma = "en" | "es";

const en = {
  // Cabecera y navegación
  "cabecera.lema": "Your documents, as just another folder on your computer",
  "cabecera.anadir": "Add connection",
  "cabecera.volver": "Back",

  // Requisitos del sistema
  "falta.titulo": "{que} is missing",
  "falta.queHacer": "What to do",
  "falta.descargar": "Download {que}",

  // Versión nueva
  "nueva.titulo": "A new version of IureDav is available: {version}",
  "nueva.texto":
    "It can be installed from here (AppImage, Windows and macOS): the drives are unmounted, the update is installed and IureDav restarts. If you installed it with .deb or .rpm, download the new package.",
  "nueva.comprobando": "Checking…",
  "nueva.error":
    "Couldn't update from the app ({error}). Download the installer from the release page.",
  "nueva.actualizar": "Update now",
  "nueva.verDescarga": "See the download",
  "nueva.ahoraNo": "Not now",
  "nueva.instalando": "Downloading and installing…",

  // Acciones que la unidad no permite
  "accion.DELETE": "delete documents",
  "accion.MKCOL": "create folders",
  "accion.MOVE": "move or rename",
  "accion.COPY": "copy within the drive",
  "lista.ni": " or ",
  "donde.gestor": "in {gestor}",
  "donde.generico": "in your server's web app",

  // Detalle de una conexión
  "detalle.tituloMedir":
    "The measurement is saved and doesn't expire: if the server changes, you need to check it again.",
  "detalle.comprobando": "Checking…",
  "detalle.volverAComprobar": "Check again",
  "detalle.comprobarAhora": "Check now?",
  "detalle.comprobarSubidas": "Also check whether it accepts uploads",
  "detalle.sinComprobar":
    "This connection hasn't been checked yet. It will happen on its own the first time you mount it.",
  "avisoEscritura":
    "A diagnostic file will be created on the server ({ruta}). If the server doesn't allow deleting —as with Iurefficient— that file stays there. Repeating the check doesn't pile up files, only versions of the same one.",

  // Modo edición
  "editar.hayQueComprobar": "To be able to edit, first we need to check that this server accepts uploads.",
  "editar.comprobarloAhora": "Check it now?",
  "editar.noAceptaSubidas":
    "This server doesn't accept uploads ({estado}), so the folder will stay read-only.",
  "editar.versiona": "On this server, every time you save a document a new version will be created. ",
  "editar.seguirasSinPoder": "And from the folder you still won't be able to {limites}. ",
  "editar.activar": "Turn on edit mode?",
  "olvidar.antesDesmonta": "Unmount the connection before removing it.",
  "enlace.noHay": "There's no connection \"{perfil}\" to mount.",

  // Lista
  "lista.cargando": "Loading…",
  "vacio.titulo": "There are no connections yet",
  "vacio.texto":
    "Add your Iurefficient instance and it will show up as a folder on your computer, with your cases inside.",
  "vacio.boton": "Add my first connection",

  // Preferencias
  "pref.idioma": "Language / Idioma",
  "pref.idioma.auto": "Automatic (system)",
  "pref.idioma.detalle": "Changes the language of the window and the tray menu.",
  "pref.autoarranque": "Start when you sign in",
  "pref.autoarranque.detalle":
    "IureDav stays in the system tray. Closing the window doesn't stop it or unmount anything; that's what \"Quit\" in the tray is for.",
  "pref.minimizado": "Start minimized",
  "pref.minimizado.detalle":
    "When you sign in it doesn't open the window: only the tray icon appears. If you open it yourself, the window is always shown.",
  "pref.avisar": "Notify me of new versions",
  "pref.avisar.detalle":
    "Once a day it checks GitHub for a newer version and says so here and in the tray. It doesn't download or install anything.",

  // Tarjeta de conexión
  "conexion.montado": "Mounted",
  "conexion.desmontado": "Unmounted",
  "conexion.edicion": "Editing",
  "conexion.soloLectura": "Read-only",
  "conexion.tituloActualizar":
    "Reads the server listing again. Changes made in Iurefficient take a few minutes to show up on their own.",
  "conexion.actualizar": "Refresh",
  "conexion.abrirCarpeta": "Open folder",
  "conexion.montar": "Mount",
  "conexion.desmontar": "Unmount",
  "conexion.limites.titulo": "This drive has limits",
  "conexion.limites.texto":
    "It doesn't let you {lista}. Those operations are done {donde}; they don't work from the folder on your computer.",
  "conexion.verCapacidades": "See what this server can do",
  "conexion.pasarASoloLectura": "Switch to read-only",
  "conexion.permitirEdicion": "Allow editing",
  "conexion.eliminar": "Remove",

  // Sin conexión
  "anclajes.titulo": "Available offline",
  "anclajes.pista":
    "Mark the folders you want to be able to open without internet. They're downloaded and kept in your computer's cache.",
  "anclajes.descargando_one": "downloading… {n} file",
  "anclajes.descargando_other": "downloading… {n} files",
  "anclajes.archivos_one": "{n} file",
  "anclajes.archivos_other": "{n} files",
  "anclajes.sinDescargar": ", {n} not downloaded",
  "anclajes.lista": "ready",
  "anclajes.quitar": "Remove",
  "anclajes.anadir": "Add folder",
  "anclajes.elegir": "Choose a folder to keep available offline",

  // Pie
  "pie.lema": "understands every project from its case file, with an AI that reads and cites it.",
  "pie.demo": "Try the demo",
  "pie.nueva": " · a new version is available: {version}",
  "pie.todas": "All versions on GitHub",

  // Formulario de conexión
  "form.sesionComo": "Signed in as {nombre}. ",
  "form.sesion": "Signed in. ",
  "form.reutilizada": "The app password saved in the keychain was reused.",
  "form.creada": "An app password was created in this computer's name.",
  "form.titulo": "New connection",
  "form.tipo": "Server type",
  "form.nombre": "Name",
  "form.nombre.pista": "How you want to see it in the list and in your folder.",
  "form.url": "Server address",
  "form.url.ejemploIurefficient": "your-instance.iurefficient.com",
  "form.url.ejemploGenerico": "https://cloud.example.com/remote.php/dav/files/your-username/",
  "form.url.pistaAntes": "The domain is enough: ",
  "form.url.pistaDespues": " is added automatically.",
  "form.correo": "Your email",
  "form.correo.ejemplo": "name@firm.com",
  "form.acceso": "Access",
  "form.acceso.cuenta": "With my Iurefficient account (recommended)",
  "form.acceso.manual": "I already have an app password",
  "form.codigo": "Verification code",
  "form.passCuenta": "Iurefficient password",
  "form.codigo.ejemplo": "6 digits",
  "form.passCuenta.ejemplo": "The same one you use to sign in on the web",
  "form.passCuenta.pista":
    "Your password isn't stored: IureDav signs in, asks the instance for an app password in this computer's name and saves it in the system keychain, shared with IureTranscribe, IureEditor and IureOCR.",
  "form.conectando": "Connecting…",
  "form.verificar": "Verify",
  "form.conectar": "Connect and get access",
  "form.pass": "Password",
  "form.perfil.tituloSinUrl": "Enter your instance's address first and the link will point to yours.",
  "form.perfil.abrir": "Open my profile to generate it →",
  "form.punto": "Folder where it will appear",
  "form.cancelar": "Cancel",
  "form.comprobando": "Checking…",
  "form.probar": "Test connection",
  "form.guardando": "Saving…",
  "form.guardar": "Save",
  "form.correcta": "Connection successful",
  "form.encontrado":
    "Found: {raiz}. This is what the server can really do:",
  "form.sinCarpetas": "(no folders at the root)",

  // Panel de capacidades
  "caps.explica.DELETE": "Documents can't be deleted from the drive. Do it {sitio}.",
  "caps.explica.MKCOL": "Folders are created {sitio}, not from the drive.",
  "caps.explica.MOVE": "Files can't be moved or renamed from the drive.",
  "caps.explica.PROPPATCH":
    "The modification date can't be written, so it can't be used to detect changes.",
  "caps.explica.otro": "The server rejects {verbo}.",
  "caps.que.PROPFIND": "List folders",
  "caps.que.GET": "Open documents",
  "caps.que.PUT": "Upload documents",
  "caps.que.DELETE": "Delete",
  "caps.que.MKCOL": "Create folders",
  "caps.que.MOVE": "Move or rename",
  "caps.que.PROPPATCH": "Set the date",
  "caps.que.LOCK": "Lock while editing",
  "caps.discrepa.titulo": "This server advertises things it doesn't deliver",
  "caps.discrepa.texto":
    "It claims it can {lista}, but it fails when it tries. IureDav has taken those away so it doesn't attempt them and leave operations half-done.",
  "caps.comprobado": "What was checked",
  "caps.operacion": "Operation",
  "caps.anuncia": "Advertised",
  "caps.funciona": "Really works",
  "caps.si": "yes",
  "caps.no": "no",
  "caps.detalles": "Details that affect syncing",
  "caps.etag": "Content fingerprint (ETag)",
  "caps.etag.si": "available",
  "caps.etag.no": "missing — a change can't be detected by content",
  "caps.rangos": "Partial reads",
  "caps.rangos.si": "yes — large files open without downloading them entirely",
  "caps.sobrescribir": "Saving over an existing document",
  "caps.sobrescribir.version": "creates a new version, doesn't overwrite",
  "caps.sobrescribir.si": "overwrites",
  "caps.sinComprobar": "not checked",
  "caps.locks": "Locks between programs",
  "caps.locks.no": "not reliable — Office may fail now and then",
  "caps.locks.si": "reliable",
  "caps.soloLectura.titulo": "Only reading was checked",
  "caps.soloLectura.texto":
    "The write diagnostic creates a test file that, on a server without DELETE, can't be deleted afterwards. That's why it has to be requested separately.",

  // Estado de un verbo
  "verdict.funciona": "works",
  "verdict.rechazado": "rejected ({status})",
  "verdict.roto": "broken ({status})",
  "verdict.sinProbar": "not tested",
  "verdict.error": "error: {detalle}",

  // Apps de Iurefficient
  "apps.titulo": "Iurefficient apps",
  "apps.texto":
    "IureDav mounts your documents as a drive; IureTranscribe transcribes meetings and audio; IureEditor edits documents and uploads versions; IureOCR recognizes the text of scanned documents and images with Tesseract and includes PDF tools. They share the account and the keychain.",
  "apps.estaApp": "this app",
  "apps.instalada": "installed",
  "apps.noInstalada": "not installed",
  "apps.ultima": " · latest {version}",
  "apps.abrir": "Open",
  "apps.descargar": "Download",
};

export type Clave = keyof typeof en;

const es: Record<Clave, string> = {
  "cabecera.lema": "Tus documentos, como una carpeta más de tu equipo",
  "cabecera.anadir": "Añadir conexión",
  "cabecera.volver": "Volver",

  "falta.titulo": "Falta {que}",
  "falta.queHacer": "Qué hacer",
  "falta.descargar": "Descargar {que}",

  "nueva.titulo": "Hay una versión nueva de IureDav: {version}",
  "nueva.texto":
    "Puede instalarse desde aquí (AppImage, Windows y macOS): se desmontan las unidades, se instala y IureDav se reinicia. Si se instaló con .deb o .rpm, descarga el paquete nuevo.",
  "nueva.comprobando": "Comprobando…",
  "nueva.error":
    "No se pudo actualizar desde la app ({error}). Descarga el instalador desde la página de la release.",
  "nueva.actualizar": "Actualizar ahora",
  "nueva.verDescarga": "Ver la descarga",
  "nueva.ahoraNo": "Ahora no",
  "nueva.instalando": "Descargando e instalando…",

  "accion.DELETE": "eliminar documentos",
  "accion.MKCOL": "crear carpetas",
  "accion.MOVE": "mover o renombrar",
  "accion.COPY": "copiar dentro de la unidad",
  "lista.ni": " ni ",
  "donde.gestor": "desde {gestor}",
  "donde.generico": "desde la aplicación web de tu servidor",

  "detalle.tituloMedir":
    "La medición se guarda y no caduca: si el servidor cambia, hay que volver a comprobarlo.",
  "detalle.comprobando": "Comprobando…",
  "detalle.volverAComprobar": "Volver a comprobar",
  "detalle.comprobarAhora": "¿Comprobar ahora?",
  "detalle.comprobarSubidas": "Comprobar también si acepta subidas",
  "detalle.sinComprobar":
    "Esta conexión todavía no se ha comprobado. Se hará sola la primera vez que la montes.",
  "avisoEscritura":
    "Se creará un archivo de diagnóstico en el servidor ({ruta}). Si el servidor no permite eliminar —el caso de Iurefficient— ese archivo se queda ahí. Repetir la comprobación no acumula archivos, solo versiones del mismo.",

  "editar.hayQueComprobar": "Para poder editar hay que comprobar antes que este servidor acepta subidas.",
  "editar.comprobarloAhora": "¿Comprobarlo ahora?",
  "editar.noAceptaSubidas":
    "Este servidor no acepta subidas ({estado}), así que la carpeta seguirá siendo de solo lectura.",
  "editar.versiona": "En este servidor, cada vez que guardes un documento se creará una versión nueva. ",
  "editar.seguirasSinPoder": "Y desde la carpeta seguirás sin poder {limites}. ",
  "editar.activar": "¿Activar el modo edición?",
  "olvidar.antesDesmonta": "Desmonta la conexión antes de eliminarla.",
  "enlace.noHay": "No hay ninguna conexión «{perfil}» que montar.",

  "lista.cargando": "Cargando…",
  "vacio.titulo": "Todavía no hay ninguna conexión",
  "vacio.texto":
    "Añade tu instancia de Iurefficient y aparecerá como una carpeta de tu equipo, con tus casos dentro.",
  "vacio.boton": "Añadir mi primera conexión",

  "pref.idioma": "Idioma / Language",
  "pref.idioma.auto": "Automático (sistema)",
  "pref.idioma.detalle": "Cambia el idioma de la ventana y del menú de la bandeja.",
  "pref.autoarranque": "Arrancar al iniciar sesión",
  "pref.autoarranque.detalle":
    "IureDav se queda en la bandeja del sistema. Cerrar la ventana no lo detiene ni desmonta nada; para eso está «Salir» en la bandeja.",
  "pref.minimizado": "Arrancar minimizado",
  "pref.minimizado.detalle":
    "Al iniciar sesión no abre la ventana: solo aparece el icono en la bandeja. Si lo abres tú, la ventana se muestra siempre.",
  "pref.avisar": "Avisar de versiones nuevas",
  "pref.avisar.detalle":
    "Una vez al día consulta en GitHub si hay una versión más nueva y lo dice aquí y en la bandeja. No descarga ni instala nada.",

  "conexion.montado": "Montado",
  "conexion.desmontado": "Desmontado",
  "conexion.edicion": "Edición",
  "conexion.soloLectura": "Solo lectura",
  "conexion.tituloActualizar":
    "Vuelve a leer el listado del servidor. Los cambios hechos desde Iurefficient tardan unos minutos en aparecer solos.",
  "conexion.actualizar": "Actualizar",
  "conexion.abrirCarpeta": "Abrir carpeta",
  "conexion.montar": "Montar",
  "conexion.desmontar": "Desmontar",
  "conexion.limites.titulo": "Esta unidad tiene límites",
  "conexion.limites.texto":
    "No permite {lista}. Esas operaciones se hacen {donde}; desde la carpeta de tu equipo no funcionan.",
  "conexion.verCapacidades": "Ver qué sabe hacer este servidor",
  "conexion.pasarASoloLectura": "Pasar a solo lectura",
  "conexion.permitirEdicion": "Permitir edición",
  "conexion.eliminar": "Eliminar",

  "anclajes.titulo": "Disponible sin conexión",
  "anclajes.pista":
    "Marca las carpetas que quieras poder abrir sin internet. Se descargan y se guardan en la caché de tu equipo.",
  "anclajes.descargando_one": "descargando… {n} archivo",
  "anclajes.descargando_other": "descargando… {n} archivos",
  "anclajes.archivos_one": "{n} archivo",
  "anclajes.archivos_other": "{n} archivos",
  "anclajes.sinDescargar": ", {n} sin descargar",
  "anclajes.lista": "lista",
  "anclajes.quitar": "Quitar",
  "anclajes.anadir": "Añadir carpeta",
  "anclajes.elegir": "Elige una carpeta para tenerla sin conexión",

  "pie.lema": "entiende cada proyecto desde su expediente, con una IA que lo lee y lo cita.",
  "pie.demo": "Probar la demo",
  "pie.nueva": " · hay una versión nueva: {version}",
  "pie.todas": "Todas las versiones en GitHub",

  "form.sesionComo": "Sesión iniciada como {nombre}. ",
  "form.sesion": "Sesión iniciada. ",
  "form.reutilizada": "Se reutilizó la contraseña de aplicación guardada en el llavero.",
  "form.creada": "Se creó una contraseña de aplicación a nombre de este equipo.",
  "form.titulo": "Nueva conexión",
  "form.tipo": "Tipo de servidor",
  "form.nombre": "Nombre",
  "form.nombre.pista": "Como quieres verla en la lista y en tu carpeta.",
  "form.url": "Dirección del servidor",
  "form.url.ejemploIurefficient": "tu-instancia.iurefficient.com",
  "form.url.ejemploGenerico": "https://nube.ejemplo.com/remote.php/dav/files/tu-usuario/",
  "form.url.pistaAntes": "Basta el dominio: se completa con ",
  "form.url.pistaDespues": ".",
  "form.correo": "Tu correo",
  "form.correo.ejemplo": "nombre@despacho.com",
  "form.acceso": "Acceso",
  "form.acceso.cuenta": "Con mi cuenta de Iurefficient (recomendado)",
  "form.acceso.manual": "Ya tengo una contraseña de aplicación",
  "form.codigo": "Código de verificación",
  "form.passCuenta": "Contraseña de Iurefficient",
  "form.codigo.ejemplo": "6 dígitos",
  "form.passCuenta.ejemplo": "La misma con la que entras a la web",
  "form.passCuenta.pista":
    "Tu contraseña no se guarda: IureDav inicia sesión, pide a la instancia una contraseña de aplicación a nombre de este equipo y la guarda en el llavero del sistema, compartido con IureTranscribe, IureEditor e IureOCR.",
  "form.conectando": "Conectando…",
  "form.verificar": "Verificar",
  "form.conectar": "Conectar y obtener acceso",
  "form.pass": "Contraseña",
  "form.perfil.tituloSinUrl": "Escribe antes la dirección de tu instancia y el enlace apuntará a la tuya.",
  "form.perfil.abrir": "Abrir mi perfil para generarla →",
  "form.punto": "Carpeta donde aparecerá",
  "form.cancelar": "Cancelar",
  "form.comprobando": "Comprobando…",
  "form.probar": "Probar conexión",
  "form.guardando": "Guardando…",
  "form.guardar": "Guardar",
  "form.correcta": "Conexión correcta",
  "form.encontrado":
    "Se encontró: {raiz}. Esto es lo que el servidor sabe hacer de verdad:",
  "form.sinCarpetas": "(sin carpetas en la raíz)",

  "caps.explica.DELETE": "Los documentos no se pueden eliminar desde la unidad. Hazlo {sitio}.",
  "caps.explica.MKCOL": "Las carpetas se crean {sitio}, no desde la unidad.",
  "caps.explica.MOVE": "No se puede mover ni renombrar archivos desde la unidad.",
  "caps.explica.PROPPATCH":
    "La fecha de modificación no se puede escribir, así que no sirve para detectar cambios.",
  "caps.explica.otro": "El servidor rechaza {verbo}.",
  "caps.que.PROPFIND": "Listar carpetas",
  "caps.que.GET": "Abrir documentos",
  "caps.que.PUT": "Subir documentos",
  "caps.que.DELETE": "Eliminar",
  "caps.que.MKCOL": "Crear carpetas",
  "caps.que.MOVE": "Mover o renombrar",
  "caps.que.PROPPATCH": "Fijar la fecha",
  "caps.que.LOCK": "Bloquear mientras se edita",
  "caps.discrepa.titulo": "Este servidor anuncia cosas que no cumple",
  "caps.discrepa.texto":
    "Dice saber {lista}, pero al intentarlo falla. IureDav se lo ha retirado para que no lo intente y deje operaciones a medias.",
  "caps.comprobado": "Lo que se comprobó",
  "caps.operacion": "Operación",
  "caps.anuncia": "Lo anuncia",
  "caps.funciona": "Funciona de verdad",
  "caps.si": "sí",
  "caps.no": "no",
  "caps.detalles": "Detalles que afectan a la sincronización",
  "caps.etag": "Huella de contenido (ETag)",
  "caps.etag.si": "disponible",
  "caps.etag.no": "ausente — no se puede detectar un cambio por contenido",
  "caps.rangos": "Lectura por trozos",
  "caps.rangos.si": "sí — los archivos grandes se abren sin descargarlos enteros",
  "caps.sobrescribir": "Guardar sobre un documento existente",
  "caps.sobrescribir.version": "crea una versión nueva, no sobrescribe",
  "caps.sobrescribir.si": "sobrescribe",
  "caps.sinComprobar": "sin comprobar",
  "caps.locks": "Bloqueos entre programas",
  "caps.locks.no": "no son fiables — Office puede fallar de vez en cuando",
  "caps.locks.si": "fiables",
  "caps.soloLectura.titulo": "Solo se comprobó la lectura",
  "caps.soloLectura.texto":
    "El diagnóstico de escritura crea un archivo de prueba que, en un servidor sin DELETE, después no se puede borrar. Por eso hay que pedirlo aparte.",

  "verdict.funciona": "funciona",
  "verdict.rechazado": "rechazado ({status})",
  "verdict.roto": "roto ({status})",
  "verdict.sinProbar": "sin probar",
  "verdict.error": "error: {detalle}",

  "apps.titulo": "Apps de Iurefficient",
  "apps.texto":
    "IureDav monta tus documentos como una unidad; IureTranscribe transcribe reuniones y audios; IureEditor edita los documentos y sube versiones; IureOCR reconoce el texto de documentos escaneados e imágenes con Tesseract y trae herramientas PDF. Comparten la cuenta y el llavero.",
  "apps.estaApp": "esta app",
  "apps.instalada": "instalada",
  "apps.noInstalada": "no instalada",
  "apps.ultima": " · última {version}",
  "apps.abrir": "Abrir",
  "apps.descargar": "Descargar",
};

export const diccionarios: Record<Idioma, Record<Clave, string>> = { en, es };

let actual: Idioma = "en";
const oyentes = new Set<() => void>();

/** Texto traducido, con `{nombre}` sustituido por `params.nombre`. */
export function t(clave: Clave, params?: Record<string, string | number>): string {
  const texto = diccionarios[actual][clave] ?? en[clave] ?? clave;
  if (!params) return texto;
  return texto.replace(/\{(\w+)\}/g, (m, k: string) => (k in params ? String(params[k]) : m));
}

/** Plural sencillo: `clave_one` si `n` es 1, `clave_other` si no. Pasa `n` como parámetro. */
export function tn(
  clave: string,
  n: number,
  params?: Record<string, string | number>,
): string {
  return t(`${clave}_${n === 1 ? "one" : "other"}` as Clave, { n, ...params });
}

export function idiomaActual(): Idioma {
  return actual;
}

/** Locale para `toLocale*` / `Intl`. */
export function locale(): string {
  return actual === "es" ? "es-MX" : "en-US";
}

export function fijarIdioma(l: string) {
  const nuevo: Idioma = l === "es" ? "es" : "en";
  document.documentElement.lang = nuevo;
  if (nuevo === actual) return;
  actual = nuevo;
  oyentes.forEach((f) => f());
}

function suscribir(f: () => void) {
  oyentes.add(f);
  return () => {
    oyentes.delete(f);
  };
}

/** Idioma actual; el componente se vuelve a pintar cuando cambia. */
export function useIdioma(): Idioma {
  return useSyncExternalStore(suscribir, idiomaActual);
}

/** Pregunta a Rust el idioma resuelto (ajuste o, en automático, el del sistema). */
export async function cargarIdioma(): Promise<void> {
  try {
    fijarIdioma(await invoke<string>("ui_language"));
  } catch {
    fijarIdioma("en");
  }
}
