//! Guarda la contrasena de aplicacion en el llavero del sistema.
//!
//! Nunca en un fichero de configuracion: los perfiles se sincronizan, se copian y
//! acaban en copias de seguridad. El llavero (Secret Service en Linux, Llaveros en
//! macOS, Administrador de credenciales en Windows) los cifra con la sesion del
//! usuario y es lo que espera cualquier aplicacion de escritorio seria.

use anyhow::{Context, Result};
use iurefficient_connect::lang::pick;
use keyring::Entry;

const SERVICIO: &str = "iuredav";

fn entrada(perfil_id: &str, usuario: &str) -> Result<Entry> {
    // La clave incluye el perfil para que la misma persona pueda tener credenciales
    // distintas en instancias distintas.
    Entry::new(SERVICIO, &format!("{perfil_id}:{usuario}")).with_context(|| {
        pick(
            "couldn't open the system keychain",
            "no se pudo abrir el llavero del sistema",
        )
    })
}

pub fn guardar(perfil_id: &str, usuario: &str, password: &str) -> Result<()> {
    entrada(perfil_id, usuario)?
        .set_password(password)
        .with_context(|| {
            pick(
                "couldn't save the password to the keychain",
                "no se pudo guardar la contraseña en el llavero",
            )
        })
}

/// `Ok(None)` si no hay nada guardado, que es distinto de que el llavero falle.
pub fn leer(perfil_id: &str, usuario: &str) -> Result<Option<String>> {
    match entrada(perfil_id, usuario)?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).with_context(|| {
            pick(
                "couldn't read from the keychain",
                "no se pudo leer del llavero",
            )
        }),
    }
}

pub fn borrar(perfil_id: &str, usuario: &str) -> Result<()> {
    match entrada(perfil_id, usuario)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e).with_context(|| {
            pick(
                "couldn't delete from the keychain",
                "no se pudo borrar del llavero",
            )
        }),
    }
}
