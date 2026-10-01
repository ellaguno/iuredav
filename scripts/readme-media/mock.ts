// Backend de Tauri simulado para capturar la interfaz real de IureDav en un navegador.
// Se carga antes que src/main.tsx (ver app.html). Solo datos de demostración:
// servidores ficticios (demo.iurefficient.com, dav.example.com) y rutas /home/demo.
//
// Parámetros de la URL:
//   ?lang=en|es     idioma de la interfaz (lo que respondería Rust en `ui_language`)
//   ?medido=1       la conexión de Iurefficient ya tiene la medición completa
//   ?montado=1      la conexión de Iurefficient arranca montada
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";

mockWindows("main");

const q = new URLSearchParams(location.search);
const lang = q.get("lang") === "es" ? "es" : "en";
const L = (en: string, es: string) => (lang === "es" ? es : en);

// ---- Mediciones ---------------------------------------------------------------
// Lo que `iuredav probe` mide contra el doble de pruebas (tests/servidor-falso.py),
// que imita el WebDAV de Iurefficient: anuncia DELETE, MOVE, MKCOL y PROPPATCH y
// luego los rechaza.
const ALLOW_IURE = ["OPTIONS", "GET", "HEAD", "PROPFIND", "PUT", "DELETE", "COPY", "MOVE", "MKCOL", "PROPPATCH", "LOCK", "UNLOCK"];
const capsIure = (escritura: boolean) => ({
  probed_at: "2026-09-30T16:20:00Z",
  url: "https://demo.iurefficient.com/webdav/",
  anunciado: { allow: ALLOW_IURE, dav: ["1", "2"], server: "WsgiDAV/4.3.3" },
  real: {
    propfind_depth0: { estado: "funciona" },
    propfind_depth1: { estado: "funciona" },
    get: { estado: "funciona" },
    rangos: "soportado",
    etag: false,
    last_modified: true,
    put_crear: escritura ? { estado: "funciona" } : { estado: "sin_probar" },
    put_sobrescribir: escritura ? "crea_version" : "desconocido",
    mkcol: escritura ? { estado: "rechazado", status: 403 } : { estado: "sin_probar" },
    mover: escritura ? { estado: "roto", status: 502 } : { estado: "sin_probar" },
    borrar: escritura ? { estado: "rechazado", status: 403 } : { estado: "sin_probar" },
    proppatch_modtime: escritura ? { estado: "rechazado", status: 403 } : { estado: "sin_probar" },
    locks: escritura ? { lock: { estado: "funciona" }, cruza_procesos: false } : { lock: { estado: "sin_probar" }, cruza_procesos: null },
    raiz: ["Casos", "General"],
  },
  sonda_escritura: escritura,
});

// Un Nextcloud honesto: lo que anuncia, lo cumple.
const capsNextcloud = () => ({
  probed_at: "2026-09-30T16:25:00Z",
  url: "https://dav.example.com/remote.php/dav/files/ana/",
  anunciado: { allow: ["OPTIONS", "GET", "HEAD", "DELETE", "PROPFIND", "PUT", "PROPPATCH", "COPY", "MOVE", "MKCOL", "LOCK", "UNLOCK"], dav: ["1", "3"], server: null },
  real: {
    propfind_depth0: { estado: "funciona" },
    propfind_depth1: { estado: "funciona" },
    get: { estado: "funciona" },
    rangos: "soportado",
    etag: true,
    last_modified: true,
    put_crear: { estado: "sin_probar" },
    put_sobrescribir: "desconocido",
    mkcol: { estado: "sin_probar" },
    mover: { estado: "sin_probar" },
    borrar: { estado: "sin_probar" },
    proppatch_modtime: { estado: "sin_probar" },
    locks: { lock: { estado: "sin_probar" }, cruza_procesos: null },
    raiz: [L("Documents", "Documentos"), L("Photos", "Fotos"), L("Shared", "Compartido")],
  },
  sonda_escritura: false,
});

// ---- Conexiones -----------------------------------------------------------------
const conexiones: any[] = [
  {
    id: "iurefficient",
    nombre: "Iurefficient",
    url: "https://demo.iurefficient.com/webdav/",
    usuario: "ana.torres@example.com",
    preset: "iurefficient",
    punto_montaje: "/home/demo/Iurefficient",
    escritura: false,
    anclados: [
      "/home/demo/Iurefficient/Casos/Ruiz & Asociados — Arrendamiento",
      "/home/demo/Iurefficient/General/Plantillas",
    ],
    capacidades: q.get("medido") === "1" ? capsIure(true) : null,
    montado: q.get("montado") === "1",
    limites: ["DELETE", "MKCOL", "MOVE", "PROPPATCH"],
  },
  {
    id: "nextcloud",
    nombre: L("Office Nextcloud", "Nextcloud del despacho"),
    url: "https://dav.example.com/remote.php/dav/files/ana/",
    usuario: "ana",
    preset: "generico",
    punto_montaje: "/home/demo/Nextcloud",
    escritura: true,
    anclados: [],
    capacidades: capsNextcloud(),
    montado: false,
    limites: [],
  },
];

