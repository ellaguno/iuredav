//! Inicio de sesion con la cuenta de Iurefficient.
//!
//! En vez de pedir al usuario que genere una contrasena de aplicacion `iurdav_...`
//! en su perfil web, IureDav inicia sesion con su correo y contrasena (y el codigo
//! de dos pasos si lo tiene), le pide a la instancia una contrasena de aplicacion
//! a nombre de este equipo y la guarda en el llavero compartido con las demas
//! aplicaciones de escritorio. El usuario nunca ve el `iurdav_...`.
//!
//! La contrasena de la cuenta no se guarda nunca: solo la sesion (cookies) y la
//! contrasena de aplicacion resultante.

use anyhow::{Context, Result};
use iurefficient_connect::rest::{Login, Session};
use iurefficient_connect::{api, secrets, user_agent, Account};

/// Resultado de un intento de inicio de sesion.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoLogin {
    /// La cuenta tiene verificacion en dos pasos: repite la llamada con `totp_token` y el codigo.
    pub requiere_totp: bool,
    pub totp_token: Option<String>,
    /// Contrasena de aplicacion WebDAV lista para usar (creada o reutilizada del llavero).
    pub password_app: Option<String>,
    /// Nombre de la persona segun la instancia.
    pub nombre: Option<String>,
    /// `true` si la contrasena de aplicacion ya existia en el llavero compartido.
    pub reutilizada: bool,
}

fn agente() -> String {
    user_agent("IureDav", env!("CARGO_PKG_VERSION"))
}

/// Nombre con el que se etiqueta la contrasena de aplicacion en la instancia.
pub fn etiqueta_equipo() -> String {
    let equipo = std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|h| h.trim().to_string())
        })
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "este equipo".to_string());
    format!("IureDav en {equipo}")
}

/// Inicia sesion y devuelve una contrasena de aplicacion WebDAV.
///
/// `dominio` acepta lo mismo que el formulario (`2.ds.ejemplo.com`, con o sin
/// `https://` o `/webdav/`). Con `totp = Some((token, codigo))` completa el segundo
/// paso de una cuenta con verificacion en dos pasos.
pub async fn iniciar_sesion(
    dominio: &str,
    correo: &str,
    password: &str,
    totp: Option<(&str, &str)>,
) -> Result<ResultadoLogin> {
    let cuenta = Account::new(dominio, correo)?;
    let sesion = Session::new(cuenta.clone(), &agente())?;
    let usuario = match totp {
        Some((token, codigo)) if !token.is_empty() => sesion
            .verify_totp(token, codigo)
            .await
            .context("no se pudo verificar el codigo")?,
        _ => match sesion.login(password).await? {
            Login::Ok(u) => u,
            Login::TotpRequired { totp_token } => {
                return Ok(ResultadoLogin {
                    requiere_totp: true,
                    totp_token: Some(totp_token),
                    password_app: None,
                    nombre: None,
                    reutilizada: false,
                });
            }
        },
    };

    // Sesion compartida con IureTranscribe e IureEditor (mismo llavero), y cuenta
    // activa para que arranquen ya conectadas sin volver a pedir dominio ni correo.
    if let Ok(json) = serde_json::to_string(&sesion.export()) {
        let _ = secrets::guardar(&cuenta, secrets::Kind::Session, &json);
    }
    let _ = iurefficient_connect::account::set_active(&cuenta, "IureDav");

    // Si otra app ya creo una contrasena de aplicacion para esta cuenta, se reutiliza.
    if let Ok(Some(existente)) = secrets::leer(&cuenta, secrets::Kind::WebDav) {
        if !existente.trim().is_empty() {
            return Ok(ResultadoLogin {
                requiere_totp: false,
                totp_token: None,
                password_app: Some(existente),
                nombre: nombre_de(&usuario),
                reutilizada: true,
            });
        }
    }

    let creada = api::create_webdav_token(&sesion, &etiqueta_equipo(), None)
        .await
        .context("no se pudo crear la contrasena de aplicacion")?;
    let _ = secrets::guardar(&cuenta, secrets::Kind::WebDav, &creada.secret);
    Ok(ResultadoLogin {
        requiere_totp: false,
        totp_token: None,
        password_app: Some(creada.secret),
        nombre: nombre_de(&usuario),
        reutilizada: false,
    })
}

/// Cuenta con la que otra app de Iurefficient inicio sesion en este equipo, para
/// rellenar el formulario de conexion: (dominio, correo).
pub fn cuenta_activa() -> Option<(String, String)> {
    iurefficient_connect::account::active().map(|a| (a.domain, a.email))
}

fn nombre_de(u: &iurefficient_connect::rest::User) -> Option<String> {
    u.name
        .clone()
        .or_else(|| {
            u.extra
                .get("full_name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .filter(|n| !n.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_etiqueta_nombra_a_iuredav() {
        assert!(etiqueta_equipo().starts_with("IureDav en "));
    }
}
