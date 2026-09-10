//! Supervisor del sidecar rclone.
//!
//! Se levanta **un solo** proceso `rclone rcd` de larga vida y todos los montajes
//! se piden por su API remota. Asi la UI tiene estado en vivo (`core/stats`,
//! `vfs/stats`) sin invocar la CLI una y otra vez.
//!
//! Sobre los secretos: la contrasena de aplicacion **nunca** va en `argv`, porque
//! cualquier usuario de la maquina puede leer la linea de comandos de un proceso
//! ajeno con `ps`. El remoto se crea en caliente con `config/create` y la
//! contrasena viaja en el cuerpo de la peticion RC, que solo escucha en localhost
//! y va autenticada con credenciales aleatorias de un solo uso.

use std::net::TcpListener;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::caps::OpcionesRclone;
use crate::errors::{traducir, MensajeAmistoso};

/// Version de rclone que se empaqueta como sidecar. Fijada a proposito: el
/// comportamiento de los flags de VFS cambia entre versiones y no queremos que la
/// unidad se comporte distinto segun lo que haya instalado el usuario.
pub const RCLONE_VERSION: &str = "1.75.1";

pub struct Rclone {
    proceso: Child,
    client: reqwest::Client,
    base: String,
    usuario: String,
    password: String,
}

impl Rclone {
    /// Arranca el sidecar y espera a que responda.
    ///
    /// `binario` es la ruta al rclone empaquetado; en desarrollo vale el del sistema.
    pub async fn arrancar(binario: &str) -> Result<(Self, mpsc::Receiver<MensajeAmistoso>)> {
        let puerto = puerto_libre().context("no hay puertos libres en localhost")?;
        let usuario = token_aleatorio();
        let password = token_aleatorio();

        let mut proceso = Command::new(binario)
            .arg("rcd")
            .arg("--rc-addr")
            .arg(format!("127.0.0.1:{puerto}"))
            // Las credenciales de la API de control van por **entorno**, no por
            // argv: /proc/PID/cmdline lo lee cualquier usuario de la maquina (444),
            // mientras que /proc/PID/environ solo su dueno (400). Quien leyera esas
            // credenciales podria hablar con la API local, que sabe montar remotos y
            // ejecutar ordenes.
            .env("RCLONE_RC_USER", &usuario)
            .env("RCLONE_RC_PASS", &password)
            // Sin servir objetos por HTTP: solo queremos la API de control.
            .arg("--rc-serve=false")
            .arg("--log-level")
            .arg("INFO")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("no se pudo ejecutar rclone en {binario}"))?;

        // El log de rclone sale por stderr. Lo leemos linea a linea y lo pasamos por
        // el traductor: de ahi salen las notificaciones que ve el usuario.
        let (tx, rx) = mpsc::channel(64);
        if let Some(stderr) = proceso.stderr.take() {
            tokio::spawn(async move {
                let mut lineas = BufReader::new(stderr).lines();
                while let Ok(Some(linea)) = lineas.next_line().await {
                    debug!(target: "rclone", "{linea}");
                    if let Some(m) = traducir(&linea) {
                        if tx.send(m).await.is_err() {
                            break;
                        }
                    }
                }
            });
        }

        let rc = Self {
            proceso,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()?,
            base: format!("http://127.0.0.1:{puerto}"),
            usuario,
            password,
        };

        rc.esperar_a_que_responda().await?;
        let v = rc.llamar("core/version", json!({})).await?;
        info!(version = ?v.get("version"), "sidecar rclone listo");

