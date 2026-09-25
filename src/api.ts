/** Puente tipado con el nucleo en Rust. Los tipos reflejan `iuredav-core`. */
import { invoke } from "@tauri-apps/api/core";
import { t } from "./i18n";

export type Verdict =
  | { estado: "funciona" }
  | { estado: "rechazado"; status: number }
  | { estado: "roto"; status: number }
  | { estado: "sin_probar" }
  | { estado: "error"; detalle: string };

export type Sobrescritura = "sobrescribe" | "crea_version" | "sin_efecto" | "desconocido";
export type SoporteRango = "soportado" | "ignorado" | "desconocido";

export interface Real {
  propfind_depth0: Verdict;
  propfind_depth1: Verdict;
  get: Verdict;
  rangos: SoporteRango;
  etag: boolean;
  last_modified: boolean;
  put_crear: Verdict;
  put_sobrescribir: Sobrescritura;
  mkcol: Verdict;
  mover: Verdict;
  borrar: Verdict;
  proppatch_modtime: Verdict;
  locks: { lock: Verdict; cruza_procesos: boolean | null };
  raiz: string[];
}

export interface Capacidades {
  probed_at: string;
  url: string;
  anunciado: { allow: string[]; dav: string[]; server: string | null };
  real: Real;
  sonda_escritura: boolean;
}

export interface Preset {
  id: string;
  nombre: string;
  descripcion: string;
  sufijo_url: string | null;
  ruta_selftest: string;
  carpeta_muestra: string | null;
  nombre_volumen: string;
  /** Nombre de la aplicación web del servidor, si se conoce. */
  donde_gestionar: string | null;
  pista_password: string;
  /** Página donde generar la contraseña, relativa al dominio de la instancia. */
  ruta_credenciales: string | null;
}

export interface Conexion {
  id: string;
  nombre: string;
  url: string;
  usuario: string;
  preset: string;
  punto_montaje: string;
  escritura: boolean;
  anclados: string[];
  capacidades: Capacidades | null;
  montado: boolean;
  /**
   * Lo que esta unidad no puede hacer, lo anuncie el servidor o no. No son las
   * discrepancias: un servidor honesto sobre sus límites sigue teniéndolos.
   */
  limites: string[];
}

export interface Requisito {
  que_falta: string;
  por_que: string;
  como_instalar: string;
  url: string | null;
}

/** Progreso de la descarga de una carpeta marcada para uso sin conexión. */
export interface AvanceAnclaje {
  conexion: string;
  carpeta: string;
  archivos: number;
  bytes: number;
  fallidos: number;
  terminado: boolean;
}

export type AppId = "transcribe" | "editor" | "dav" | "ocr";
/** Una app de escritorio de Iurefficient: si está instalada aquí y su última versión. */
export interface EstadoApp {
  id: AppId;
  name: string;
  description: string;
  installed: boolean;
  path: string | null;
  downloadUrl: string;
  latestVersion: string | null;
}

/** La versión que corre y la página con todas las releases. */
export interface AcercaDe {
  version: string;
  url_releases: string;
}

/** Una versión más nueva que la instalada, y dónde descargarla. */
export interface Actualizacion {
  version: string;
  url: string;
}

export interface Aviso {
  titulo: string;
  detalle: string;
  severidad: "limite" | "aviso" | "error";
  ruta: string | null;
}

export interface ResultadoLogin {
  requiereTotp: boolean;
  totpToken: string | null;
  passwordApp: string | null;
  nombre: string | null;
  reutilizada: boolean;
}

