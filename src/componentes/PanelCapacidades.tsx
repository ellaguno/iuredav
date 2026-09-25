import { Capacidades, Verdict, describir, funciona } from "../api";
import { Clave, t, useIdioma } from "../i18n";

/** Traduce cada límite a algo que una persona pueda accionar.
 *
 * `sitio` es el nombre de la aplicación web del servidor. Contra un WebDAV
 * cualquiera no lo sabemos, así que se habla en genérico en vez de inventarlo. */
function explicar(verbo: string, sitio: string): string {
  switch (verbo) {
    case "DELETE": return t("caps.explica.DELETE", { sitio });
    case "MKCOL": return t("caps.explica.MKCOL", { sitio });
    case "MOVE": return t("caps.explica.MOVE");
    case "PROPPATCH": return t("caps.explica.PROPPATCH");
    default: return t("caps.explica.otro", { verbo });
  }
}

function Celda({ v }: { v: Verdict }) {
  if (funciona(v)) return <span className="si">{t("caps.si")}</span>;
  if (v.estado === "roto") return <span className="mal">{describir(v)}</span>;
  return <span className="no">{describir(v)}</span>;
}

export default function PanelCapacidades({
  caps,
  gestor,
}: {
  caps: Capacidades;
  /** Nombre de la aplicación web del servidor, o `null` si no se conoce. */
  gestor: string | null;
}) {
  useIdioma();
  const sitio = gestor ? t("donde.gestor", { gestor }) : t("donde.generico");
  const r = caps.real;
  const anuncia = (v: string) => caps.anunciado.allow.some((a) => a.toUpperCase() === v);

  const filas: Array<{ verbo: string; v: Verdict; que: string }> = (
    [
      ["PROPFIND", r.propfind_depth1],
      ["GET", r.get],
      ["PUT", r.put_crear],
      ["DELETE", r.borrar],
      ["MKCOL", r.mkcol],
      ["MOVE", r.mover],
      ["PROPPATCH", r.proppatch_modtime],
      ["LOCK", r.locks.lock],
    ] as Array<[string, Verdict]>
  ).map(([verbo, v]) => ({ verbo, v, que: t(`caps.que.${verbo}` as Clave) }));

  const discrepancias = filas.filter(
    (f) => anuncia(f.verbo) && !funciona(f.v) && f.v.estado !== "sin_probar",
  );

  return (
    <>
      {discrepancias.length > 0 && (
        <div className="tarjeta">
          <h2>{t("caps.discrepa.titulo")}</h2>
          <p style={{ color: "var(--tenue)", marginTop: 4 }}>
            {t("caps.discrepa.texto", {
              lista: discrepancias.map((d) => d.que.toLowerCase()).join(", "),
            })}
          </p>
          {discrepancias.map((d) => (
            <div className="nota limite" key={d.verbo}>
              <strong>{d.que}</strong>
              <p>{explicar(d.verbo, sitio)}</p>
            </div>
          ))}
        </div>
      )}

      <div className="tarjeta">
        <h2>{t("caps.comprobado")}</h2>
        <table className="caps" style={{ marginTop: 12 }}>
          <thead>
            <tr>
              <th>{t("caps.operacion")}</th>
              <th>{t("caps.anuncia")}</th>
              <th>{t("caps.funciona")}</th>
            </tr>
          </thead>
          <tbody>
            {filas.map((f) => {
              const discrepa = anuncia(f.verbo) && !funciona(f.v) && f.v.estado !== "sin_probar";
              return (
                <tr key={f.verbo} className={discrepa ? "discrepa" : ""}>
                  <td>
                    {f.que} <span className="no">({f.verbo})</span>
                  </td>
                  <td>{anuncia(f.verbo) ? t("caps.si") : <span className="no">{t("caps.no")}</span>}</td>
                  <td>
                    <Celda v={f.v} />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="tarjeta">
        <h2>{t("caps.detalles")}</h2>
        <table className="caps" style={{ marginTop: 12 }}>
          <tbody>
            <tr>
              <td>{t("caps.etag")}</td>
              <td>
                {r.etag ? (
                  <span className="si">{t("caps.etag.si")}</span>
                ) : (
                  <span className="mal">{t("caps.etag.no")}</span>
                )}
              </td>
            </tr>
            <tr>
              <td>{t("caps.rangos")}</td>
              <td>
                {r.rangos === "soportado" ? (
                  <span className="si">{t("caps.rangos.si")}</span>
                ) : (
                  <span className="no">{t("caps.no")}</span>
                )}
              </td>
            </tr>
            <tr>
              <td>{t("caps.sobrescribir")}</td>
              <td>
                {r.put_sobrescribir === "crea_version" ? (
                  <span className="mal">{t("caps.sobrescribir.version")}</span>
                ) : r.put_sobrescribir === "sobrescribe" ? (
                  <span className="si">{t("caps.sobrescribir.si")}</span>
                ) : (
                  <span className="no">{t("caps.sinComprobar")}</span>
                )}
              </td>
            </tr>
            <tr>
              <td>{t("caps.locks")}</td>
              <td>
                {r.locks.cruza_procesos === false ? (
                  <span className="mal">{t("caps.locks.no")}</span>
                ) : r.locks.cruza_procesos === true ? (
                  <span className="si">{t("caps.locks.si")}</span>
                ) : (
                  <span className="no">{t("caps.sinComprobar")}</span>
                )}
              </td>
            </tr>
          </tbody>
        </table>
        {!caps.sonda_escritura && (
          <div className="nota aviso">
            <strong>{t("caps.soloLectura.titulo")}</strong>
            <p>{t("caps.soloLectura.texto")}</p>
          </div>
        )}
      </div>
    </>
  );
}