        Ok((rc, rx))
    }

    async fn esperar_a_que_responda(&self) -> Result<()> {
        // El arranque es casi inmediato, pero en un portatil frio con el binario sin
        // cachear puede tardar. 5 s de margen, comprobando cada 50 ms.
        for _ in 0..100 {
            if self.llamar("rc/noop", json!({})).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        bail!("el sidecar rclone no respondio en 5 segundos")
    }

    /// Llama a un metodo de la API remota de rclone.
    pub async fn llamar(&self, metodo: &str, params: Value) -> Result<Value> {
        let r = self
            .client
            .post(format!("{}/{metodo}", self.base))
            .basic_auth(&self.usuario, Some(&self.password))
            .json(&params)
            .send()
            .await
            .with_context(|| format!("fallo la llamada a {metodo}"))?;

        let status = r.status();
        let cuerpo: Value = r.json().await.unwrap_or(Value::Null);

        if !status.is_success() {
            let detalle = cuerpo
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("sin detalle")
                .to_string();
            return Err(anyhow!("{metodo} devolvio {status}: {detalle}"));
        }
        Ok(cuerpo)
    }

    /// Crea (o actualiza) el remoto WebDAV en caliente, sin tocar el fichero de
    /// configuracion de rclone ni pasar la contrasena por `argv`.
    pub async fn crear_remoto(
        &self,
        nombre: &str,
        url: &str,
        usuario: &str,
        password: &str,
    ) -> Result<()> {
        self.llamar(
            "config/create",
            json!({
                "name": nombre,
                "type": "webdav",
                "parameters": {
                    "url": url,
                    "vendor": "other",
                    "user": usuario,
                    "pass": password,
                },
                // Sin persistir en disco: el remoto vive solo mientras dure la sesion.
                "opt": { "obscure": true, "nonInteractive": true },
            }),
        )
        .await?;
        Ok(())
    }

    /// Monta el remoto. Las opciones salen de [`crate::caps::opciones_de_montaje`],
    /// derivadas de lo que la sonda midio, no de una lista escrita a mano.
    pub async fn montar(&self, remoto: &str, punto: &str, o: &OpcionesRclone) -> Result<()> {
        let mut peticion = serde_json::Map::new();
        peticion.insert("fs".into(), json!(format!("{remoto}:")));
        peticion.insert("mountPoint".into(), json!(punto));
        peticion.insert("mountType".into(), json!(self.tipo_de_montaje().await?));
        peticion.insert("mountOpt".into(), Value::Object(o.mount.clone()));
        peticion.insert("vfsOpt".into(), Value::Object(o.vfs.clone()));
        // `_config` es como la API RC acepta las opciones globales del bloque `main`;
        // ahi es donde viaja DisableFeatures, que es lo que impide que rclone se crea
        // el `Allow:` del servidor.
        peticion.insert("_config".into(), Value::Object(o.config.clone()));

        self.llamar("mount/mount", Value::Object(peticion)).await?;
        info!(punto, "montado");
        Ok(())
    }

    /// Mecanismos de montaje que este rclone dice tener.
    pub async fn tipos_de_montaje(&self) -> Result<Vec<String>> {
        let v = self.llamar("mount/types", json!({})).await?;
        Ok(v.get("mountTypes")
            .and_then(|t| t.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn tipo_de_montaje(&self) -> Result<String> {
        let tipo = elegir_tipo(&self.tipos_de_montaje().await?)?;
        debug!(tipo, "mecanismo de montaje elegido");
        Ok(tipo)
    }

    pub async fn desmontar(&self, punto: &str) -> Result<()> {
        self.llamar("mount/unmount", json!({ "mountPoint": punto }))
            .await?;
        Ok(())
    }

    pub async fn montajes(&self) -> Result<Value> {
        self.llamar("mount/listmounts", json!({})).await
    }

    pub async fn estadisticas(&self) -> Result<Value> {
        self.llamar("core/stats", json!({})).await
    }

    /// Olvida la cache de directorios de una ruta: es el boton "Actualizar" de la UI.
    /// Hace falta porque el proveedor no notifica cambios y montamos con
    /// `--poll-interval 0`.
    pub async fn refrescar(&self, remoto: &str, ruta: &str) -> Result<()> {
        self.llamar(
            "vfs/forget",
            json!({ "fs": format!("{remoto}:"), "dir": ruta }),
        )
        .await?;
        Ok(())
    }

    pub async fn apagar(mut self) -> Result<()> {
        let _ = self.llamar("mount/unmountall", json!({})).await;
        let _ = self.llamar("core/quit", json!({})).await;
        // Margen para que desmonte limpiamente antes de matarlo.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = self.proceso.kill().await;
        Ok(())
    }
}

/// Orden de preferencia de mecanismos de montaje para esta plataforma.
///
/// En macOS se prefiere `nfsmount`: rclone levanta un servidor NFS local y el
/// sistema lo monta, lo que evita pedirle al usuario que instale macFUSE. Esa es
/// la mayor friccion de instalacion de este tipo de aplicaciones, y quitarla vale
/// mas que cualquier otra cosa que podamos hacer en el instalador.
fn preferencias() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["nfsmount", "mount", "cmount", "mount2"]
    } else if cfg!(target_os = "windows") {
        &["mount", "cmount"] // ambos sobre WinFsp
    } else {
        &["mount", "mount2", "nfsmount"]
    }
}

/// Elige el primer mecanismo preferido que rclone diga tener.
///
/// No se codifica un nombre fijo porque los nombres cambian entre versiones: en
/// rclone 1.60 solo existen `mount` y `mount2`; `nfsmount` aparece despues. Un
/// nombre inventado hace que `mount/mount` falle con un error que no dice nada.
fn elegir_tipo(disponibles: &[String]) -> Result<String> {
    for preferido in preferencias() {
        if disponibles.iter().any(|d| d == preferido) {
            return Ok((*preferido).to_string());
        }
    }
    bail!(
        "este rclone no ofrece ningun mecanismo de montaje utilizable (tiene: {}). \
         En Windows suele significar que falta WinFsp; en macOS, que el binario esta incompleto",
        if disponibles.is_empty() {
            "ninguno".to_string()
        } else {
            disponibles.join(", ")
        }
    )
}

fn puerto_libre() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0")
        .ok()?
        .local_addr()
        .ok()
        .map(|a| a.port())
}

