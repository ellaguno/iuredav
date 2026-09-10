import { useEffect, useState } from "react";
import { Capacidades, api } from "../api";
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

  const completo = url.trim() && usuario.trim() && password;

  async function probar() {
    setProbando(true);
    setError(null);
    setCaps(null);
    try {
      // Solo la fase de lectura: no deja rastro en el servidor.
      setCaps(await api.probar(url.trim(), usuario.trim(), password, false));
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
        password, puntoMontaje: punto, capacidades: caps,
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
            <label htmlFor="nombre">Nombre</label>
            <input id="nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} />
            <div className="pista">Como quieres verla en la lista y en tu carpeta.</div>
          </div>

          <div className="campo">
            <label htmlFor="url">Dirección del servidor</label>
            <input
              id="url" value={url} onChange={(e) => setUrl(e.target.value)}
              placeholder="https://tu-instancia.iurefficient.com/webdav/"
              spellCheck={false} autoCapitalize="off"
            />
          </div>

          <div className="campo">
            <label htmlFor="usuario">Tu correo</label>
            <input
              id="usuario" type="email" value={usuario} onChange={(e) => setUsuario(e.target.value)}
              placeholder="nombre@despacho.com" spellCheck={false} autoCapitalize="off"
            />
          </div>

          <div className="campo">
            <label htmlFor="pass">Contraseña de aplicación</label>
            <input
              id="pass" type="password" value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="iurdav_…" spellCheck={false}
            />
            <div className="pista">
              No es la contraseña con la que entras a Iurefficient. Es una contraseña
              de aplicación, empieza por <code>iurdav_</code> y la generas —y puedes
              revocarla— desde tu perfil. Se guarda en el llavero de tu sistema.
            </div>
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
