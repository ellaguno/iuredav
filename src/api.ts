/** Puente tipado con el nucleo en Rust. Los tipos reflejan `iuredav-core`. */
import { invoke } from "@tauri-apps/api/core";

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
  /** Verbos que el servidor anuncia y no cumple. */
  incumple: string[];
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

export interface Aviso {
  titulo: string;
  detalle: string;
  severidad: "limite" | "aviso" | "error";
  ruta: string | null;
}

export const api = {
  listar: () => invoke<Conexion[]>("listar_conexiones"),

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
  desmontar: (id: string) => invoke<void>("desmontar", { id }),
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
};

/** Un verbo solo cuenta como disponible si se comprobo que funciona. */
export const funciona = (v: Verdict) => v.estado === "funciona";

export function describir(v: Verdict): string {
  switch (v.estado) {
    case "funciona": return "funciona";
    case "rechazado": return `rechazado (${v.status})`;
    case "roto": return `roto (${v.status})`;
    case "sin_probar": return "sin probar";
    case "error": return `error: ${v.detalle}`;
  }
}
