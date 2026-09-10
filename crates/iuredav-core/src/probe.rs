//! Sonda de capacidades: descubre lo que el servidor hace de verdad.
//!
//! El servidor de Iurefficient anuncia en `Allow:` verbos que no cumple. Cualquier
//! cliente que se crea ese anuncio planifica mal y falla al ejecutar. Asi que aqui
//! no se lee el anuncio para decidir nada: se prueba cada verbo.
//!
//! La sonda tiene dos fases:
//!
//! * **Fase A (solo lectura)** — siempre se ejecuta. No deja rastro en el servidor.
//! * **Fase B (escritura)** — opt-in, porque **no se puede limpiar lo que crea**:
//!   DELETE devuelve 403. Por eso escribe siempre en la misma ruta fija y
//!   reconocible, [`RUTA_SELFTEST`], de modo que repetir el diagnostico genere
//!   versiones de un unico documento en vez de acumular ficheros nuevos.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Response, StatusCode, Url};
use tracing::{debug, info, warn};

use crate::caps::{
    Anunciado, InformeLocks, SemanticaSobrescritura, ServerCapabilities, SoporteRango, Verdict,
};
use crate::dav::{metodo, parse_multistatus, DavEntry};
use crate::presets::Preset;

/// Ruta de la sonda de escritura del perfil de Iurefficient. Se conserva como
/// constante porque la CLI la nombra en su aviso; las rutas reales salen del
/// [`Preset`], para que la misma sonda sirva con cualquier servidor.
pub const RUTA_SELFTEST: &str = "General/.iuredav-selftest.txt";

const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:propfind xmlns:D="DAV:"><D:prop>
<D:resourcetype/><D:getcontentlength/><D:getlastmodified/><D:getetag/>
</D:prop></D:propfind>"#;

const LOCK_BODY: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:lockinfo xmlns:D="DAV:">
<D:lockscope><D:exclusive/></D:lockscope>
<D:locktype><D:write/></D:locktype>
<D:owner><D:href>iuredav-selftest</D:href></D:owner>
</D:lockinfo>"#;

pub struct Probe {
    client: Client,
    base: Url,
    usuario: String,
    password: String,
    preset: Preset,
}

/// Clasifica un codigo de estado. La distincion importa: un 403 es un "no"
/// deliberado alrededor del cual se puede disenar; un 5xx es un verbo roto, que el
/// cliente no puede distinguir de una caida pasajera.
fn clasificar(status: StatusCode) -> Verdict {
    if status.is_success() {
        Verdict::Funciona
    } else if status.is_server_error() {
        Verdict::Roto {
            status: status.as_u16(),
        }
    } else {
        Verdict::Rechazado {
            status: status.as_u16(),
        }
    }
}

impl Probe {
    /// Sonda con el perfil de Iurefficient. Atajo para la CLI y los tests.
    pub fn nuevo(url: &str, usuario: &str, password: &str) -> Result<Self> {
        Self::con_preset(url, usuario, password, Preset::iurefficient())
    }

    pub fn con_preset(url: &str, usuario: &str, password: &str, preset: Preset) -> Result<Self> {
        let mut base = url.trim().to_string();
        if !base.ends_with('/') {
            base.push('/');
        }
        let base = Url::parse(&base).with_context(|| format!("URL inválida: {base}"))?;
        if base.scheme() != "https" && base.host_str() != Some("localhost") {
            warn!("la conexión no es HTTPS: la contraseña de aplicación viajará en claro");
        }
        Ok(Self {
            client: construir_cliente()?,
            base,
            usuario: usuario.to_string(),
            password: password.to_string(),
            preset,
        })
    }

    /// Carpeta de prueba para MKCOL, junto al fichero de diagnostico.
    fn ruta_dir_prueba(&self) -> String {
        format!(
            "{}-dir/",
            self.preset.ruta_selftest.trim_end_matches(".txt")
        )
    }

    /// Destino de prueba para MOVE, junto al fichero de diagnostico.
    fn ruta_movido(&self) -> String {
        format!(
            "{}-movido.txt",
            self.preset.ruta_selftest.trim_end_matches(".txt")
        )
    }

    fn url(&self, rel: &str) -> Result<Url> {
        self.base
            .join(rel)
            .with_context(|| format!("ruta inválida: {rel}"))
    }

