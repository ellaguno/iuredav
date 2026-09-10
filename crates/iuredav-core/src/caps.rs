//! Modelo de capacidades del servidor y derivacion de los flags de montaje.
//!
//! El principio de todo el proyecto: el servidor de Iurefficient (WsgiDAV con un
//! proveedor propio) **anuncia capacidades que no tiene**. Su cabecera `Allow:`
//! promete DELETE, COPY, MOVE y PROPPATCH, pero DELETE y MKCOL devuelven 403 y
//! MOVE devuelve 502. rclone se cree ese anuncio, planifica con el, y revienta al
//! ejecutar.
//!
//! Por eso aqui nada se deduce de lo que el servidor dice: todo sale de lo que la
//! sonda ([`crate::probe`]) comprobo de verdad. Lo que no se ha probado se asume
//! no soportado.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Resultado de probar un verbo WebDAV concreto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado", rename_all = "snake_case")]
pub enum Verdict {
    /// Comprobado: funciona de verdad.
    Funciona,
    /// El servidor lo rechaza de forma limpia y deliberada (403, 405...).
    /// Es un "no" honesto: podemos disenar alrededor.
    Rechazado { status: u16 },
    /// El servidor lo acepta en teoria pero falla al ejecutarlo (5xx).
    /// Peor que un rechazo limpio, porque el cliente no puede distinguirlo de una
    /// caida temporal.
    Roto { status: u16 },
    /// No se probo (la sonda de escritura es opt-in).
    SinProbar,
    /// La prueba no pudo completarse (red, TLS, timeout).
    Error { detalle: String },
}

impl Verdict {
    /// Solo `Funciona` habilita una capacidad. Todo lo demas es "no".
    pub fn usable(&self) -> bool {
        matches!(self, Verdict::Funciona)
    }

    pub fn descripcion(&self) -> String {
        match self {
            Verdict::Funciona => "funciona".into(),
            Verdict::Rechazado { status } => format!("rechazado ({status})"),
            Verdict::Roto { status } => format!("ROTO ({status})"),
            Verdict::SinProbar => "sin probar".into(),
            Verdict::Error { detalle } => format!("error: {detalle}"),
        }
    }
}

/// Que ocurre al hacer PUT sobre una ruta que ya existe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticaSobrescritura {
    /// Comportamiento WebDAV estandar: el contenido nuevo reemplaza al viejo.
    Sobrescribe,
    /// Iurefficient: el PUT crea una version nueva del documento. El GET posterior
    /// devuelve el contenido nuevo, pero en el servidor quedan las dos.
    CreaVersion,
    /// El GET posterior devolvio el contenido viejo: el PUT no tuvo efecto visible.
    SinEfecto,
    Desconocido,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoporteRango {
    /// Respondio 206 con el trozo pedido.
    Soportado,
    /// Ignoro el `Range:` y devolvio el fichero entero.
    Ignorado,
    Desconocido,
}

/// Los locks del servidor viven en memoria y hay `gunicorn --workers 3`, asi que
/// un lock tomado por un worker es invisible para los otros dos. Office y Finder
/// dependen de que los locks sean fiables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InformeLocks {
    pub lock: Verdict,
    /// `Some(false)` = confirmado que un segundo LOCK desde otra conexion tuvo
    /// exito sobre un recurso ya bloqueado, es decir, los locks NO son fiables.
    pub cruza_procesos: Option<bool>,
}

/// Lo que el servidor *dice* de si mismo. Se guarda solo para contrastarlo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Anunciado {
    pub allow: Vec<String>,
    pub dav: Vec<String>,
    pub server: Option<String>,
}

/// Lo que el servidor *hace*. Esta es la fuente de verdad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Real {
    pub propfind_depth0: Verdict,
    pub propfind_depth1: Verdict,
    pub get: Verdict,
    pub rangos: SoporteRango,
    /// PROPFIND devolvio `getetag` para los ficheros.
    pub etag: bool,
    /// PROPFIND devolvio `getlastmodified`.
    pub last_modified: bool,
    pub put_crear: Verdict,
    pub put_sobrescribir: SemanticaSobrescritura,
    pub mkcol: Verdict,
    pub mover: Verdict,
    pub borrar: Verdict,
    /// PROPPATCH de `getlastmodified`: si no funciona, no hay modtime escribible.
    pub proppatch_modtime: Verdict,
    pub locks: InformeLocks,
    /// Nombres de primer nivel encontrados (se espera `Casos` y `General`).
    pub raiz: Vec<String>,
}

