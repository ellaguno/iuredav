import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AppId, EstadoApp, api } from "../api";

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

  useEffect(() => {
    api.apps(false).then(setApps).catch(() => {});
    api.apps(true).then(setApps).catch(() => {});
  }, []);

  if (!apps) return null;
  return (
    <div className="tarjeta apps">
      <h3>Apps de Iurefficient</h3>
      <p className="detalle">
        IureDav monta tus documentos como una unidad; IureTranscribe transcribe reuniones y
        audios; IureEditor edita los documentos y sube versiones; IureOCR reconoce el texto de documentos
        escaneados e imágenes con Tesseract y trae herramientas PDF. Comparten la cuenta y el llavero.
      </p>
      {apps.map((a) => (
        <div className="fila" key={a.id}>
          <div className="crece">
            <strong>{a.name}</strong>{" "}
            <span className="detalle">
              {a.id === "dav" ? `esta app${version ? ` ${version}` : ""}` : a.installed ? "instalada" : "no instalada"}
              {a.latestVersion ? ` · última ${a.latestVersion}` : ""}
            </span>
            <div className="detalle">{a.description}</div>
          </div>
          {a.id !== "dav" &&
            (a.installed ? (
              <button className="btn" title={a.path ?? ""} onClick={() => api.lanzarApp(a.id).catch((e) => setError(String(e)))}>
                Abrir
              </button>
            ) : (
              <button className="btn principal" onClick={() => void openUrl(a.downloadUrl)}>
                Descargar
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