    async fn peticion(&self, verbo: &str, rel: &str) -> Result<reqwest::RequestBuilder> {
        Ok(self
            .client
            .request(metodo(verbo), self.url(rel)?)
            .basic_auth(&self.usuario, Some(&self.password)))
    }

    /// Ejecuta la sonda completa. Nunca devuelve `Err` por un fallo de una prueba
    /// concreta: cada fallo se registra como [`Verdict`] y el informe sale entero.
    /// Solo falla si no se puede ni contactar con el servidor.
    pub async fn ejecutar(&self, con_escritura: bool) -> Result<ServerCapabilities> {
        let mut caps = ServerCapabilities::nuevo(self.base.as_str());

        self.fase_lectura(&mut caps).await?;

        if con_escritura {
            info!(
                ruta = %self.preset.ruta_selftest,
                "fase de escritura: si DELETE da 403, el fichero de prueba quedará en el servidor"
            );
            self.fase_escritura(&mut caps).await;
            caps.sonda_escritura = true;
        }

        Ok(caps)
    }

    // ---------------------------------------------------------------- fase A

    async fn fase_lectura(&self, caps: &mut ServerCapabilities) -> Result<()> {
        // 1. OPTIONS: guardamos lo que el servidor *dice*, solo para contrastarlo.
        match self.peticion("OPTIONS", "").await?.send().await {
            Ok(r) => {
                caps.anunciado = leer_anuncio(&r);
                debug!(allow = ?caps.anunciado.allow, "el servidor anuncia");
            }
            Err(e) => {
                return Err(anyhow!("no se pudo contactar con el servidor: {e}"));
            }
        }

        // 2. PROPFIND Depth 0 sobre la raiz: la prueba minima de que hay un WebDAV vivo.
        caps.real.propfind_depth0 = match self.propfind("", 0).await {
            Ok((v, _)) => v,
            Err(e) => Verdict::Error {
                detalle: e.to_string(),
            },
        };

        // Si ni siquiera esto funciona, lo mas probable es que la contrasena de
        // aplicacion no sea valida. Merece un mensaje propio.
        if let Verdict::Rechazado { status: 401 } = caps.real.propfind_depth0 {
            return Err(anyhow!(
                "401 No autorizado: la contraseña de aplicación (iurdav_…) no es válida o fue revocada"
            ));
        }

        // 3. PROPFIND Depth 1: el listado real de la raiz.
        let entradas = match self.propfind("", 1).await {
            Ok((v, e)) => {
                caps.real.propfind_depth1 = v;
                e
            }
            Err(e) => {
                caps.real.propfind_depth1 = Verdict::Error {
                    detalle: e.to_string(),
                };
                Vec::new()
            }
        };

        let base_path = self.base.path().trim_end_matches('/').to_string();
        caps.real.raiz = entradas
            .iter()
            .filter(|e| e.href.trim_end_matches('/') != base_path)
            .map(|e| e.nombre())
            .filter(|n| !n.is_empty())
            .collect();

        // 4. Un fichero real para probar GET y Range. Se busca dentro de `General/`,
        //    que es el arbol comun; nunca dentro de `Casos/`, para no tocar
        //    documentos de clientes.
        let fichero = self.buscar_fichero(self.preset.carpeta_muestra()).await;

        if let Some(f) = &fichero {
            caps.real.etag = f.etag.is_some();
            caps.real.last_modified = f.last_modified.is_some();
            let (get, rangos) = self.probar_get(f).await;
            caps.real.get = get;
            caps.real.rangos = rangos;
        } else {
            info!(
                carpeta = self.preset.carpeta_muestra(),
                "no se encontró ningún fichero: no se pueden probar GET ni Range"
            );
        }

        Ok(())
    }

    async fn propfind(&self, rel: &str, depth: u8) -> Result<(Verdict, Vec<DavEntry>)> {
        let r = self
            .peticion("PROPFIND", rel)
            .await?
            .header("Depth", depth.to_string())
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(PROPFIND_BODY)
            .send()
            .await?;

        let status = r.status();
        // 207 Multi-Status es el exito esperado.
        if !status.is_success() {
            return Ok((clasificar(status), Vec::new()));
        }
        let cuerpo = r.text().await.unwrap_or_default();
        let entradas = parse_multistatus(&cuerpo).unwrap_or_default();
        Ok((Verdict::Funciona, entradas))
    }

