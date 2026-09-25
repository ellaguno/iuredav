import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Capacidades, Preset, api } from "../api";
import PanelCapacidades from "./PanelCapacidades";
import { t, useIdioma } from "../i18n";

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
  const idioma = useIdioma();
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

  // Descripciones y pistas llegan redactadas desde Rust: se piden en cada idioma.
  useEffect(() => {
    api.listarPresets().then(setPresets).catch(() => {});
  }, [idioma]);

  useEffect(() => {
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
          (r.nombre ? t("form.sesionComo", { nombre: r.nombre }) : t("form.sesion")) +
            (r.reutilizada ? t("form.reutilizada") : t("form.creada")),
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
        <h2>{t("form.titulo")}</h2>

        <div style={{ marginTop: 16 }}>
          <div className="campo">
            <label htmlFor="tipo">{t("form.tipo")}</label>
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
            <label htmlFor="nombre">{t("form.nombre")}</label>
            <input id="nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} />
            <div className="pista">{t("form.nombre.pista")}</div>
          </div>

          <div className="campo">
            <label htmlFor="url">{t("form.url")}</label>
            <input
              id="url" value={url} onChange={(e) => setUrl(e.target.value)}
              placeholder={
                presetId === "iurefficient"
                  ? t("form.url.ejemploIurefficient")
                  : t("form.url.ejemploGenerico")
              }
              spellCheck={false} autoCapitalize="off"
            />
            {presetId === "iurefficient" && (
              <div className="pista">
                {t("form.url.pistaAntes")}<code>/webdav/</code>{t("form.url.pistaDespues")}
              </div>
            )}
          </div>

          <div className="campo">
            <label htmlFor="usuario">{t("form.correo")}</label>
            <input
              id="usuario" type="email" value={usuario} onChange={(e) => setUsuario(e.target.value)}
              placeholder={t("form.correo.ejemplo")} spellCheck={false} autoCapitalize="off"
            />
          </div>

          {presetId === "iurefficient" && (
            <div className="campo">
              <label>{t("form.acceso")}</label>
              <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
                <label style={{ display: "flex", gap: 6, alignItems: "center", cursor: "pointer" }}>
                  <input type="radio" checked={modo === "cuenta"} onChange={() => setModo("cuenta")} />
                  {t("form.acceso.cuenta")}
                </label>
                <label style={{ display: "flex", gap: 6, alignItems: "center", cursor: "pointer" }}>
                  <input type="radio" checked={modo === "manual"} onChange={() => setModo("manual")} />
                  {t("form.acceso.manual")}
                </label>
              </div>
            </div>
          )}

          {conCuenta ? (
            <div className="campo">
              <label htmlFor="passCuenta">{totpToken ? t("form.codigo") : t("form.passCuenta")}</label>
              {totpToken ? (
                <input
                  id="passCuenta" value={totpCode} inputMode="numeric" autoComplete="one-time-code"
                  onChange={(e) => setTotpCode(e.target.value)} placeholder={t("form.codigo.ejemplo")}
                  onKeyDown={(e) => e.key === "Enter" && void conectarCuenta()}
                />
              ) : (
                <input
                  id="passCuenta" type="password" value={passCuenta} autoComplete="current-password"
                  onChange={(e) => setPassCuenta(e.target.value)} placeholder={t("form.passCuenta.ejemplo")}
                  onKeyDown={(e) => e.key === "Enter" && void conectarCuenta()}
                />
              )}
              <div className="pista">
                {t("form.passCuenta.pista")}
              </div>
              <button
                className="btn"
                style={{ marginTop: 8 }}
                disabled={conectando || !url.trim() || !usuario.trim() || (totpToken ? totpCode.trim().length < 6 : !passCuenta)}
                onClick={() => void conectarCuenta()}
              >
                {conectando ? t("form.conectando") : totpToken ? t("form.verificar") : t("form.conectar")}
              </button>
              {conectado && <div className="pista" style={{ marginTop: 6 }}>✓ {conectado}</div>}
            </div>
          ) : (
            <div className="campo">
              <label htmlFor="pass">{t("form.pass")}</label>
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
                    credenciales ?? t("form.perfil.tituloSinUrl")
                  }
                  onClick={() => credenciales && void openUrl(credenciales)}
                >
                  {t("form.perfil.abrir")}
                </button>
              )}
            </div>
          )}

          <div className="campo">
            <label htmlFor="punto">{t("form.punto")}</label>
            <input id="punto" value={punto} onChange={(e) => setPunto(e.target.value)} spellCheck={false} />
          </div>
        </div>

        {error && <div className="error-caja">{error}</div>}

        <div className="acciones">
          <button className="btn" onClick={onCancelar}>{t("form.cancelar")}</button>
          <span className="crece" />
          <button className="btn" onClick={probar} disabled={!completo || probando}>
            {probando ? t("form.comprobando") : t("form.probar")}
          </button>
          <button className="btn principal" onClick={guardar} disabled={!completo || guardando}>
            {guardando ? t("form.guardando") : t("form.guardar")}
          </button>
        </div>
      </div>

      {caps && (
        <>
          <div className="nota limite" style={{ marginBottom: 14 }}>
            <strong>{t("form.correcta")}</strong>
            <p>{t("form.encontrado", { raiz: caps.real.raiz.join(", ") || t("form.sinCarpetas") })}</p>
          </div>
          <PanelCapacidades caps={caps} gestor={preset?.donde_gestionar ?? null} />
        </>
      )}
    </>
  );
}