impl Default for Real {
    fn default() -> Self {
        Self {
            propfind_depth0: Verdict::SinProbar,
            propfind_depth1: Verdict::SinProbar,
            get: Verdict::SinProbar,
            rangos: SoporteRango::Desconocido,
            etag: false,
            last_modified: false,
            put_crear: Verdict::SinProbar,
            put_sobrescribir: SemanticaSobrescritura::Desconocido,
            mkcol: Verdict::SinProbar,
            mover: Verdict::SinProbar,
            borrar: Verdict::SinProbar,
            proppatch_modtime: Verdict::SinProbar,
            locks: InformeLocks {
                lock: Verdict::SinProbar,
                cruza_procesos: None,
            },
            raiz: Vec::new(),
        }
    }
}

/// Informe completo de una sonda, persistido por perfil.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub probed_at: DateTime<Utc>,
    /// URL base, sin credenciales.
    pub url: String,
    pub anunciado: Anunciado,
    pub real: Real,
    /// Si la fase de escritura llego a ejecutarse.
    pub sonda_escritura: bool,
}

/// Una discrepancia entre lo anunciado y lo real: la fila de la tabla que la UI
/// ensena en la pantalla "Estado del servidor".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discrepancia {
    pub verbo: String,
    pub anunciado: bool,
    pub real: String,
    /// Explicacion en lenguaje llano para el usuario final.
    pub explicacion: String,
}

impl ServerCapabilities {
    pub fn nuevo(url: impl Into<String>) -> Self {
        Self {
            probed_at: Utc::now(),
            url: url.into(),
            anunciado: Anunciado::default(),
            real: Real::default(),
            sonda_escritura: false,
        }
    }

    fn anuncia(&self, verbo: &str) -> bool {
        self.anunciado
            .allow
            .iter()
            .any(|v| v.eq_ignore_ascii_case(verbo))
    }

    /// Verbos que el servidor promete en `Allow:` pero que no funcionan.
    /// Si esta lista no esta vacia, el `Allow:` del servidor no es de fiar y
    /// cualquier cliente que lo crea (rclone incluido) fallara en ejecucion.
    pub fn discrepancias(&self) -> Vec<Discrepancia> {
        let mut filas = Vec::new();
        let mut revisar = |verbo: &str, v: &Verdict, explicacion: &str| {
            let anunciado = self.anuncia(verbo);
            if anunciado && !v.usable() && !matches!(v, Verdict::SinProbar) {
                filas.push(Discrepancia {
                    verbo: verbo.to_string(),
                    anunciado,
                    real: v.descripcion(),
                    explicacion: explicacion.to_string(),
                });
            }
        };
        revisar(
            "DELETE",
            &self.real.borrar,
            "Los documentos no se pueden eliminar desde la unidad. Hazlo desde Iurefficient.",
        );
        revisar(
            "MKCOL",
            &self.real.mkcol,
            "Las carpetas se crean desde Iurefficient, no desde la unidad.",
        );
        revisar(
            "MOVE",
            &self.real.mover,
            "Este servidor no permite mover ni renombrar archivos.",
        );
        revisar("PROPPATCH", &self.real.proppatch_modtime,
            "La fecha de modificacion no se puede escribir, asi que no sirve para detectar cambios.");
        filas
    }

    /// Resumen de una linea para la CLI y la bandeja del sistema.
    pub fn resumen(&self) -> String {
        let d = self.discrepancias().len();
        if d == 0 {
            "El servidor cumple lo que anuncia.".into()
        } else {
            format!("{d} capacidad(es) anunciadas que en realidad no funcionan.")
        }
    }
}

// ---------------------------------------------------------------------------
// Opciones de rclone
//
// Los nombres y tipos de aqui no son inventados: salen de `rclone rc --loopback
// options/get`, que es la lista autoritativa. Tres detalles que no se adivinan y
// que rompen el montaje en silencio si se equivocan:
//
//   * las duraciones van en **nanosegundos** como entero, no como "5m";
//   * `CacheMode` es un **entero** (0 off, 1 minimal, 2 writes, 3 full), no "full";
//   * la clave del trozo de lectura es `ChunkSize`, y `NoModTime` lleva T mayuscula.
//
// Por eso se construyen objetos JSON tipados en vez de generar una linea de
// comandos y volver a trocearla.
// ---------------------------------------------------------------------------

use serde_json::{json, Map, Value};

const NS: i64 = 1_000_000_000;
const MIN: i64 = 60 * NS;
const HORA: i64 = 60 * MIN;
const MIB: i64 = 1024 * 1024;
const GIB: i64 = 1024 * MIB;

/// `--vfs-cache-mode full`
const CACHE_MODE_FULL: i64 = 3;