    /// Primer fichero (no coleccion) dentro de `rel`.
    async fn buscar_fichero(&self, rel: &str) -> Option<DavEntry> {
        let (_, entradas) = self.propfind(rel, 1).await.ok()?;
        entradas
            .into_iter()
            .find(|e| !e.es_coleccion && e.content_length.unwrap_or(0) > 0)
    }

    async fn probar_get(&self, f: &DavEntry) -> (Verdict, SoporteRango) {
        let url = match Url::parse(self.base.as_str()).and_then(|u| u.join(&f.href)) {
            Ok(u) => u,
            Err(e) => {
                return (
                    Verdict::Error {
                        detalle: e.to_string(),
                    },
                    SoporteRango::Desconocido,
                )
            }
        };

        let get = match self
            .client
            .get(url.clone())
            .basic_auth(&self.usuario, Some(&self.password))
            .send()
            .await
        {
            Ok(r) => clasificar(r.status()),
            Err(e) => Verdict::Error {
                detalle: e.to_string(),
            },
        };

        if !get.usable() {
            return (get, SoporteRango::Desconocido);
        }

        // Range: pedimos los primeros 16 bytes. Si el servidor responde 206, la
        // lectura por trozos es viable y el montaje puede abrir ficheros grandes sin
        // descargarlos enteros.
        let rangos = match self
            .client
            .get(url)
            .basic_auth(&self.usuario, Some(&self.password))
            .header("Range", "bytes=0-15")
            .send()
            .await
        {
            Ok(r) if r.status() == StatusCode::PARTIAL_CONTENT => SoporteRango::Soportado,
            Ok(_) => SoporteRango::Ignorado,
            Err(_) => SoporteRango::Desconocido,
        };

        (get, rangos)
    }

    // ---------------------------------------------------------------- fase B

    /// Orden deliberado: DELETE va el ultimo, para que si resulta que *si* funciona
    /// se lleve por delante el fichero de prueba y no quede rastro.
    async fn fase_escritura(&self, caps: &mut ServerCapabilities) {
        let r = &mut caps.real;

        let selftest = self.preset.ruta_selftest.clone();
        r.put_crear = self.probar_put(&selftest, b"iuredav selftest v1").await;

        if r.put_crear.usable() {
            r.put_sobrescribir = self.probar_sobrescritura().await;
            r.proppatch_modtime = self.probar_proppatch().await;
            r.locks = self.probar_locks().await;
        }

        r.mkcol = self.probar_simple("MKCOL", &self.ruta_dir_prueba()).await;

        if r.put_crear.usable() {
            r.mover = self.probar_move().await;
            r.borrar = self.probar_simple("DELETE", &selftest).await;
            if !r.borrar.usable() {
                warn!("DELETE no funciona: {selftest} queda en el servidor (esperado)");
            }
        }
    }

    async fn probar_put(&self, rel: &str, cuerpo: &'static [u8]) -> Verdict {
        match self.peticion("PUT", rel).await {
            Ok(b) => match b.body(cuerpo).send().await {
                Ok(r) => clasificar(r.status()),
                Err(e) => Verdict::Error {
                    detalle: e.to_string(),
                },
            },
            Err(e) => Verdict::Error {
                detalle: e.to_string(),
            },
        }
    }

    async fn probar_simple(&self, verbo: &str, rel: &str) -> Verdict {
        match self.peticion(verbo, rel).await {
            Ok(b) => match b.send().await {
                Ok(r) => clasificar(r.status()),
                Err(e) => Verdict::Error {
                    detalle: e.to_string(),
                },
            },
            Err(e) => Verdict::Error {
                detalle: e.to_string(),
            },
        }
    }

