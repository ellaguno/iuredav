//! Perfiles de conexion.
//!
//! Se guardan en el directorio de configuracion del usuario, **sin secretos**: la
//! contrasena vive en el llavero ([`crate::secretos`]) y aqui solo queda el usuario
//! con el que buscarla.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::caps::ServerCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perfil {
    pub id: String,
    pub nombre: String,
    /// URL base del WebDAV, con barra final.
    pub url: String,
    pub usuario: String,
    pub punto_montaje: PathBuf,
    /// Modo edicion. Por defecto `false`.
    #[serde(default)]
    pub escritura: bool,
    /// Carpetas marcadas como disponibles sin conexion.
    #[serde(default)]
    pub anclados: Vec<String>,
    /// Ultima medicion de la sonda. Es lo que determina como se monta.
    #[serde(default)]
    pub capacidades: Option<ServerCapabilities>,
}

impl Perfil {
    pub fn nuevo(id: &str, url: &str, usuario: &str) -> Self {
        let mut url = url.trim().to_string();
        if !url.ends_with('/') {
            url.push('/');
        }
        Self {
            id: id.to_string(),
            nombre: id.to_string(),
            url,
            usuario: usuario.to_string(),
            punto_montaje: punto_por_defecto(id),
            escritura: false,
            anclados: Vec::new(),
            capacidades: None,
        }
    }
}

/// `~/Iurefficient` en Linux y macOS. En Windows, una letra de unidad: montar en una
/// carpeta funciona, pero los usuarios esperan ver una unidad en el Explorador.
pub fn punto_por_defecto(id: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("I:")
    } else {
        directories::UserDirs::new()
            .map(|d| d.home_dir().join(nombre_bonito(id)))
            .unwrap_or_else(|| PathBuf::from("/tmp").join(id))
    }
}

fn nombre_bonito(id: &str) -> String {
    let mut c = id.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => "IureDav".into(),
    }
}

pub fn directorio_config() -> Result<PathBuf> {
    let d = directories::ProjectDirs::from("com", "Iurefficient", "IureDav")
        .context("no se pudo determinar el directorio de configuracion")?;
    let d = d.config_dir().to_path_buf();
    fs::create_dir_all(&d).with_context(|| format!("no se pudo crear {}", d.display()))?;
    Ok(d)
}

fn fichero() -> Result<PathBuf> {
    Ok(directorio_config()?.join("perfiles.json"))
}

pub fn cargar() -> Result<Vec<Perfil>> {
    let f = fichero()?;
    if !f.exists() {
        return Ok(Vec::new());
    }
    let texto = fs::read_to_string(&f).with_context(|| format!("no se pudo leer {}", f.display()))?;
    serde_json::from_str(&texto).with_context(|| format!("{} esta corrupto", f.display()))
}

pub fn guardar(perfiles: &[Perfil]) -> Result<()> {
    let f = fichero()?;
    // Escritura atomica: si la aplicacion muere a media escritura, el fichero viejo
    // sigue intacto en vez de quedarse truncado.
    let tmp = f.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(perfiles)?)
        .with_context(|| format!("no se pudo escribir {}", tmp.display()))?;
    fs::rename(&tmp, &f).with_context(|| format!("no se pudo reemplazar {}", f.display()))?;
    Ok(())
}

pub fn buscar(id: &str) -> Result<Option<Perfil>> {
    Ok(cargar()?.into_iter().find(|p| p.id == id))
}

/// Anade o reemplaza un perfil conservando el resto.
pub fn upsert(perfil: Perfil) -> Result<()> {
    let mut todos = cargar()?;
    match todos.iter_mut().find(|p| p.id == perfil.id) {
        Some(existente) => *existente = perfil,
        None => todos.push(perfil),
    }
    guardar(&todos)
}

/// Elimina un perfil. Devuelve `false` si no existia.
pub fn borrar(id: &str) -> Result<bool> {
    let mut todos = cargar()?;
    let antes = todos.len();
    todos.retain(|p| p.id != id);
    if todos.len() == antes {
        return Ok(false);
    }
    guardar(&todos)?;
    Ok(true)
}

/// Crea el punto de montaje si hace falta y comprueba que se puede usar.
///
/// Montar sobre una carpeta con contenido lo oculta mientras dure el montaje, que
/// es una forma bastante desagradable de "perder" archivos. Mejor negarse.
pub fn preparar_punto(p: &Path) -> Result<()> {
    if cfg!(windows) {
        return Ok(()); // Una letra de unidad no se crea.
    }
    if !p.exists() {
        fs::create_dir_all(p).with_context(|| format!("no se pudo crear {}", p.display()))?;
        return Ok(());
    }
    if !p.is_dir() {
        anyhow::bail!("{} existe y no es una carpeta", p.display());
    }
    let vacia = fs::read_dir(p)
        .with_context(|| format!("no se pudo leer {}", p.display()))?
        .next()
        .is_none();
    if !vacia {
        anyhow::bail!(
            "{} no esta vacia. Montar ahi ocultaria lo que ya contiene; elige otra carpeta",
            p.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_url_siempre_acaba_en_barra() {
        // Sin la barra, `join` se come el ultimo segmento y las peticiones acaban
        // fuera de /webdav/.
        let p = Perfil::nuevo("x", "https://a.test/webdav", "u@e.c");
        assert_eq!(p.url, "https://a.test/webdav/");
        let p = Perfil::nuevo("x", "https://a.test/webdav/", "u@e.c");
        assert_eq!(p.url, "https://a.test/webdav/");
    }

    #[test]
    fn un_perfil_nuevo_no_escribe() {
        assert!(!Perfil::nuevo("x", "https://a.test/", "u@e.c").escritura);
    }

    #[test]
    fn se_niega_a_montar_sobre_una_carpeta_con_contenido() {
        let d = std::env::temp_dir().join(format!("iuredav-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);

        preparar_punto(&d).expect("deberia crearla");
        assert!(d.is_dir());
        preparar_punto(&d).expect("vacia: vale");

        fs::write(d.join("algo.txt"), b"x").unwrap();
        let e = preparar_punto(&d).expect_err("con contenido: no");
        assert!(e.to_string().contains("no esta vacia"));

        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn el_perfil_no_guarda_la_contrasena() {
        let p = Perfil::nuevo("x", "https://a.test/", "u@e.c");
        let json = serde_json::to_string(&p).unwrap();
        for prohibido in ["pass", "password", "contrasena", "secret"] {
            assert!(!json.to_lowercase().contains(prohibido), "el perfil filtra '{prohibido}': {json}");
        }
    }
}
