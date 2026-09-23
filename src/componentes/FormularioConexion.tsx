import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
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

/**
 * Dónde genera este usuario su contraseña de aplicación.
 *
 * Sale del dominio que acaba de escribir, no de una instancia escrita a mano:
 * cada despacho tiene la suya. `null` si todavía no hay dirección, si no se
 * entiende, o si el tipo de servidor no tiene una página conocida.
 */
function urlCredenciales(preset: Preset | undefined, escrito: string): string | null {
  const ruta = preset?.ruta_credenciales;
  const dominio = escrito.trim();
  if (!ruta || !dominio) return null;
  try {
    // Se queda solo con el origen: lo escrito puede llevar ya /webdav/.
    const base = new URL(dominio.includes("://") ? dominio : `https://${dominio}`);
    return new URL(ruta, `${base.origin}/`).toString();
  } catch {
    return null;
  }
}

export default function FormularioConexion({ onGuardado, onCancelar }: Props) {
  const [presets, setPresets] = useState<Preset[]>([]);
  const [presetId, setPresetId] = useState("iurefficient");
  const [nombre, setNombre] = useState("Iurefficient");
  const [url, setUrl] = useState("");
  const [usuario, setUsuario] = useState("");
  const [password, setPassword] = useState("");
  const [punto, setPunto] = useState("");

  // Modo de credencial para Iurefficient: con la cuenta (recomendado) o con una
  // contraseña de aplicación escrita a mano.
  const [modo, setModo] = useState<"cuenta" | "manual">("cuenta");
  const [passCuenta, setPassCuenta] = useState("");
  const [totpToken, setTotpToken] = useState<string | null>(null);
  const [totpCode, setTotpCode] = useState("");
  const [conectando, setConectando] = useState(false);
  const [conectado, setConectado] = useState<string | null>(null);

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
    // Si otra app de Iurefficient (IureTranscribe, IureEditor, IureOCR) ya inició
    // sesión en este equipo, se parte de esa instancia y correo: solo falta la contraseña.
    api.cuentaActiva().then((c) => {
      if (!c) return;
      setUrl((u) => u || c.dominio);
      setUsuario((v) => v || c.correo);
    }).catch(() => {});
  }, []);

  const preset = presets.find((p) => p.id === presetId);
  const credenciales = urlCredenciales(preset, url);

  const completo = url.trim() && usuario.trim() && password;
  const conCuenta = presetId === "iurefficient" && modo === "cuenta";

  async function conectarCuenta() {
    setConectando(true);
    setError(null);
    try {
      const r = await api.iniciarSesion(
        url.trim(), usuario.trim(), passCuenta,
        totpToken ?? undefined, totpToken ? totpCode.trim() : undefined,
      );
      if (r.requiereTotp) {
        setTotpToken(r.totpToken);
        return;
      }
      if (r.passwordApp) {
        setPassword(r.passwordApp);
        setPassCuenta("");
        setTotpToken(null);
        setTotpCode("");
        setConectado(
          (r.nombre ? `Sesión iniciada como ${r.nombre}. ` : "Sesión iniciada. ") +
            (r.reutilizada
              ? "Se reutilizó la contraseña de aplicación guardada en el llavero."
              : "Se creó una contraseña de aplicación a nombre de este equipo."),
        );
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setConectando(false);
    }
  }

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

          {presetId === "iurefficient" && (
            <div className="campo">
              <label>Acceso</label>
              <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
                <label style={{ display: "flex", gap: 6, alignItems: "center", cursor: "pointer" }}>
                  <input type="radio" checked={modo === "cuenta"} onChange={() => setModo("cuenta")} />
                  Con mi cuenta de Iurefficient (recomendado)
                </label>
                <label style={{ display: "flex", gap: 6, alignItems: "center", cursor: "pointer" }}>
                  <input type="radio" checked={modo === "manual"} onChange={() => setModo("manual")} />
                  Ya tengo una contraseña de aplicación
                </label>
              </div>
            </div>
          )}

          {conCuenta ? (
            <div className="campo">
              <label htmlFor="passCuenta">{totpToken ? "Código de verificación" : "Contraseña de Iurefficient"}</label>
              {totpToken ? (
                <input
                  id="passCuenta" value={totpCode} inputMode="numeric" autoComplete="one-time-code"
                  onChange={(e) => setTotpCode(e.target.value)} placeholder="6 dígitos"
                  onKeyDown={(e) => e.key === "Enter" && void conectarCuenta()}
                />
              ) : (
                <input
                  id="passCuenta" type="password" value={passCuenta} autoComplete="current-password"
                  onChange={(e) => setPassCuenta(e.target.value)} placeholder="La misma con la que entras a la web"
                  onKeyDown={(e) => e.key === "Enter" && void conectarCuenta()}
                />
              )}
              <div className="pista">
                Tu contraseña no se guarda: IureDav inicia sesión, pide a la instancia una contraseña de
                aplicación a nombre de este equipo y la guarda en el llavero del sistema, compartido con
                IureTranscribe, IureEditor e IureOCR.
              </div>
              <button
                className="btn"
                style={{ marginTop: 8 }}
                disabled={conectando || !url.trim() || !usuario.trim() || (totpToken ? totpCode.trim().length < 6 : !passCuenta)}
                onClick={() => void conectarCuenta()}
              >
                {conectando ? "Conectando…" : totpToken ? "Verificar" : "Conectar y obtener acceso"}
              </button>
              {conectado && <div className="pista" style={{ marginTop: 6 }}>✓ {conectado}</div>}
            </div>
          ) : (
            <div className="campo">
              <label htmlFor="pass">Contraseña</label>
              <input
                id="pass" type="password" value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder={presetId === "iurefficient" ? "iurdav_…" : "••••••••"}
                spellCheck={false}
              />
              <div className="pista">{preset?.pista_password ?? ""}</div>
              {preset?.ruta_credenciales && (
                <button
                  className="btn plano"
                  style={{ marginTop: 6, paddingLeft: 0 }}
                  disabled={!credenciales}
                  title={
                    credenciales ??
                    "Escribe antes la dirección de tu instancia y el enlace apuntará a la tuya."
                  }
                  onClick={() => credenciales && void openUrl(credenciales)}
                >
                  Abrir mi perfil para generarla →
                </button>
              )}
            </div>
          )}

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
          <PanelCapacidades caps={caps} gestor={preset?.donde_gestionar ?? null} />
        </>
      )}
    </>
  );
}