    /// Segundo PUT con contenido distinto + GET. Si el GET devuelve lo nuevo, el
    /// servidor sobrescribe *o* versiona; en Iurefficient versiona, y eso lo
    /// distinguimos porque el propio servidor lo documenta asi. Lo que si podemos
    /// medir aqui es el caso patologico: que el GET siga devolviendo lo viejo.
    async fn probar_sobrescritura(&self) -> SemanticaSobrescritura {
        const NUEVO: &[u8] = b"iuredav selftest v2 - contenido distinto";
        let selftest = &self.preset.ruta_selftest;
        if !self.probar_put(selftest, NUEVO).await.usable() {
            return SemanticaSobrescritura::Desconocido;
        }

        let leido = match self.peticion("GET", selftest).await {
            Ok(b) => match b.send().await {
                Ok(r) => r.text().await.unwrap_or_default(),
                Err(_) => return SemanticaSobrescritura::Desconocido,
            },
            Err(_) => return SemanticaSobrescritura::Desconocido,
        };

        if leido.as_bytes() == NUEVO {
            // El contenido nuevo es el que se sirve. En este servidor eso significa
            // que se creo una version; en un WebDAV estandar, que se sobrescribio.
            // Se marca como version porque es lo que aplica aqui y es la hipotesis
            // conservadora: obliga a la UI a avisar del coste de cada guardado.
            SemanticaSobrescritura::CreaVersion
        } else if leido.starts_with("iuredav selftest v1") {
            SemanticaSobrescritura::SinEfecto
        } else {
            SemanticaSobrescritura::Desconocido
        }
    }

    async fn probar_proppatch(&self) -> Verdict {
        const BODY: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:propertyupdate xmlns:D="DAV:"><D:set><D:prop>
<D:getlastmodified>Wed, 01 Jan 2025 00:00:00 GMT</D:getlastmodified>
</D:prop></D:set></D:propertyupdate>"#;

        let r = match self.peticion("PROPPATCH", &self.preset.ruta_selftest).await {
            Ok(b) => {
                b.header("Content-Type", "application/xml; charset=utf-8")
                    .body(BODY)
                    .send()
                    .await
            }
            Err(e) => {
                return Verdict::Error {
                    detalle: e.to_string(),
                }
            }
        };

        match r {
            Err(e) => Verdict::Error {
                detalle: e.to_string(),
            },
            Ok(r) => {
                let status = r.status();
                if !status.is_success() {
                    return clasificar(status);
                }
                // Un 207 puede ser exito global con fallo por propiedad: hay que leer
                // el estado de dentro, no fiarse del de fuera.
                let cuerpo = r.text().await.unwrap_or_default();
                if cuerpo.contains("200 OK") {
                    Verdict::Funciona
                } else if cuerpo.contains("403") {
                    Verdict::Rechazado { status: 403 }
                } else {
                    Verdict::Rechazado { status: 207 }
                }
            }
        }
    }

    async fn probar_move(&self) -> Verdict {
        let movido = self.ruta_movido();
        let destino = match self.url(&movido) {
            Ok(u) => u.to_string(),
            Err(e) => {
                return Verdict::Error {
                    detalle: e.to_string(),
                }
            }
        };

        let v = match self.peticion("MOVE", &self.preset.ruta_selftest).await {
            Ok(b) => match b
                .header("Destination", &destino)
                .header("Overwrite", "T")
                .send()
                .await
            {
                Ok(r) => clasificar(r.status()),
                Err(e) => Verdict::Error {
                    detalle: e.to_string(),
                },
            },
            Err(e) => {
                return Verdict::Error {
                    detalle: e.to_string(),
                }
            }
        };

        // Si de verdad funciona, lo devolvemos a su sitio para no dejar el arbol
        // desordenado, y para que el DELETE de despues encuentre el fichero.
        if v.usable() {
            let origen = self
                .url(&self.preset.ruta_selftest)
                .map(|u| u.to_string())
                .unwrap_or_default();
            if let Ok(b) = self.peticion("MOVE", &movido).await {
                let _ = b
                    .header("Destination", origen)
                    .header("Overwrite", "T")
                    .send()
                    .await;
            }
        }
        v
    }