const presets = [
  {
    id: "iurefficient", nombre: "Iurefficient",
    descripcion: L("Your cases and documents from an Iurefficient instance.", "Tus casos y documentos de una instancia de Iurefficient."),
    sufijo_url: "webdav/", ruta_selftest: "General/.iuredav-selftest.txt", carpeta_muestra: "General/",
    nombre_volumen: "Iurefficient", donde_gestionar: "Iurefficient",
    pista_password: L("An app password: it starts with iurdav_.", "Una contraseña de aplicación: empieza por iurdav_."),
    ruta_credenciales: "dashboard/profile",
  },
  {
    id: "generico", nombre: L("Other WebDAV server", "Otro servidor WebDAV"),
    descripcion: L("Nextcloud, ownCloud, Synology, Seafile or any WebDAV over HTTPS.", "Nextcloud, ownCloud, Synology, Seafile o cualquier WebDAV sobre HTTPS."),
    sufijo_url: null, ruta_selftest: ".iuredav-selftest.txt", carpeta_muestra: null,
    nombre_volumen: "WebDAV", donde_gestionar: null,
    pista_password: L(
      "Your password, or better an app password if your server offers them. It's stored in your system keychain.",
      "Tu contraseña, o mejor una contraseña de aplicación si tu servidor las ofrece. Se guarda en el llavero de tu sistema.",
    ),
    ruta_credenciales: null,
  },
];

const apps = [
  { id: "transcribe", name: "IureTranscribe", description: L("Transcribes audio and video locally, records meetings and generates summaries and minutes.", "Transcribe audio y video localmente, graba reuniones y genera resúmenes y minutas."), installed: true, path: "/usr/bin/iuretranscribe", downloadUrl: "https://github.com/ellaguno/iuretranscribe/releases/latest", latestVersion: "0.7.0" },
  { id: "editor", name: "iureditor", description: L("Markdown editor with diagrams, formulas and export to PDF and DOCX.", "Editor Markdown con diagramas, fórmulas y exportación a PDF y DOCX."), installed: true, path: "/usr/bin/iureditor", downloadUrl: "https://github.com/ellaguno/iureditor/releases/latest", latestVersion: "1.9.2" },
  { id: "dav", name: "IureDav", description: L("Mounts your Iurefficient documents as a drive on your computer.", "Monta los documentos de Iurefficient como una unidad de tu equipo."), installed: true, path: null, downloadUrl: "https://github.com/ellaguno/iuredav/releases/latest", latestVersion: "0.8.1" },
  { id: "ocr", name: "IureOCR", description: L("Recognizes text in scans and photos on your computer and produces searchable PDFs ready for Iurefficient.", "Reconoce el texto de escaneos y fotos en tu equipo y deja PDF buscables listos para Iurefficient."), installed: false, path: null, downloadUrl: "https://github.com/ellaguno/iureocr/releases/latest", latestVersion: "0.5.1" },
];

// Llamadas que la captura resuelve a mano, para fotografiar el estado intermedio.
const pendientes: Record<string, () => void> = {};
const esperar = (clave: string, valor: () => any) =>
  new Promise((resolve) => { pendientes[clave] = () => resolve(valor()); });

const buscar = (id: string) => conexiones.find((c) => c.id === id);

mockIPC(
  (cmd, args: any) => {
    switch (cmd) {
      case "ui_language": return lang;
      case "idioma_preferido": return "auto";
      case "fijar_idioma": return lang;
      case "listar_conexiones": return structuredClone(conexiones);
      case "comprobar_sistema": return null;
      case "nombre_destino": return L("Folder", "Carpeta");
      case "listar_presets": return presets;
      case "autoarranque": return true;
      case "arranque_oculto": return true;
      case "avisar_actualizaciones": return true;
      case "acerca_de": return { version: "0.8.1", url_releases: "https://github.com/ellaguno/iuredav/releases" };
      case "actualizacion_disponible": return null;
      case "enlaces_iniciales": return [];
      case "apps_estado": return apps;
      case "punto_sugerido": return `/home/demo/${args.id === "conexion" ? "WebDAV" : args.id.replace(/(^|-)(\w)/g, (_: string, s: string, c: string) => (s ? " " : "") + c.toUpperCase())}`;
      case "cuenta_activa": return { dominio: "demo.iurefficient.com", correo: "ana.torres@example.com" };
      case "iniciar_sesion":
        return { requiereTotp: false, totpToken: null, passwordApp: "iurdav_demo", nombre: "Ana Torres", reutilizada: false };
      case "probar":
        return esperar("probar", () => (args.preset === "iurefficient" ? capsIure(false) : capsNextcloud()));
      case "resondear":
        return esperar("resondear", () => {
          const c = buscar(args.id);
          c.capacidades = capsIure(args.escritura);
          return c.capacidades;
        });
      case "guardar_conexion": return null;
      case "montar":
        return esperar("montar", () => { buscar(args.id).montado = true; return buscar(args.id).punto_montaje; });
      case "desmontar": buscar(args.id).montado = false; return null;
      case "cambiar_modo": buscar(args.id).escritura = args.escritura; return null;
      case "refrescar": case "abrir_carpeta": case "anclar": case "desanclar": case "olvidar_conexion":
      case "fijar_autoarranque": case "fijar_arranque_oculto": case "fijar_avisar_actualizaciones": case "lanzar_app":
        return null;
      default:
        if (cmd.startsWith("plugin:")) return null;
        console.warn("mock: sin respuesta para", cmd);
        return null;
    }
  },
  { shouldMockEvents: true },
);

const w = window as any;
w.__emit = emit;
w.__resolver = (clave: string) => { pendientes[clave]?.(); delete pendientes[clave]; };
w.__conexiones = conexiones;
// Los diálogos de confirmación nativos no aparecen en la captura: se aceptan solos.
window.confirm = () => true;