/// Ajustes que elige el usuario, no el servidor.
#[derive(Debug, Clone)]
pub struct MountOptions {
    /// Modo edicion. Por defecto `false`: la unidad se monta de solo lectura.
    pub escritura: bool,
    pub cache_max_size_bytes: i64,
    pub cache_max_age_ns: i64,
    /// Retardo de subida: agrupa los autoguardados de Office para no generar una
    /// version en el servidor por cada Ctrl+S.
    pub write_back_ns: i64,
    /// Nombre visible de la unidad en el escritorio.
    pub nombre_volumen: String,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            escritura: false,
            cache_max_size_bytes: 10 * GIB,
            cache_max_age_ns: 720 * HORA,
            write_back_ns: 30 * NS,
            nombre_volumen: "Iurefficient".into(),
        }
    }
}

/// Los tres bloques que espera `mount/mount` de la API RC de rclone.
#[derive(Debug, Clone, Default)]
pub struct OpcionesRclone {
    /// Opciones globales (bloque `main`), que viajan como `_config`.
    pub config: Map<String, Value>,
    pub mount: Map<String, Value>,
    pub vfs: Map<String, Value>,
}

impl OpcionesRclone {
    /// Linea de comandos equivalente, para el panel de diagnostico y para que el
    /// usuario pueda reproducir el montaje a mano si pide soporte.
    pub fn linea_equivalente(&self, remoto: &str, punto: &str) -> String {
        let mut p = vec![
            "rclone".to_string(),
            "mount".into(),
            format!("{remoto}:"),
            punto.to_string(),
        ];
        let dur = |v: &Value| -> String {
            let ns = v.as_i64().unwrap_or(0);
            if ns == 0 {
                "0".into()
            } else if ns % HORA == 0 {
                format!("{}h", ns / HORA)
            } else if ns % MIN == 0 {
                format!("{}m", ns / MIN)
            } else {
                format!("{}s", ns / NS)
            }
        };
        if let Some(Value::Array(d)) = self.config.get("DisableFeatures") {
            let l: Vec<String> = d
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect();
            p.push(format!("--disable {}", l.join(",")));
        }
        if self.vfs.get("ReadOnly") == Some(&Value::Bool(true)) {
            p.push("--read-only".into());
        }
        if self.vfs.get("NoModTime") == Some(&Value::Bool(true)) {
            p.push("--no-modtime".into());
        }
        if self.config.get("UseServerModTime") == Some(&Value::Bool(true)) {
            p.push("--use-server-modtime".into());
        }
        p.push("--vfs-cache-mode full".into());
        if let Some(v) = self.vfs.get("CacheMaxSize") {
            p.push(format!(
                "--vfs-cache-max-size {}M",
                v.as_i64().unwrap_or(0) / MIB
            ));
        }
        if let Some(v) = self.vfs.get("CacheMaxAge") {
            p.push(format!("--vfs-cache-max-age {}", dur(v)));
        }
        if let Some(v) = self.vfs.get("WriteBack") {
            p.push(format!("--vfs-write-back {}", dur(v)));
        }
        if let Some(v) = self.vfs.get("ChunkSize") {
            p.push(format!(
                "--vfs-read-chunk-size {}M",
                v.as_i64().unwrap_or(0) / MIB
            ));
        }
        if let Some(v) = self.vfs.get("DirCacheTime") {
            p.push(format!("--dir-cache-time {}", dur(v)));
        }
        p.push("--poll-interval 0".into());
        if let Some(v) = self.mount.get("AttrTimeout") {
            p.push(format!("--attr-timeout {}", dur(v)));
        }
        for (k, f) in [
            ("Transfers", "--transfers"),
            ("Checkers", "--checkers"),
            ("LowLevelRetries", "--low-level-retries"),
            ("MultiThreadStreams", "--multi-thread-streams"),
        ] {
            if let Some(v) = self.config.get(k) {
                p.push(format!("{f} {}", v.as_i64().unwrap_or(0)));
            }
        }
        if let Some(v) = self.config.get("Timeout") {
            p.push(format!("--timeout {}", dur(v)));
        }
        p.join(" \\\n  ")
    }
}

