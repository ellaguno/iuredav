import { Capacidades, Verdict, describir, funciona } from "../api";

/** Traduce cada limite a algo que una persona pueda accionar. */
const EXPLICACION: Record<string, string> = {
  DELETE: "Los documentos no se pueden eliminar desde la unidad. Hazlo desde Iurefficient.",
  MKCOL: "Las carpetas se crean desde Iurefficient, no desde la unidad.",
  MOVE: "No se puede mover ni renombrar archivos desde la unidad.",
  PROPPATCH: "La fecha de modificación no se puede escribir, así que no sirve para detectar cambios.",
};

function Celda({ v }: { v: Verdict }) {
  if (funciona(v)) return <span className="si">sí</span>;
  if (v.estado === "roto") return <span className="mal">{describir(v)}</span>;
  return <span className="no">{describir(v)}</span>;
}

export default function PanelCapacidades({ caps }: { caps: Capacidades }) {
  const r = caps.real;
  const anuncia = (v: string) => caps.anunciado.allow.some((a) => a.toUpperCase() === v);

  const filas: Array<{ verbo: string; v: Verdict; que: string }> = [
    { verbo: "PROPFIND", v: r.propfind_depth1, que: "Listar carpetas" },
    { verbo: "GET", v: r.get, que: "Abrir documentos" },
    { verbo: "PUT", v: r.put_crear, que: "Subir documentos" },
    { verbo: "DELETE", v: r.borrar, que: "Eliminar" },
    { verbo: "MKCOL", v: r.mkcol, que: "Crear carpetas" },
    { verbo: "MOVE", v: r.mover, que: "Mover o renombrar" },
    { verbo: "PROPPATCH", v: r.proppatch_modtime, que: "Fijar la fecha" },
    { verbo: "LOCK", v: r.locks.lock, que: "Bloquear mientras se edita" },
  ];

  const discrepancias = filas.filter(
    (f) => anuncia(f.verbo) && !funciona(f.v) && f.v.estado !== "sin_probar",
  );

  return (
    <>
      {discrepancias.length > 0 && (
        <div className="tarjeta">
          <h2>Este servidor anuncia cosas que no cumple</h2>
          <p style={{ color: "var(--tenue)", marginTop: 4 }}>
            Dice saber hacer {discrepancias.map((d) => d.verbo).join(", ")}, pero al
            intentarlo falla. IureDav se lo ha retirado para que no lo intente y deje
            operaciones a medias.
          </p>
          {discrepancias.map((d) => (
            <div className="nota limite" key={d.verbo}>
              <strong>{d.que}</strong>
              <p>{EXPLICACION[d.verbo] ?? `El servidor rechaza ${d.verbo}.`}</p>
            </div>
          ))}
        </div>
      )}

      <div className="tarjeta">
        <h2>Lo que se comprobó</h2>
        <table className="caps" style={{ marginTop: 12 }}>
          <thead>
            <tr>
              <th>Operación</th>
              <th>Lo anuncia</th>
              <th>Funciona de verdad</th>
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
                  <td>{anuncia(f.verbo) ? "sí" : <span className="no">no</span>}</td>
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
        <h2>Detalles que afectan a la sincronización</h2>
        <table className="caps" style={{ marginTop: 12 }}>
          <tbody>
            <tr>
              <td>Huella de contenido (ETag)</td>
              <td>
                {r.etag ? (
                  <span className="si">disponible</span>
                ) : (
                  <span className="mal">ausente — no se puede detectar un cambio por contenido</span>
                )}
              </td>
            </tr>
            <tr>
              <td>Lectura por trozos</td>
              <td>
                {r.rangos === "soportado" ? (
                  <span className="si">sí — los archivos grandes se abren sin descargarlos enteros</span>
                ) : (
                  <span className="no">no</span>
                )}
              </td>
            </tr>
            <tr>
              <td>Guardar sobre un documento existente</td>
              <td>
                {r.put_sobrescribir === "crea_version" ? (
                  <span className="mal">crea una versión nueva, no sobrescribe</span>
                ) : r.put_sobrescribir === "sobrescribe" ? (
                  <span className="si">sobrescribe</span>
                ) : (
                  <span className="no">sin comprobar</span>
                )}
              </td>
            </tr>
            <tr>
              <td>Bloqueos entre programas</td>
              <td>
                {r.locks.cruza_procesos === false ? (
                  <span className="mal">no son fiables — Office puede fallar de vez en cuando</span>
                ) : r.locks.cruza_procesos === true ? (
                  <span className="si">fiables</span>
                ) : (
                  <span className="no">sin comprobar</span>
                )}
              </td>
            </tr>
          </tbody>
        </table>
        {!caps.sonda_escritura && (
          <div className="nota aviso">
            <strong>Solo se comprobó la lectura</strong>
            <p>
              El diagnóstico de escritura crea un archivo de prueba que, en un servidor
              sin DELETE, después no se puede borrar. Por eso hay que pedirlo aparte.
            </p>
          </div>
        )}
      </div>
    </>
  );
}
