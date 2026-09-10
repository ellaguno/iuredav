import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { Aviso, Conexion, Requisito, api } from "./api";
import FormularioConexion from "./componentes/FormularioConexion";
import Marca from "./componentes/Marca";
import PanelCapacidades from "./componentes/PanelCapacidades";

/** Un verbo del protocolo no le dice nada a nadie: se nombra la accion. */
const ACCION: Record<string, string> = {
  DELETE: "eliminar documentos",
  MKCOL: "crear carpetas",
  MOVE: "mover o renombrar",
  COPY: "copiar dentro de la unidad",
  // PROPPATCH se omite a proposito: no es algo que nadie intente hacer, asi que en
  // el titular seria ruido. Sigue apareciendo en la tabla del detalle.
};

/** "eliminar documentos, crear carpetas ni mover" en vez de "delete, mkcol, move". */
function enumerar(verbos: string[]): string {
  const partes = verbos.map((v) => ACCION[v.toUpperCase()]).filter(Boolean);
  if (partes.length === 0) return "";
  if (partes.length === 1) return partes[0];
  return `${partes.slice(0, -1).join(", ")} ni ${partes[partes.length - 1]}`;
}

type Vista = { pantalla: "lista" } | { pantalla: "nueva" } | { pantalla: "detalle"; id: string };

