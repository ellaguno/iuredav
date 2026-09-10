import { useEffect, useState } from "react";
import { Capacidades, Preset, api } from "../api";
import PanelCapacidades from "./PanelCapacidades";

interface Props {
  onGuardado: () => void;
  onCancelar: () => void;
}

/** Convierte "Despacho Central" en "despacho-central", que sirve de identificador. */
const aId = (s: string) =>
  s.trim().toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "")
    .replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "conexion";

export default function FormularioConexion({ onGuardado, onCancelar }: Props) {
  const [presets, setPresets] = useState<Preset[]>([]);
  const [presetId, setPresetId] = useState("iurefficient");
  const [nombre, setNombre] = useState("Iurefficient");
  const [url, setUrl] = useState("");
  const [usuario, setUsuario] = useState("");
  const [password, setPassword] = useState("");
  const [punto, setPunto] = useState("");

  const [probando, setProbando] = useState(false);
  const [caps, setCaps] = useState<Capacidades | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [guardando, setGuardando] = useState(false);

  const id = aId(nombre);
  useEffect(() => {
    api.puntoSugerido(id).then(setPunto).catch(() => {});
  }, [id]);

  useEffect(() => {
    api.listarPresets().then(setPresets).catch(() => {});
  }, []);

  const preset = presets.find((p) => p.id === presetId);

  const completo = url.trim() && usuario.trim() && password;

  async function probar() {
    setProbando(true);
    setError(null);
    setCaps(null);
    try {
      // Solo la fase de lectura: no deja rastro en el servidor.
      setCaps(await api.probar(url.trim(), usuario.trim(), password, false, presetId));
    } catch (e) {
      setError(String(e));
    } finally {
      setProbando(false);
    }
  }

  async function guardar() {
    setGuardando(true);
    setError(null);
    try {
      await api.guardar({
        id, nombre: nombre.trim(), url: url.trim(), usuario: usuario.trim(),
        password, puntoMontaje: punto, capacidades: caps, preset: presetId,
      });
      onGuardado();
    } catch (e) {
      setError(String(e));
      setGuardando(false);
    }
  }

  return (
    <>
      <div className="tarjeta">
        <h2>Nueva conexión</h2>

        <div style={{ marginTop: 16 }}>
          <div className="campo">
            <label htmlFor="tipo">Tipo de servidor</label>
            <select
              id="tipo"
              value={presetId}
              onChange={(e) => {
                setPresetId(e.target.value);
                setCaps(null);
                const p = presets.find((x) => x.id === e.target.value);
                if (p) setNombre(p.nombre_volumen);
              }}
            >
              {presets.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.nombre}
                </option>
              ))}
            </select>
            {preset && <div className="pista">{preset.descripcion}</div>}
          </div>

          <div className="campo">
            <label htmlFor="nombre">Nombre</label>
            <input id="nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} />
            <div className="pista">Como quieres verla en la lista y en tu carpeta.</div>
          </div>

          <div className="campo">
            <label htmlFor="url">Dirección del servidor</label>
            <input
              id="url" value={url} onChange={(e) => setUrl(e.target.value)}
              placeholder={
                presetId === "iurefficient"
                  ? "tu-instancia.iurefficient.com"
                  : "https://nube.ejemplo.com/remote.php/dav/files/tu-usuario/"
              }
              spellCheck={false} autoCapitalize="off"
            />
            {presetId === "iurefficient" && (
              <div className="pista">
                Basta el dominio: se completa con <code>/webdav/</code>.
              </div>
            )}
          </div>

          <div className="campo">
            <label htmlFor="usuario">Tu correo</label>
            <input
              id="usuario" type="email" value={usuario} onChange={(e) => setUsuario(e.target.value)}
              placeholder="nombre@despacho.com" spellCheck={false} autoCapitalize="off"
            />
          </div>

          <div className="campo">
            <label htmlFor="pass">Contraseña</label>
            <input
              id="pass" type="password" value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={presetId === "iurefficient" ? "iurdav_…" : "••••••••"}
              spellCheck={false}
            />
            <div className="pista">{preset?.pista_password ?? ""}</div>
          </div>

          <div className="campo">
            <label htmlFor="punto">Carpeta donde aparecerá</label>
            <input id="punto" value={punto} onChange={(e) => setPunto(e.target.value)} spellCheck={false} />
          </div>
        </div>

        {error && <div className="error-caja">{error}</div>}

        <div className="acciones">
          <button className="btn" onClick={onCancelar}>Cancelar</button>
          <span className="crece" />
          <button className="btn" onClick={probar} disabled={!completo || probando}>
            {probando ? "Comprobando…" : "Probar conexión"}
          </button>
          <button className="btn principal" onClick={guardar} disabled={!completo || guardando}>
            {guardando ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </div>

      {caps && (
        <>
          <div className="nota limite" style={{ marginBottom: 14 }}>
            <strong>Conexión correcta</strong>
            <p>
              Se encontró: {caps.real.raiz.join(", ") || "(sin carpetas en la raíz)"}. Esto
              es lo que el servidor sabe hacer de verdad:
            </p>
          </div>
          <PanelCapacidades caps={caps} />
        </>
      )}
    </>
  );
}