    /// Toma un lock y trata de tomar otro desde una conexion nueva. Si el segundo
    /// lo consigue, los locks no cruzan procesos: el servidor los guarda en memoria
    /// y corre con `gunicorn --workers 3`, asi que cada worker tiene su propia tabla.
    /// Eso es justo lo que hace fallar a Office y a Finder de forma intermitente.
    async fn probar_locks(&self) -> InformeLocks {
        let primero = match self.peticion("LOCK", &self.preset.ruta_selftest).await {
            Ok(b) => {
                b.header("Content-Type", "application/xml; charset=utf-8")
                    .header("Timeout", "Second-60")
                    .header("Depth", "0")
                    .body(LOCK_BODY)
                    .send()
                    .await
            }
            Err(e) => {
                return InformeLocks {
                    lock: Verdict::Error {
                        detalle: e.to_string(),
                    },
                    cruza_procesos: None,
                }
            }
        };

        let (verdict, token) = match primero {
            Err(e) => {
                return InformeLocks {
                    lock: Verdict::Error {
                        detalle: e.to_string(),
                    },
                    cruza_procesos: None,
                }
            }
            Ok(r) => {
                let status = r.status();
                let token = r
                    .headers()
                    .get("Lock-Token")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                (clasificar(status), token)
            }
        };

        if !verdict.usable() {
            return InformeLocks {
                lock: verdict,
                cruza_procesos: None,
            };
        }

        // Varios intentos desde clientes nuevos: el reparto de peticiones entre
        // workers es rotatorio, asi que un solo intento podria caer en el mismo
        // worker que tiene el lock y dar un falso negativo.
        let mut cruza = Some(true);
        for intento in 0..5 {
            let Ok(otro) = construir_cliente() else { break };
            let Ok(url) = self.url(RUTA_SELFTEST) else {
                break;
            };
            let r = otro
                .request(metodo("LOCK"), url)
                .basic_auth(&self.usuario, Some(&self.password))
                .header("Content-Type", "application/xml; charset=utf-8")
                .header("Timeout", "Second-60")
                .header("Depth", "0")
                .body(LOCK_BODY)
                .send()
                .await;
            if let Ok(r) = r {
                if r.status().is_success() {
                    warn!(
                        intento,
                        "un segundo LOCK tuvo éxito sobre un recurso ya bloqueado"
                    );
                    cruza = Some(false);
                    break;
                }
            }
        }

        // Soltamos el lock para no dejar el fichero bloqueado 60 s.
        if let Some(t) = token {
            if let Ok(b) = self.peticion("UNLOCK", &self.preset.ruta_selftest).await {
                let _ = b.header("Lock-Token", t).send().await;
            }
        }

        InformeLocks {
            lock: verdict,
            cruza_procesos: cruza,
        }
    }
}

fn construir_cliente() -> Result<Client> {
    Client::builder()
        .user_agent(concat!("iuredav/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        // Sin reutilizar conexiones: para la prueba de locks necesitamos poder
        // llegar a workers distintos de gunicorn.
        .pool_max_idle_per_host(0)
        .build()
        .context("no se pudo construir el cliente HTTP")
}

fn leer_anuncio(r: &Response) -> Anunciado {
    let lista = |cabecera: &str| -> Vec<String> {
        r.headers()
            .get(cabecera)
            .and_then(|v| v.to_str().ok())
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    Anunciado {
        allow: lista("Allow"),
        dav: lista("DAV"),
        server: r
            .headers()
            .get("Server")
            .and_then(|v| v.to_str().ok())
            .map(String::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_403_es_un_no_deliberado_y_un_502_es_un_verbo_roto() {
        assert_eq!(
            clasificar(StatusCode::FORBIDDEN),
            Verdict::Rechazado { status: 403 }
        );
        assert_eq!(
            clasificar(StatusCode::BAD_GATEWAY),
            Verdict::Roto { status: 502 }
        );
        assert_eq!(clasificar(StatusCode::CREATED), Verdict::Funciona);
        assert_eq!(clasificar(StatusCode::MULTI_STATUS), Verdict::Funciona);
    }

    #[test]
    fn la_url_base_se_normaliza_con_barra_final() {
        let p = Probe::nuevo("https://ejemplo.test/webdav", "a@b.c", "iurdav_x").unwrap();
        assert!(p.base.as_str().ends_with('/'));
        // Sin la barra final, `join` se comeria el ultimo segmento y la sonda
        // acabaria escribiendo fuera de /webdav/.
        assert_eq!(
            p.url("General/x.txt").unwrap().path(),
            "/webdav/General/x.txt"
        );
    }
}
