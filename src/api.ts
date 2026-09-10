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

export interface Conexion {
  id: string;
  nombre: string;
  url: string;
  usuario: string;
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

export interface Aviso {
  titulo: string;
  detalle: string;
  severidad: "limite" | "aviso" | "error";
  ruta: string | null;
}

export const api = {
  listar: () => invoke<Conexion[]>("listar_conexiones"),

  probar: (url: string, usuario: string, password: string, escritura: boolean) =>
    invoke<Capacidades>("probar", { url, usuario, password, escritura }),

  guardar: (c: {
    id: string;
    nombre: string;
    url: string;
    usuario: string;
    password: string;
    puntoMontaje: string;
    capacidades: Capacidades | null;
  }) =>
    invoke<void>("guardar_conexion", {
      id: c.id,
      nombre: c.nombre,
      url: c.url,
      usuario: c.usuario,
      password: c.password,
      puntoMontaje: c.puntoMontaje,
      capacidades: c.capacidades,
    }),

  olvidar: (id: string) => invoke<void>("olvidar_conexion", { id }),
  montar: (id: string, escritura: boolean) => invoke<string>("montar", { id, escritura }),
  desmontar: (id: string) => invoke<void>("desmontar", { id }),
  puntoSugerido: (id: string) => invoke<string>("punto_sugerido", { id }),

  /** `null` si esta maquina puede montar; si no, que le falta. */
  comprobarSistema: () => invoke<Requisito | null>("comprobar_sistema"),
  nombreDestino: () => invoke<string>("nombre_destino"),
  cambiarModo: (id: string, escritura: boolean) =>
    invoke<void>("cambiar_modo", { id, escritura }),
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
