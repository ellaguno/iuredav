import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { open as elegirCarpeta } from "@tauri-apps/plugin-dialog";
import { AcercaDe, Actualizacion, AvanceAnclaje, Aviso, Capacidades, Conexion, Preset, Requisito, api, describir } from "./api";
import FormularioConexion from "./componentes/FormularioConexion";
import Marca from "./componentes/Marca";
import PanelCapacidades from "./componentes/PanelCapacidades";
import AppsIurefficient from "./componentes/AppsIurefficient";
import { Clave, fijarIdioma, t, tn, useIdioma } from "./i18n";

/** Un verbo del protocolo no le dice nada a nadie: se nombra la accion. */
const ACCION: Record<string, Clave> = {
  DELETE: "accion.DELETE",
  MKCOL: "accion.MKCOL",
  MOVE: "accion.MOVE",
  COPY: "accion.COPY",
  // PROPPATCH se omite a proposito: no es algo que nadie intente hacer, asi que en
  // el titular seria ruido. Sigue apareciendo en la tabla del detalle.
};

/** "eliminar documentos, crear carpetas ni mover" en vez de "delete, mkcol, move". */
function enumerar(verbos: string[]): string {
  const partes = verbos
    .map((v) => ACCION[v.toUpperCase()])
    .filter(Boolean)
    .map((k) => t(k));
  if (partes.length === 0) return "";
  if (partes.length === 1) return partes[0];
  return `${partes.slice(0, -1).join(", ")}${t("lista.ni")}${partes[partes.length - 1]}`;
}

type Vista = { pantalla: "lista" } | { pantalla: "nueva" } | { pantalla: "detalle"; id: string };

