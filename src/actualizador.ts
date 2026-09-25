// Instalar una versión nueva dentro de la app (releases firmadas de GitHub, vía el
// actualizador de Tauri). En Linux sólo aplica al AppImage; con .deb/.rpm el aviso
// enlaza la descarga. Antes de instalar se desmontan las unidades para que rclone
// no se quede colgado del binario viejo; siguen marcadas para volver al arrancar.
import { api } from "./api";

export type ResultadoActualizacion = "instalada" | "no-disponible" | "cancelada";

export async function instalarActualizacion(
  confirmar: (version: string) => Promise<boolean>,
  avisar: (texto: string) => void,
): Promise<ResultadoActualizacion> {
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return "no-disponible";
  if (!(await confirmar(update.version))) return "cancelada";
  try {
    for (const c of await api.listar()) {
      if (c.montado) await api.desmontar(c.id, true);
    }
  } catch {
    /* si no se pudo desmontar, la instalación sigue: el sidecar se relanza */
  }
  avisar("Descargando e instalando…");
  await update.downloadAndInstall();
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
  return "instalada";
}