export const api = {
  listar: () => invoke<Conexion[]>("listar_conexiones"),

  /**
   * Inicia sesión con la cuenta de Iurefficient y obtiene una contraseña de
   * aplicación WebDAV (creada a nombre de este equipo, o reutilizada del llavero
   * compartido). La contraseña de la cuenta no se guarda.
   */
  iniciarSesion: (dominio: string, usuario: string, password: string, totpToken?: string, totpCode?: string) =>
    invoke<ResultadoLogin>("iniciar_sesion", { dominio, usuario, password, totpToken: totpToken ?? null, totpCode: totpCode ?? null }),

  /** Dominio y correo con los que otra app de Iurefficient ya inició sesión en este equipo. */
  cuentaActiva: () => invoke<{ dominio: string; correo: string } | null>("cuenta_activa"),

  probar: (url: string, usuario: string, password: string, escritura: boolean, preset: string) =>
    invoke<Capacidades>("probar", { url, usuario, password, escritura, preset }),

  /**
   * Vuelve a medir una conexión ya guardada y actualiza su informe.
   * Con `escritura` se prueban también PUT, MKCOL, MOVE y DELETE: es lo único
   * que puede confirmar que el servidor acepta subidas, y por tanto lo único
   * que desbloquea el modo edición. Deja un archivo de diagnóstico.
   */
  resondear: (id: string, escritura: boolean) =>
    invoke<Capacidades>("resondear", { id, escritura }),

  listarPresets: () => invoke<Preset[]>("listar_presets"),

  guardar: (datos: {
    id: string;
    nombre: string;
    url: string;
    usuario: string;
    password: string;
    puntoMontaje: string;
    capacidades: Capacidades | null;
    preset: string;
  }) => invoke<void>("guardar_conexion", { datos }),

  olvidar: (id: string) => invoke<void>("olvidar_conexion", { id }),
  montar: (id: string, escritura: boolean) => invoke<string>("montar", { id, escritura }),
  /** `recordar`: sin olvidar que estaba montada, para que vuelva a montarse al arrancar. */
  desmontar: (id: string, recordar = false) => invoke<void>("desmontar", { id, recordar }),
  /** Abre el punto de montaje en el gestor de archivos. Solo si está montada. */
  abrirCarpeta: (id: string) => invoke<void>("abrir_carpeta", { id }),
  puntoSugerido: (id: string) => invoke<string>("punto_sugerido", { id }),

  /** `null` si esta maquina puede montar; si no, que le falta. */
  comprobarSistema: () => invoke<Requisito | null>("comprobar_sistema"),
  nombreDestino: () => invoke<string>("nombre_destino"),
  cambiarModo: (id: string, escritura: boolean) =>
    invoke<void>("cambiar_modo", { id, escritura }),

  anclar: (id: string, carpeta: string) => invoke<void>("anclar", { id, carpeta }),
  desanclar: (id: string, ruta: string) => invoke<void>("desanclar", { id, ruta }),

  /** Vuelve a leer el listado del servidor, sin esperar a que caduque la caché. */
  refrescar: (id: string) => invoke<void>("refrescar", { id, ruta: "" }),

  autoarranque: () => invoke<boolean>("autoarranque"),
  fijarAutoarranque: (activo: boolean) => invoke<void>("fijar_autoarranque", { activo }),

  /** Al arrancar con la sesión, quedarse en la bandeja sin abrir la ventana. */
  arranqueOculto: () => invoke<boolean>("arranque_oculto"),
  fijarArranqueOculto: (activo: boolean) => invoke<void>("fijar_arranque_oculto", { activo }),

  acercaDe: () => invoke<AcercaDe>("acerca_de"),

  /** Apps hermanas (IureTranscribe, IureEditor, IureOCR): con red consulta también la última versión. */
  apps: (conRed: boolean) => invoke<EstadoApp[]>("apps_estado", { conRed }),
  lanzarApp: (app: AppId) => invoke<void>("lanzar_app", { app }),
  /** Enlaces `iuredav://` con los que se abrió la app. */
  enlacesIniciales: () => invoke<string[]>("enlaces_iniciales"),

  /** La versión nueva encontrada, si hay; mientras la ventana está abierta llega también por evento. */
  actualizacion: () => invoke<Actualizacion | null>("actualizacion_disponible"),
  avisarActualizaciones: () => invoke<boolean>("avisar_actualizaciones"),
  fijarAvisarActualizaciones: (activo: boolean) =>
    invoke<void>("fijar_avisar_actualizaciones", { activo }),

  /** Ajuste de idioma tal cual: "auto", "en" o "es". */
  idiomaPreferido: () => invoke<string>("idioma_preferido"),
  /** Guarda el idioma y devuelve el ya resuelto ("en" o "es"). */
  fijarIdioma: (idioma: string) => invoke<string>("fijar_idioma", { idioma }),
};

/** Un verbo solo cuenta como disponible si se comprobo que funciona. */
export const funciona = (v: Verdict) => v.estado === "funciona";

export function describir(v: Verdict): string {
  switch (v.estado) {
    case "funciona": return t("verdict.funciona");
    case "rechazado": return t("verdict.rechazado", { status: v.status });
    case "roto": return t("verdict.roto", { status: v.status });
    case "sin_probar": return t("verdict.sinProbar");
    case "error": return t("verdict.error", { detalle: v.detalle });
  }
}