export default function App() {
  const [vista, setVista] = useState<Vista>({ pantalla: "lista" });
  const [conexiones, setConexiones] = useState<Conexion[] | null>(null);
  const [avisos, setAvisos] = useState<Aviso[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [ocupado, setOcupado] = useState<string | null>(null);
  const [falta, setFalta] = useState<Requisito | null>(null);
  const [destino, setDestino] = useState("Carpeta");

  const recargar = useCallback(async () => {
    try {
      setConexiones(await api.listar());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => { void recargar(); }, [recargar]);

  // Si a la maquina le falta la pieza que permite montar, se dice al entrar y no
  // cuando el usuario pulsa Montar y recibe un error del sistema.
  useEffect(() => {
    api.comprobarSistema().then(setFalta).catch(() => {});
    api.nombreDestino().then(setDestino).catch(() => {});
  }, []);

  // Los limites del servidor llegan traducidos desde Rust y se muestran tal cual.
  useEffect(() => {
    const p = listen<Aviso>("iuredav://aviso", (e) => {
      setAvisos((prev) => [e.payload, ...prev].slice(0, 4));
    });
    return () => { void p.then((quitar) => quitar()); };
  }, []);

  async function alternar(c: Conexion) {
    setOcupado(c.id);
    setError(null);
    try {
      if (c.montado) await api.desmontar(c.id);
      else await api.montar(c.id, c.escritura);
      await recargar();
    } catch (e) {
      setError(String(e));
    } finally {
      setOcupado(null);
    }
  }

  async function cambiarModo(c: Conexion) {
    if (!c.escritura) {
      const puedeEscribir = c.capacidades?.real.put_crear.estado === "funciona";
      const aviso = puedeEscribir
        ? "En este servidor, cada vez que guardes un documento se creará una versión nueva. " +
          "Y desde la carpeta seguirás sin poder eliminar ni renombrar.\n\n¿Activar el modo edición?"
        : "Todavía no se ha comprobado que este servidor acepte subidas. Puedes activarlo, " +
          "pero si no las acepta la carpeta seguirá siendo de solo lectura.\n\n¿Activar de todas formas?";
      if (!window.confirm(aviso)) return;
    }
    setOcupado(c.id);
    try {
      await api.cambiarModo(c.id, !c.escritura);
      await recargar();
    } catch (e) {
      setError(String(e));
    } finally {
      setOcupado(null);
    }
  }

  async function olvidar(c: Conexion) {
    if (c.montado) {
      setError("Desmonta la conexión antes de eliminarla.");
      return;
    }
    setOcupado(c.id);
    try {
      await api.olvidar(c.id);
      await recargar();
    } catch (e) {
      setError(String(e));
    } finally {
      setOcupado(null);
    }
  }

  const detalle =
    vista.pantalla === "detalle" ? conexiones?.find((c) => c.id === vista.id) : undefined;

  return (
    <div className="marco">
      <header className="cabecera">
        <Marca />
        <div className="crece">
          <h1>IureDav</h1>
          <div style={{ fontSize: 12, color: "var(--tenue)" }}>
            Tus documentos, como una carpeta más de tu equipo
          </div>
        </div>
        {vista.pantalla === "lista" && (
          <button className="btn principal" onClick={() => setVista({ pantalla: "nueva" })}>
            Añadir conexión
          </button>
        )}
        {vista.pantalla !== "lista" && (
          <button className="btn" onClick={() => setVista({ pantalla: "lista" })}>
            Volver
          </button>
        )}
      </header>

      {falta && (
        <div className="tarjeta">
          <h2>Falta {falta.que_falta}</h2>
          <p style={{ color: "var(--tenue)", marginTop: 4 }}>{falta.por_que}</p>
          <div className="nota aviso">
            <strong>Qué hacer</strong>
            <p>{falta.como_instalar}</p>
          </div>
          {falta.url && (
            <div className="acciones">
              <span className="crece" />
              <button className="btn principal" onClick={() => void openUrl(falta.url!)}>
                Descargar {falta.que_falta}
              </button>
            </div>
          )}
        </div>
      )}

      {error && <div className="error-caja">{error}</div>}

      {avisos.map((a, i) => (
        <div className={`nota ${a.severidad}`} key={i}>
          <strong>{a.titulo}</strong>
          <p>
            {a.detalle}
            {a.ruta && <> — <span className="ruta">{a.ruta}</span></>}
          </p>
        </div>
      ))}

      {vista.pantalla === "nueva" && (
        <FormularioConexion
          onCancelar={() => setVista({ pantalla: "lista" })}
          onGuardado={() => { setVista({ pantalla: "lista" }); void recargar(); }}
        />
      )}

      {vista.pantalla === "detalle" && detalle && (
        <>
          <div className="tarjeta">
            <h2>{detalle.nombre}</h2>
            <div className="ruta">{detalle.url}</div>
          </div>
          {detalle.capacidades ? (
            <PanelCapacidades caps={detalle.capacidades} />
          ) : (
            <div className="tarjeta">
              <p style={{ color: "var(--tenue)", margin: 0 }}>
                Esta conexión todavía no se ha comprobado. Se hará sola la primera vez
                que la montes.
              </p>
            </div>
          )}
        </>
      )}

      {vista.pantalla === "lista" && (
        <>
          {conexiones === null && <div className="cargando">Cargando…</div>}

          {conexiones?.length === 0 && (
            <div className="vacio">
              <h2>Todavía no hay ninguna conexión</h2>
              <p>
                Añade tu instancia de Iurefficient y aparecerá como una carpeta de tu
                equipo, con tus casos dentro.
              </p>
              <button
                className="btn principal grande"
                style={{ marginTop: 14 }}
                onClick={() => setVista({ pantalla: "nueva" })}
              >
                Añadir mi primera conexión
              </button>
            </div>
          )}

          {conexiones?.map((c) => (
            <div className="tarjeta" key={c.id}>
              <div className="fila">
                <div className="crece">
                  <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                    <h2>{c.nombre}</h2>
                    <span className={`pastilla ${c.montado ? "viva" : ""}`}>
                      <span className="punto" />
                      {c.montado ? "Montado" : "Desmontado"}
                    </span>
                    <span className="pastilla">{c.escritura ? "Edición" : "Solo lectura"}</span>
                  </div>
                  <div className="ruta">
                    {destino}: {c.punto_montaje}
                  </div>
                </div>

                {c.montado && (
                  <button className="btn" onClick={() => void openPath(c.punto_montaje)}>
                    Abrir carpeta
                  </button>
                )}
                <button
                  className={`btn ${c.montado ? "" : "principal"}`}
                  onClick={() => void alternar(c)}
                  disabled={ocupado === c.id}
                >
                  {ocupado === c.id ? "…" : c.montado ? "Desmontar" : "Montar"}
                </button>
              </div>

              {c.incumple.length > 0 && (
                <div className="nota limite">
                  <strong>Este servidor tiene límites</strong>
                  <p>
                    No permite {enumerar(c.incumple)}. Esas operaciones se hacen desde
                    Iurefficient; desde la carpeta de tu equipo no funcionan.
                  </p>
                </div>
              )}

              <div className="acciones" style={{ marginTop: 12 }}>
                <button className="btn plano" onClick={() => setVista({ pantalla: "detalle", id: c.id })}>
                  Ver qué sabe hacer este servidor
                </button>
                <button className="btn plano" onClick={() => void cambiarModo(c)} disabled={c.montado}>
                  {c.escritura ? "Pasar a solo lectura" : "Permitir edición"}
                </button>
                <span className="crece" />
                <button className="btn plano peligro" onClick={() => void olvidar(c)}>
                  Eliminar
                </button>
              </div>
            </div>
          ))}
        </>
      )}
    </div>
  );
}