/// Traduce las capacidades medidas a opciones de rclone.
///
/// Nada de esto esta escrito a mano en una constante: si manana el servidor
/// arregla DELETE, la sonda lo detecta y la restriccion desaparece sola.
pub fn opciones_de_montaje(caps: &ServerCapabilities, opts: &MountOptions) -> OpcionesRclone {
    let mut o = OpcionesRclone::default();

    // --- Lo mas importante de todo el fichero ---
    // rclone deduce Copy/Move/DirMove/Purge de la cabecera `Allow:`. Como ese
    // anuncio miente, le retiramos las capacidades que la sonda no pudo confirmar.
    // Sin esto, rclone planifica un renombrado como MOVE, recibe 502 y deja la
    // operacion a medias; y planifica un borrado como Purge y lo mismo.
    let mut desactivar: Vec<&str> = Vec::new();
    if !caps.real.mover.usable() {
        // COPY del lado servidor no se prueba por separado: si MOVE esta roto en
        // este proveedor, fiarse de COPY seria optimismo sin evidencia.
        desactivar.extend_from_slice(&["Move", "DirMove", "Copy"]);
    }
    if !caps.real.borrar.usable() {
        desactivar.extend_from_slice(&["Purge", "CleanUp"]);
    }
    if !desactivar.is_empty() {
        desactivar.sort_unstable();
        desactivar.dedup();
        o.config.insert("DisableFeatures".into(), json!(desactivar));
    }

    // --- Solo lectura ---
    // Se fuerza si el usuario no pidio modo edicion, y tambien si la sonda no logro
    // escribir: montar en escritura contra un servidor que rechaza PUT solo produce
    // errores confusos.
    let solo_lectura = !opts.escritura || !caps.real.put_crear.usable();
    o.vfs.insert("ReadOnly".into(), json!(solo_lectura));

    // --- Sin fecha de modificacion escribible ---
    // El servidor reporta la precision centinela de rclone (3.15e18 ns) para decir
    // "no soportado". Leemos su fecha, pero no intentamos escribirla.
    if !caps.real.proppatch_modtime.usable() {
        o.vfs.insert("NoModTime".into(), json!(true));
        o.config.insert("UseServerModTime".into(), json!(true));
    }

    // --- Cache ---
    // Sin hash y sin fecha fiable no hay forma de verificar nada contra el servidor,
    // asi que la copia local es la referencia durante la sesion. `full` ademas es lo
    // unico que permite a Office y LibreOffice abrir con acceso aleatorio.
    o.vfs.insert("CacheMode".into(), json!(CACHE_MODE_FULL));
    o.vfs
        .insert("CacheMaxSize".into(), json!(opts.cache_max_size_bytes));
    o.vfs
        .insert("CacheMaxAge".into(), json!(opts.cache_max_age_ns));
    if !solo_lectura {
        o.vfs.insert("WriteBack".into(), json!(opts.write_back_ns));
    }

    // --- Lectura por trozos ---
    if matches!(caps.real.rangos, SoporteRango::Soportado) {
        o.vfs.insert("ChunkSize".into(), json!(32 * MIB));
    }
    // Un solo flujo por fichero: al otro lado hay una aplicacion Flask, no un
    // almacen de objetos que agradezca el paralelismo.
    o.config.insert("MultiThreadStreams".into(), json!(0));

    // --- Cache de directorios ---
    // El proveedor no notifica cambios, asi que sondearlo cada minuto solo gastaria
    // peticiones. Se refresca por tiempo y con el boton "Actualizar" de la UI.
    o.vfs.insert("DirCacheTime".into(), json!(5 * MIN));
    o.vfs.insert("PollInterval".into(), json!(0));
    o.mount.insert("AttrTimeout".into(), json!(5 * NS));
    o.mount
        .insert("VolumeName".into(), json!(opts.nombre_volumen));

    // --- Concurrencia contenida ---
    // El servidor corre con `gunicorn --workers 3`. Pedirle mas paralelismo del que
    // puede atender se traduce en timeouts, no en velocidad.
    o.config.insert("Transfers".into(), json!(2));
    o.config.insert("Checkers".into(), json!(2));
    o.config.insert("Timeout".into(), json!(60 * NS));
    o.config.insert("LowLevelRetries".into(), json!(5));

    o
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reproduce el servidor de Iurefficient tal y como se midio contra INST-002:
    /// anuncia todo, cumple casi nada.
    fn caps_iurefficient() -> ServerCapabilities {
        let mut c = ServerCapabilities::nuevo("https://ejemplo.test/webdav/");
        c.anunciado.allow = [
            "OPTIONS",
            "GET",
            "HEAD",
            "PROPFIND",
            "PUT",
            "DELETE",
            "COPY",
            "MOVE",
            "PROPPATCH",
            "MKCOL",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        c.real.propfind_depth1 = Verdict::Funciona;
        c.real.get = Verdict::Funciona;
        c.real.put_crear = Verdict::Funciona;
        c.real.borrar = Verdict::Rechazado { status: 403 };
        c.real.mkcol = Verdict::Rechazado { status: 403 };
        c.real.mover = Verdict::Roto { status: 502 };
        c.real.proppatch_modtime = Verdict::Rechazado { status: 403 };
        c.real.rangos = SoporteRango::Soportado;
        c.sonda_escritura = true;
        c
    }

    #[test]
    fn detecta_que_el_allow_miente() {
        let d = caps_iurefficient().discrepancias();
        let verbos: Vec<_> = d.iter().map(|x| x.verbo.as_str()).collect();
        for v in ["DELETE", "MKCOL", "MOVE", "PROPPATCH"] {
            assert!(verbos.contains(&v), "no detecto la mentira sobre {v}");
        }
    }

    #[test]
    fn retira_de_rclone_los_verbos_que_no_funcionan() {
        let o = opciones_de_montaje(&caps_iurefficient(), &MountOptions::default());
        let d = o
            .config
            .get("DisableFeatures")
            .expect("falta DisableFeatures");
        let lista: Vec<&str> = d
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        for esperado in ["Copy", "DirMove", "Move", "Purge", "CleanUp"] {
            assert!(
                lista.contains(&esperado),
                "DisableFeatures no incluye {esperado}: {lista:?}"
            );
        }
    }

    #[test]
    fn solo_lectura_por_defecto() {
        let o = opciones_de_montaje(&caps_iurefficient(), &MountOptions::default());
        assert_eq!(o.vfs.get("ReadOnly"), Some(&json!(true)));
    }

    #[test]
    fn el_modo_edicion_no_basta_si_el_put_no_funciona() {
        let opts = MountOptions {
            escritura: true,
            ..Default::default()
        };

        let o = opciones_de_montaje(&caps_iurefficient(), &opts);
        assert_eq!(o.vfs.get("ReadOnly"), Some(&json!(false)));
        assert!(
            o.vfs.contains_key("WriteBack"),
            "en escritura hace falta agrupar los guardados"
        );

        let mut sin_put = caps_iurefficient();
        sin_put.real.put_crear = Verdict::Rechazado { status: 403 };
        let o = opciones_de_montaje(&sin_put, &opts);
        assert_eq!(
            o.vfs.get("ReadOnly"),
            Some(&json!(true)),
            "sin PUT hay que forzar solo lectura"
        );
    }

    /// Las claves y los tipos tienen que coincidir con `rclone rc --loopback
    /// options/get`. Si esto se rompe, el montaje falla en silencio.
    #[test]
    fn los_tipos_son_los_que_espera_la_api_rc() {
        let o = opciones_de_montaje(&caps_iurefficient(), &MountOptions::default());
        // CacheMode es un entero, no la cadena "full".
        assert_eq!(o.vfs.get("CacheMode"), Some(&json!(3)));
        // Las duraciones van en nanosegundos.
        assert_eq!(o.vfs.get("DirCacheTime"), Some(&json!(300_000_000_000i64)));
        assert_eq!(o.mount.get("AttrTimeout"), Some(&json!(5_000_000_000i64)));
        // El tamano en bytes, no "10G".
        assert_eq!(o.vfs.get("CacheMaxSize"), Some(&json!(10_737_418_240i64)));
        // La clave del trozo es ChunkSize, y NoModTime lleva T mayuscula.
        assert!(o.vfs.contains_key("ChunkSize"));
        assert!(o.vfs.contains_key("NoModTime"));
    }

    #[test]
    fn un_servidor_sano_no_pierde_capacidades() {
        let mut c = caps_iurefficient();
        c.real.borrar = Verdict::Funciona;
        c.real.mover = Verdict::Funciona;
        c.real.mkcol = Verdict::Funciona;
        c.real.proppatch_modtime = Verdict::Funciona;

        let o = opciones_de_montaje(&c, &MountOptions::default());
        assert!(
            !o.config.contains_key("DisableFeatures"),
            "no hay que recortar un servidor que cumple"
        );
        assert!(!o.vfs.contains_key("NoModTime"));
        assert!(c.discrepancias().is_empty());
    }

    #[test]
    fn la_linea_equivalente_es_legible() {
        let o = opciones_de_montaje(&caps_iurefficient(), &MountOptions::default());
        let l = o.linea_equivalente("iurefficient", "/home/x/Iurefficient");
        assert!(
            l.contains("--disable CleanUp,Copy,DirMove,Move,Purge"),
            "{l}"
        );
        assert!(l.contains("--read-only"));
        assert!(l.contains("--dir-cache-time 5m"), "{l}");
        assert!(l.contains("--vfs-cache-max-age 720h"), "{l}");
    }
}