export default function App() {
  const idioma = useIdioma();
  const [preferido, setPreferido] = useState("auto");
  const [vista, setVista] = useState<Vista>({ pantalla: "lista" });
  const [conexiones, setConexiones] = useState<Conexion[] | null>(null);
  const [avisos, setAvisos] = useState<Aviso[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [ocupado, setOcupado] = useState<string | null>(null);
  const [falta, setFalta] = useState<Requisito | null>(null);
  const [destino, setDestino] = useState("");
  const [presets, setPresets] = useState<Preset[]>([]);
  const [auto, setAuto] = useState(false);
  const [oculto, setOculto] = useState(true);
  const [avisar, setAvisar] = useState(true);
  const [nueva, setNueva] = useState<Actualizacion | null>(null);
  const [nuevaVista, setNuevaVista] = useState(false);
  const [actualizando, setActualizando] = useState<string | null>(null);
  const [acerca, setAcerca] = useState<AcercaDe | null>(null);
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
  // cuando el usuario pulsa Montar y recibe un error del sistema. Se vuelve a
  // mirar cada vez que la ventana recupera el foco: lo normal es instalar WinFsp
  // o FUSE con IureDav abierto, y el aviso debe desaparecer solo.
  useEffect(() => {
    const comprobar = () => { api.comprobarSistema().then(setFalta).catch(() => {}); };
    comprobar();
    window.addEventListener("focus", comprobar);
    return () => window.removeEventListener("focus", comprobar);
  }, [idioma]);

  // Lo que llega redactado desde Rust se vuelve a pedir al cambiar de idioma.
  useEffect(() => {
    api.nombreDestino().then(setDestino).catch(() => {});
    api.listarPresets().then(setPresets).catch(() => {});
  }, [idioma]);

  useEffect(() => {
    api.idiomaPreferido().then(setPreferido).catch(() => {});
    api.autoarranque().then(setAuto).catch(() => {});
    api.arranqueOculto().then(setOculto).catch(() => {});
    api.avisarActualizaciones().then(setAvisar).catch(() => {});
    api.acercaDe().then(setAcerca).catch(() => {});
    // Se pregunta al abrir, y no solo se escucha: si se arrancó en la bandeja, la
    // comprobación pudo hacerse horas antes de que existiera esta ventana.
    api.actualizacion().then(setNueva).catch(() => {});
  }, []);

  useEffect(() => {
    const p = listen<Actualizacion>("iuredav://actualizacion", (e) => setNueva(e.payload));
    return () => { void p.then((quitar) => quitar()); };
  }, []);

  // Enlaces `iuredav://montar?perfil=<id>` (y `iuredav://nueva`): llegan al abrir la
  // app con el enlace o, si ya corría, desde la segunda instancia.
  const atenderEnlaces = useCallback(async (enlaces: string[]) => {
    for (const raw of enlaces) {
      let u: URL;
      try { u = new URL(raw); } catch { continue; }
      const accion = (u.host || u.pathname.replace(/^\/+/, "")).replace(/\/+$/, "").toLowerCase();
      if (accion === "nueva") setVista({ pantalla: "nueva" });
      else if (accion === "montar") {
        const perfil = u.searchParams.get("perfil");
        const lista = await api.listar().catch(() => null);
        const c = lista?.find((x) => x.id === perfil) ?? (lista?.length === 1 ? lista[0] : undefined);
        if (!c) { setError(t("enlace.noHay", { perfil: perfil ?? "" })); continue; }
        if (!c.montado) {
          setOcupado(c.id);
          try { await api.montar(c.id, c.escritura); } catch (e) { setError(String(e)); } finally { setOcupado(null); }
        }
        await recargar();
      }
    }
  }, [recargar]);
  useEffect(() => {
    api.enlacesIniciales().then((e) => { if (e.length) void atenderEnlaces(e); }).catch(() => {});
    const p = listen<string[]>("iuredav://enlace", (e) => void atenderEnlaces(e.payload));
    return () => { void p.then((quitar) => quitar()); };
  }, [atenderEnlaces]);

  // Los limites del servidor llegan traducidos desde Rust y se muestran tal cual.
  useEffect(() => {
    const p = listen<Aviso>("iuredav://aviso", (e) => {
      setAvisos((prev) => [e.payload, ...prev].slice(0, 4));
    });
    return () => { void p.then((quitar) => quitar()); };
  }, []);

  // Unidades que se vuelven a montar solas al arrancar (lo que quedó montado).
  useEffect(() => {
    const p = listen("iuredav://montajes", () => void recargar());
    return () => { void p.then((quitar) => quitar()); };
  }, [recargar]);

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
      title: t("anclajes.elegir"),
    });
    if (typeof elegida !== "string") return;
    try {
      await api.anclar(c.id, elegida);
      await recargar();
    } catch (e) {
      setError(String(e));
    }
  }

  /** Dónde sí puede el usuario hacer lo que la unidad le niega. */
  function donde(c: Conexion): string {
    const gestor = presets.find((p) => p.id === c.preset)?.donde_gestionar;
    return gestor ? t("donde.gestor", { gestor }) : t("donde.generico");
  }

  /** Lo que hay que decir antes de la fase de escritura: deja un archivo. */
  function avisoEscritura(c: Conexion): string {
    const ruta = presets.find((p) => p.id === c.preset)?.ruta_selftest ?? ".iuredav-selftest.txt";
    return t("avisoEscritura", { ruta });
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
      else {
        // Por si se instaló el requisito sin que la ventana perdiera el foco.
        setFalta(await api.comprobarSistema());
        await api.montar(c.id, c.escritura);
      }
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
          `${t("editar.hayQueComprobar")}\n\n${avisoEscritura(c)}\n\n${t("editar.comprobarloAhora")}`,
        );
        if (!quiere) return;

        const medido = await medir(c, true);
        if (!medido) return;
        if (medido.real.put_crear.estado !== "funciona") {
          setError(t("editar.noAceptaSubidas", { estado: describir(medido.real.put_crear) }));
          return;
        }
        caps = medido;
      }

      const versiona = caps.real.put_sobrescribir === "crea_version";
      const limites = enumerar(c.limites);
      const aviso =
        (versiona ? t("editar.versiona") : "") +
        (limites ? t("editar.seguirasSinPoder", { limites }) : "") +
        `\n\n${t("editar.activar")}`;
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
      setError(t("olvidar.antesDesmonta"));
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
      <div className="contenido">
      <header className="cabecera">
        <Marca />
        <div className="crece">
          <h1>IureDav</h1>
          <div style={{ fontSize: 12, color: "var(--tenue)" }}>
            {t("cabecera.lema")}
          </div>
        </div>
        {vista.pantalla === "lista" && (
          <button className="btn principal" onClick={() => setVista({ pantalla: "nueva" })}>
            {t("cabecera.anadir")}
          </button>
        )}
        {vista.pantalla !== "lista" && (
          <button className="btn" onClick={() => setVista({ pantalla: "lista" })}>
            {t("cabecera.volver")}
          </button>
        )}
      </header>

      {falta && (
        <div className="tarjeta">
          <h2>{t("falta.titulo", { que: falta.que_falta })}</h2>
          <p style={{ color: "var(--tenue)", marginTop: 4 }}>{falta.por_que}</p>
          <div className="nota aviso">
            <strong>{t("falta.queHacer")}</strong>
            <p>{falta.como_instalar}</p>
          </div>
          {falta.url && (
            <div className="acciones">
              <span className="crece" />
              <button className="btn principal" onClick={() => void openUrl(falta.url!)}>
                {t("falta.descargar", { que: falta.que_falta })}
              </button>
            </div>
          )}
        </div>
      )}

      {error && <div className="error-caja">{error}</div>}

      {nueva && !nuevaVista && (
        <div className="nota aviso">
          <strong>{t("nueva.titulo", { version: nueva.version })}</strong>
          <p>
            {actualizando
              ? actualizando
              : t("nueva.texto")}
          </p>
          <div className="acciones" style={{ marginTop: 8 }}>
            <button
              className="btn principal"
              disabled={!!actualizando}
              onClick={() => {
                setActualizando(t("nueva.comprobando"));
                import("./actualizador")
                  .then((m) =>
                    m.instalarActualizacion(
                      () => Promise.resolve(true),
                      (t) => setActualizando(t),
                    ),
                  )
                  .then((r) => {
                    if (r === "no-disponible") {
                      setActualizando(null);
                      void openUrl(nueva.url);
                    }
                  })
                  .catch((e) => {
                    setActualizando(null);
                    setError(t("nueva.error", { error: String(e) }));
                  });
              }}
            >
              {t("nueva.actualizar")}
            </button>
            <button className="btn plano" onClick={() => void openUrl(nueva.url)}>
              {t("nueva.verDescarga")}
            </button>
            <button className="btn plano" disabled={!!actualizando} onClick={() => setNuevaVista(true)}>
              {t("nueva.ahoraNo")}
            </button>
          </div>
        </div>
      )}

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
                title={t("detalle.tituloMedir")}
                onClick={() => void medir(detalle, false)}
              >
                {ocupado === detalle.id ? t("detalle.comprobando") : t("detalle.volverAComprobar")}
              </button>
              <button
                className="btn"
                disabled={ocupado === detalle.id}
                onClick={() => {
                  if (window.confirm(`${avisoEscritura(detalle)}\n\n${t("detalle.comprobarAhora")}`)) {
                    void medir(detalle, true);
                  }
                }}
              >
                {t("detalle.comprobarSubidas")}
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
                {t("detalle.sinComprobar")}
              </p>
            </div>
          )}
        </>
      )}

      {vista.pantalla === "lista" && (
        <>
          {conexiones === null && <div className="cargando">{t("lista.cargando")}</div>}

          {conexiones?.length === 0 && (
            <div className="vacio">
              <h2>{t("vacio.titulo")}</h2>
              <p>{t("vacio.texto")}</p>
              <button
                className="btn principal grande"
                style={{ marginTop: 14 }}
                onClick={() => setVista({ pantalla: "nueva" })}
              >
                {t("vacio.boton")}
              </button>
            </div>
          )}

          {conexiones !== null && (
            <div className="tarjeta preferencia">
              {/* El idioma se puede cambiar aunque no haya conexiones: es lo primero
                  que busca quien no entiende la ventana. */}
              <div className="idioma">
                <span>
                  <strong>{t("pref.idioma")}</strong>
                  <em>{t("pref.idioma.detalle")}</em>
                </span>
                <select
                  aria-label={t("pref.idioma")}
                  value={preferido}
                  onChange={async (e) => {
                    const v = e.target.value;
                    const antes = preferido;
                    setPreferido(v);
                    try {
                      fijarIdioma(await api.fijarIdioma(v));
                    } catch (err) {
                      setPreferido(antes);
                      setError(String(err));
                    }
                  }}
                >
                  <option value="auto">{t("pref.idioma.auto")}</option>
                  <option value="en">English</option>
                  <option value="es">Español</option>
                </select>
              </div>
              {conexiones.length > 0 && (
              <>
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
                  <strong>{t("pref.autoarranque")}</strong>
                  <em>{t("pref.autoarranque.detalle")}</em>
                </span>
              </label>
              {/* Solo tiene sentido si arranca con la sesión: a mano, la ventana se
                  abre siempre, porque quien hizo doble clic quiere verla. */}
              <label className={`anidada ${auto ? "" : "apagada"}`}>
                <input
                  type="checkbox"
                  checked={oculto}
                  disabled={!auto}
                  onChange={async (e) => {
                    const v = e.target.checked;
                    setOculto(v);
                    try {
                      await api.fijarArranqueOculto(v);
                    } catch (err) {
                      setOculto(!v);
                      setError(String(err));
                    }
                  }}
                />
                <span>
                  <strong>{t("pref.minimizado")}</strong>
                  <em>{t("pref.minimizado.detalle")}</em>
                </span>
              </label>
              <label className="anidada" style={{ paddingLeft: 0, marginTop: 14 }}>
                <input
                  type="checkbox"
                  checked={avisar}
                  onChange={async (e) => {
                    const v = e.target.checked;
                    setAvisar(v);
                    try {
                      await api.fijarAvisarActualizaciones(v);
                      if (!v) setNueva(null);
                    } catch (err) {
                      setAvisar(!v);
                      setError(String(err));
                    }
                  }}
                />
                <span>
                  <strong>{t("pref.avisar")}</strong>
                  <em>{t("pref.avisar.detalle")}</em>
                </span>
              </label>
              </>
              )}
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
                      {c.montado ? t("conexion.montado") : t("conexion.desmontado")}
                    </span>
                    <span className="pastilla">{c.escritura ? t("conexion.edicion") : t("conexion.soloLectura")}</span>
                  </div>
                  <div className="ruta">
                    {destino}: {c.punto_montaje}
                  </div>
                </div>

                {c.montado && (
                  <>
                    <button
                      className="btn"
                      title={t("conexion.tituloActualizar")}
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
                      {t("conexion.actualizar")}
                    </button>
                    <button
                      className="btn"
                      onClick={() => api.abrirCarpeta(c.id).catch((e) => setError(String(e)))}
                    >
                      {t("conexion.abrirCarpeta")}
                    </button>
                  </>
                )}
                <button
                  className={`btn ${c.montado ? "" : "principal"}`}
                  onClick={() => void alternar(c)}
                  disabled={ocupado === c.id}
                >
                  {ocupado === c.id ? "…" : c.montado ? t("conexion.desmontar") : t("conexion.montar")}
                </button>
              </div>

              {enumerar(c.limites) && (
                <div className="nota limite">
                  <strong>{t("conexion.limites.titulo")}</strong>
                  <p>{t("conexion.limites.texto", { lista: enumerar(c.limites), donde: donde(c) })}</p>
                </div>
              )}

              {(c.anclados.length > 0 || c.montado) && (
                <div className="anclajes">
                  <div className="titulo">{t("anclajes.titulo")}</div>
                  {c.anclados.length === 0 && (
                    <p className="pista">{t("anclajes.pista")}</p>
                  )}
                  {c.anclados.map((r) => {
                    const a = anclando[`${c.id}:${r}`];
                    return (
                      <div className="anclaje" key={r}>
                        <span className="ruta">{r}</span>
                        <span className="estado">
                          {a && !a.terminado
                            ? tn("anclajes.descargando", a.archivos)
                            : a?.terminado
                              ? tn("anclajes.archivos", a.archivos) +
                                (a.fallidos ? t("anclajes.sinDescargar", { n: a.fallidos }) : "")
                              : t("anclajes.lista")}
                        </span>
                        <button className="btn plano" onClick={async () => {
                          await api.desanclar(c.id, r);
                          await recargar();
                        }}>
                          {t("anclajes.quitar")}
                        </button>
                      </div>
                    );
                  })}
                  {c.montado && (
                    <button className="btn" style={{ marginTop: 8 }} onClick={() => void anclar(c)}>
                      {t("anclajes.anadir")}
                    </button>
                  )}
                </div>
              )}

              <div className="acciones" style={{ marginTop: 12 }}>
                <button className="btn plano" onClick={() => setVista({ pantalla: "detalle", id: c.id })}>
                  {t("conexion.verCapacidades")}
                </button>
                <button className="btn plano" onClick={() => void cambiarModo(c)} disabled={c.montado}>
                  {c.escritura ? t("conexion.pasarASoloLectura") : t("conexion.permitirEdicion")}
                </button>
                <span className="crece" />
                <button className="btn plano peligro" onClick={() => void olvidar(c)}>
                  {t("conexion.eliminar")}
                </button>
              </div>
            </div>
          ))}
        </>
      )}
      {vista.pantalla === "lista" && <AppsIurefficient version={acerca?.version} />}
      </div>

      <footer className="pie">
        <p className="lema">
          <strong>Iurefficient</strong> {t("pie.lema")}
        </p>
        <nav className="enlaces">
          <button onClick={() => void openUrl("https://iurefficient.com")}>iurefficient.com</button>
          <button onClick={() => void openUrl("https://demo.iurefficient.com")}>{t("pie.demo")}</button>
        </nav>
        {acerca && (
          <p className="version">
            IureDav {acerca.version}
            {nueva && t("pie.nueva", { version: nueva.version })}
            {" · "}
            <button onClick={() => void openUrl(acerca.url_releases)}>{t("pie.todas")}</button>
          </p>
        )}
      </footer>
    </div>
  );
}
