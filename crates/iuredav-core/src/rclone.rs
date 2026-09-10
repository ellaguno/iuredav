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
            .arg("--rc-user")
            .arg(&usuario)
            .arg("--rc-pass")
            .arg(&password)
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
    pub async fn crear_remoto(&self, nombre: &str, url: &str, usuario: &str, password: &str) -> Result<()> {
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
        peticion.insert("mountType".into(), json!(tipo_de_montaje()));
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

    pub async fn desmontar(&self, punto: &str) -> Result<()> {
        self.llamar("mount/unmount", json!({ "mountPoint": punto })).await?;
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
        self.llamar("vfs/forget", json!({ "fs": format!("{remoto}:"), "dir": ruta })).await?;
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

/// En macOS se monta por NFS, no por FUSE: rclone levanta un servidor NFS local y
/// el sistema lo monta. Eso evita pedirle al usuario que instale macFUSE, que es
/// la mayor friccion de instalacion de este tipo de aplicaciones.
fn tipo_de_montaje() -> &'static str {
    if cfg!(target_os = "macos") {
        "nfs"
    } else {
        ""  // vacio = el predeterminado de la plataforma (FUSE en Linux, WinFsp en Windows)
    }
}

fn puerto_libre() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0").ok()?.local_addr().ok().map(|a| a.port())
}

/// Credencial de un solo uso para la API RC local.
fn token_aleatorio() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    // Suficiente para un secreto local y efimero, y evita arrastrar una dependencia
    // de generacion de aleatorios solo para esto.
    let mut s = String::new();
    for _ in 0..4 {
        let h = RandomState::new().build_hasher().finish();
        s.push_str(&format!("{h:016x}"));
    }
    s
}

/// Ruta al rclone empaquetado; si no esta, el del sistema (util en desarrollo).
pub fn ruta_binario() -> String {
    let nombre = if cfg!(windows) { "rclone.exe" } else { "rclone" };
    if let Ok(dir) = std::env::current_exe() {
        if let Some(d) = dir.parent() {
            let candidato = d.join("resources").join(nombre);
            if candidato.exists() {
                return candidato.to_string_lossy().into_owned();
            }
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
    fn los_tokens_no_se_repiten() {
        assert_ne!(token_aleatorio(), token_aleatorio());
        assert_eq!(token_aleatorio().len(), 64);
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
