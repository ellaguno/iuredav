import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AppId, EstadoApp, api } from "../api";
import { t, useIdioma } from "../i18n";

const REPOS: Record<AppId, string> = {
  transcribe: "https://github.com/ellaguno/iuretranscribe",
  editor: "https://github.com/ellaguno/iureditor",
  dav: "https://github.com/ellaguno/iuredav",
  ocr: "https://github.com/ellaguno/iureocr",
};

/** «Apps de Iurefficient»: las apps de escritorio, cuáles están en este equipo y dónde bajarlas. */
export default function AppsIurefficient({ version }: { version?: string }) {
  const [apps, setApps] = useState<EstadoApp[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Las descripciones las redacta el conector en el idioma actual: se piden de nuevo al cambiarlo.
  const idioma = useIdioma();

  useEffect(() => {
    api.apps(false).then(setApps).catch(() => {});
    api.apps(true).then(setApps).catch(() => {});
  }, [idioma]);

  if (!apps) return null;
  return (
    <div className="tarjeta apps">
      <h3>{t("apps.titulo")}</h3>
      <p className="detalle">{t("apps.texto")}</p>
      {apps.map((a) => (
        <div className="fila" key={a.id}>
          <div className="crece">
            <strong>{a.name}</strong>{" "}
            <span className="detalle">
              {a.id === "dav"
                ? `${t("apps.estaApp")}${version ? ` ${version}` : ""}`
                : a.installed
                  ? t("apps.instalada")
                  : t("apps.noInstalada")}
              {a.latestVersion ? t("apps.ultima", { version: a.latestVersion }) : ""}
            </span>
            <div className="detalle">{a.description}</div>
          </div>
          {a.id !== "dav" &&
            (a.installed ? (
              <button className="btn" title={a.path ?? ""} onClick={() => api.lanzarApp(a.id).catch((e) => setError(String(e)))}>
                {t("apps.abrir")}
              </button>
            ) : (
              <button className="btn principal" onClick={() => void openUrl(a.downloadUrl)}>
                {t("apps.descargar")}
              </button>
            ))}
          <button className="btn plano" onClick={() => void openUrl(REPOS[a.id])}>
            GitHub
          </button>
        </div>
      ))}
      {error && <div className="error-caja">{error}</div>}
    </div>
  );
}