/// Credencial de un solo uso para la API RC local, del generador del sistema.
///
/// Es un secreto de verdad —quien lo tenga puede pedirle a rclone que monte lo que
/// quiera—, asi que sale del CSPRNG del sistema operativo y no de un hasher.
fn token_aleatorio() -> String {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).expect("el generador de aleatorios del sistema no responde");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Ruta al rclone empaquetado.
///
/// Tauri coloca los binarios externos junto al ejecutable principal, ya sin el
/// sufijo del triple. En desarrollo se busca ademas el descargado por
/// `scripts/descargar-rclone.py`, que si lo lleva.
pub fn ruta_binario() -> String {
    let nombre = if cfg!(windows) {
        "rclone.exe"
    } else {
        "rclone"
    };

    // Valvula de escape para pruebas y para quien quiera usar su propio rclone.
    if let Ok(p) = std::env::var("IUREDAV_RCLONE") {
        if !p.is_empty() {
            return p;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            for candidato in [d.join(nombre), d.join("resources").join(nombre)] {
                if candidato.exists() {
                    return candidato.to_string_lossy().into_owned();
                }
            }
        }
    }

    // Desarrollo: el binario descargado, con el triple en el nombre.
    let triple = env!("IUREDAV_TARGET");
    if !triple.is_empty() {
        let sufijo = if cfg!(windows) { ".exe" } else { "" };
        let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../src-tauri/binaries")
            .join(format!("iuredav-rclone-{triple}{sufijo}"));
        if dev.exists() {
            return dev.to_string_lossy().into_owned();
        }
    }

    warn!("usando el rclone del sistema; en produccion debe ir empaquetado");
    nombre.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caps::{opciones_de_montaje, MountOptions, ServerCapabilities, Verdict};

    #[test]
    fn elige_el_mecanismo_preferido_de_los_que_hay() {
        // rclone 1.75 en macOS: nfsmount evita tener que instalar macFUSE.
        let modernos: Vec<String> = ["mount", "mount2", "nfsmount"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let elegido = elegir_tipo(&modernos).unwrap();
        if cfg!(target_os = "macos") {
            assert_eq!(elegido, "nfsmount");
        } else {
            assert_eq!(elegido, "mount");
        }

        // rclone 1.60 no conoce nfsmount: hay que caer en algo que si exista, no
        // pedir un nombre inventado.
        let antiguos: Vec<String> = ["mount", "mount2"].iter().map(|s| s.to_string()).collect();
        assert_eq!(elegir_tipo(&antiguos).unwrap(), "mount");
    }

    #[test]
    fn sin_ningun_mecanismo_el_error_orienta() {
        let e = elegir_tipo(&[]).unwrap_err().to_string();
        assert!(
            e.contains("WinFsp"),
            "el error deberia orientar sobre la causa: {e}"
        );
    }

    #[test]
    fn los_tokens_no_se_repiten() {
        assert_ne!(token_aleatorio(), token_aleatorio());
        assert_eq!(token_aleatorio().len(), 64);
    }

    /// Las credenciales de la API de control no pueden acabar en la linea de
    /// comandos: ahi las lee cualquier usuario de la maquina. Esta prueba fija esa
    /// decision para que no se pierda en un refactor.
    #[test]
    fn las_credenciales_rc_no_van_en_argv() {
        let fuente = include_str!("rclone.rs");
        let arranque = &fuente[fuente.find("pub async fn arrancar").unwrap()
            ..fuente.find("// El log de rclone sale por stderr").unwrap()];
        assert!(
            !arranque.contains(r#".arg("--rc-user")"#),
            "--rc-user volvio a argv"
        );
        assert!(
            !arranque.contains(r#".arg("--rc-pass")"#),
            "--rc-pass volvio a argv"
        );
        assert!(arranque.contains(r#".env("RCLONE_RC_USER""#));
        assert!(arranque.contains(r#".env("RCLONE_RC_PASS""#));
    }

    /// La opcion que impide que rclone se crea el `Allow:` del servidor tiene que
    /// llegar en `_config`, que es el unico bloque donde la API RC la acepta.
    #[test]
    fn disable_features_viaja_en_config() {
        let mut c = ServerCapabilities::nuevo("https://x/webdav/");
        c.real.mover = Verdict::Roto { status: 502 };
        c.real.borrar = Verdict::Rechazado { status: 403 };
        let o = opciones_de_montaje(&c, &MountOptions::default());

        assert!(o.config.contains_key("DisableFeatures"));
        assert!(!o.mount.contains_key("DisableFeatures"));
        assert!(!o.vfs.contains_key("DisableFeatures"));
    }
}
