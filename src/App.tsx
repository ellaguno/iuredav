import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { open as elegirCarpeta } from "@tauri-apps/plugin-dialog";
import { AvanceAnclaje, Aviso, Capacidades, Conexion, Preset, Requisito, api, describir } from "./api";
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
  const [presets, setPresets] = useState<Preset[]>([]);
  const [auto, setAuto] = useState(false);
  const [anclando, setAnclando] = useState<Record<string, AvanceAnclaje>>({});

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
    api.listarPresets().then(setPresets).catch(() => {});
    api.autoarranque().then(setAuto).catch(() => {});
  }, []);

  // Los limites del servidor llegan traducidos desde Rust y se muestran tal cual.
  useEffect(() => {
    const p = listen<Aviso>("iuredav://aviso", (e) => {
      setAvisos((prev) => [e.payload, ...prev].slice(0, 4));
    });
    return () => { void p.then((quitar) => quitar()); };
  }, []);

  // Progreso de las carpetas que se están dejando disponibles sin conexión.
  useEffect(() => {
    const p = listen<AvanceAnclaje>("iuredav://anclaje", (e) => {
      const a = e.payload;
      setAnclando((prev) => ({ ...prev, [`${a.conexion}:${a.carpeta}`]: a }));
      if (a.terminado) void recargar();
    });
    return () => { void p.then((quitar) => quitar()); };
  }, [recargar]);

  async function anclar(c: Conexion) {
    const elegida = await elegirCarpeta({
      directory: true,
      defaultPath: c.punto_montaje,
      title: "Elige una carpeta para tenerla sin conexión",
    });
    if (typeof elegida !== "string") return;
    try {
      await api.anclar(c.id, elegida);
      await recargar();
    } catch (e) {
      setError(String(e));
    }
  }

  /** Lo que hay que decir antes de la fase de escritura: deja un archivo. */
  function avisoEscritura(c: Conexion): string {
    const ruta = presets.find((p) => p.id === c.preset)?.ruta_selftest ?? ".iuredav-selftest.txt";
    return (
      `Se creará un archivo de diagnóstico en el servidor (${ruta}). Si el servidor no permite ` +
      "eliminar —el caso de Iurefficient— ese archivo se queda ahí. Repetir la comprobación no " +
      "acumula archivos, solo versiones del mismo."
    );
  }

  /**
   * Vuelve a medir el servidor. Con `escritura` se prueban también las subidas,
   * que es lo único que puede desbloquear el modo edición: sin esa medición el
   * montaje se fuerza a solo lectura por mucho que el usuario lo active.
   */
  async function medir(c: Conexion, escritura: boolean): Promise<Capacidades | null> {
    setOcupado(c.id);
    setError(null);
    try {
      const caps = await api.resondear(c.id, escritura);
      await recargar();
      return caps;
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      setOcupado(null);
    }
  }

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
      let caps = c.capacidades;

      // Mientras no se compruebe que el servidor acepta subidas, el montaje se
      // fuerza a solo lectura: activar el modo edición aquí no cambiaría nada.
      // Así que se ofrece medirlo, que es el único camino que lo desbloquea.
      if (caps?.real.put_crear.estado !== "funciona") {
        const quiere = window.confirm(
          "Para poder editar hay que comprobar antes que este servidor acepta subidas.\n\n" +
            avisoEscritura(c) +
            "\n\n¿Comprobarlo ahora?",
        );
        if (!quiere) return;

        const medido = await medir(c, true);
        if (!medido) return;
        if (medido.real.put_crear.estado !== "funciona") {
          setError(
            `Este servidor no acepta subidas (${describir(medido.real.put_crear)}), así que la ` +
              "carpeta seguirá siendo de solo lectura.",
          );
          return;
        }
        caps = medido;
      }

      const versiona = caps.real.put_sobrescribir === "crea_version";
      const limites = enumerar(c.incumple);
      const aviso =
        (versiona
          ? "En este servidor, cada vez que guardes un documento se creará una versión nueva. "
          : "") +
        (limites ? `Y desde la carpeta seguirás sin poder ${limites}. ` : "") +
        "\n\n¿Activar el modo edición?";
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
            <div className="acciones" style={{ marginTop: 12 }}>
              <button
                className="btn"
                disabled={ocupado === detalle.id}
                title="La medición se guarda y no caduca: si el servidor cambia, hay que volver a comprobarlo."
                onClick={() => void medir(detalle, false)}
              >
                {ocupado === detalle.id ? "Comprobando…" : "Volver a comprobar"}
              </button>
              <button
                className="btn"
                disabled={ocupado === detalle.id}
                onClick={() => {
                  if (window.confirm(`${avisoEscritura(detalle)}\n\n¿Comprobar ahora?`)) {
                    void medir(detalle, true);
                  }
                }}
              >
                Comprobar también si acepta subidas
              </button>
            </div>
          </div>
          {detalle.capacidades ? (
            <PanelCapacidades
              caps={detalle.capacidades}
              gestor={presets.find((p) => p.id === detalle.preset)?.donde_gestionar ?? null}
            />
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

          {conexiones !== null && conexiones.length > 0 && (
            <div className="tarjeta preferencia">
              <label>
                <input
                  type="checkbox"
                  checked={auto}
                  onChange={async (e) => {
                    const v = e.target.checked;
                    setAuto(v);
                    try {
                      await api.fijarAutoarranque(v);
                    } catch (err) {
                      setAuto(!v);
                      setError(String(err));
                    }
                  }}
                />
                <span>
                  <strong>Arrancar al iniciar sesión</strong>
                  <em>
                    IureDav se queda en la bandeja del sistema. Cerrar la ventana no lo
                    detiene ni desmonta nada; para eso está «Salir» en la bandeja.
                  </em>
                </span>
              </label>
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
                  <>
                    <button
                      className="btn"
                      title="Vuelve a leer el listado del servidor. Los cambios hechos desde Iurefficient tardan unos minutos en aparecer solos."
                      onClick={async () => {
                        setOcupado(c.id);
                        try {
                          await api.refrescar(c.id);
                        } catch (e) {
                          setError(String(e));
                        } finally {
                          setOcupado(null);
                        }
                      }}
                    >
                      Actualizar
                    </button>
                    <button className="btn" onClick={() => void openPath(c.punto_montaje)}>
                      Abrir carpeta
                    </button>
                  </>
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

              {(c.anclados.length > 0 || c.montado) && (
                <div className="anclajes">
                  <div className="titulo">Disponible sin conexión</div>
                  {c.anclados.length === 0 && (
                    <p className="pista">
                      Marca las carpetas que quieras poder abrir sin internet. Se
                      descargan y se guardan en la caché de tu equipo.
                    </p>
                  )}
                  {c.anclados.map((r) => {
                    const a = anclando[`${c.id}:${r}`];
                    return (
                      <div className="anclaje" key={r}>
                        <span className="ruta">{r}</span>
                        <span className="estado">
                          {a && !a.terminado
                            ? `descargando… ${a.archivos} archivos`
                            : a?.terminado
                              ? `${a.archivos} archivos${a.fallidos ? `, ${a.fallidos} sin descargar` : ""}`
                              : "lista"}
                        </span>
                        <button className="btn plano" onClick={async () => {
                          await api.desanclar(c.id, r);
                          await recargar();
                        }}>
                          Quitar
                        </button>
                      </div>
                    );
                  })}
                  {c.montado && (
                    <button className="btn" style={{ marginTop: 8 }} onClick={() => void anclar(c)}>
                      Añadir carpeta
                    </button>
                  )}
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
