//! Ajustes generales de la aplicacion, los que no pertenecen a ninguna conexion.
//!
//! Van en un fichero aparte de los perfiles: se leen antes de que exista ninguna
//! ventana —al decidir si enseñarla— y no tiene sentido cargar y reescribir la
//! lista de conexiones para eso.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::perfiles::directorio_config;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ajustes {
    /// Cuando IureDav arranca con la sesion, quedarse en la bandeja sin abrir la
    /// ventana. Es lo que se espera de un agente que monta unidades: si el usuario
    /// lo puso a arrancar solo, no quiere verlo cada manana. Por eso es `true`
    /// por defecto. No afecta a los arranques a mano, que siempre abren la ventana.
    #[serde(default = "verdadero")]
    pub arrancar_oculto: bool,
    /// Consultar en GitHub si hay una version mas nueva y decirlo. Solo avisa,
    /// no instala. `true` por defecto; se puede apagar por quien no quiera que
    /// el programa hable con GitHub por su cuenta.
    #[serde(default = "verdadero")]
    pub avisar_actualizaciones: bool,
}

fn verdadero() -> bool {
    true
}

impl Default for Ajustes {
    fn default() -> Self {
        Self {
            arrancar_oculto: true,
            avisar_actualizaciones: true,
        }
    }
}

fn fichero() -> Result<PathBuf> {
    Ok(directorio_config()?.join("ajustes.json"))
}

/// Nunca falla: unos ajustes ilegibles no son motivo para no arrancar, y lo que
/// se pierde son preferencias que el usuario puede volver a marcar.
pub fn cargar() -> Ajustes {
    match leer() {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!(%e, "no se pudieron leer los ajustes; se usan los de fábrica");
            Ajustes::default()
        }
    }
}

fn leer() -> Result<Ajustes> {
    let f = fichero()?;
    if !f.exists() {
        return Ok(Ajustes::default());
    }
    let texto =
        fs::read_to_string(&f).with_context(|| format!("no se pudo leer {}", f.display()))?;
    serde_json::from_str(&texto).with_context(|| format!("{} esta corrupto", f.display()))
}

pub fn guardar(ajustes: &Ajustes) -> Result<()> {
    let f = fichero()?;
    // Escritura atomica, por lo mismo que los perfiles: morir a medias no puede
    // dejar el fichero truncado.
    let tmp = f.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(ajustes)?)
        .with_context(|| format!("no se pudo escribir {}", tmp.display()))?;
    fs::rename(&tmp, &f).with_context(|| format!("no se pudo reemplazar {}", f.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Es la razon de ser del ajuste: quien no ha tocado nada arranca en la bandeja.
    #[test]
    fn de_fabrica_arranca_oculto() {
        assert!(Ajustes::default().arrancar_oculto);
    }

    /// Un fichero de una version anterior, sin el campo, tiene que comportarse
    /// igual que si no existiera.
    #[test]
    fn un_fichero_sin_el_campo_arranca_oculto() {
        let a: Ajustes = serde_json::from_str("{}").unwrap();
        assert!(a.arrancar_oculto);
        assert!(a.avisar_actualizaciones);
    }

    #[test]
    fn el_ajuste_sobrevive_al_viaje_por_json() {
        let a = Ajustes {
            arrancar_oculto: false,
            avisar_actualizaciones: false,
        };
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Ajustes>(&json).unwrap(), a);
    }
}
