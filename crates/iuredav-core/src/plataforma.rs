//! Requisitos del sistema para poder montar, y como decirselo al usuario.
//!
//! Montar un sistema de archivos en espacio de usuario necesita una pieza distinta
//! en cada plataforma. Si falta, rclone falla con un mensaje que no ayuda a nadie.
//! Aqui se comprueba **antes** de intentarlo y se explica que instalar.
//!
//! macOS es el caso interesante: no necesita nada. rclone levanta un servidor NFS
//! local y el sistema lo monta, asi que no hay que pedirle a nadie que instale
//! macFUSE ni que autorice una extension del sistema.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requisito {
    /// Nombre para la persona, no para el paquete.
    pub que_falta: String,
    pub por_que: String,
    /// Que hacer, en concreto.
    pub como_instalar: String,
    /// Donde descargarlo, si hay que descargarlo.
    pub url: Option<String>,
}

/// `Ok(())` si se puede montar en esta maquina.
pub fn comprobar() -> Result<(), Requisito> {
    #[cfg(target_os = "windows")]
    {
        if !winfsp_instalado() {
            return Err(Requisito {
                que_falta: "WinFsp".into(),
                por_que:
                    "Es lo que permite a Windows mostrar tus documentos como una unidad de disco."
                        .into(),
                como_instalar: "Descarga e instala WinFsp, y después vuelve a abrir IureDav."
                    .into(),
                url: Some("https://winfsp.dev/rel/".into()),
            });
        }
    }

    #[cfg(target_os = "linux")]
    {
        if !fuse_disponible() {
            return Err(Requisito {
                que_falta: "FUSE 3".into(),
                por_que: "Es lo que permite mostrar tus documentos como una carpeta del sistema."
                    .into(),
                como_instalar: "sudo apt install fuse3   (o el equivalente de tu distribución)"
                    .into(),
                url: None,
            });
        }
    }

    // macOS no necesita nada: se monta con el servidor NFS que trae el propio rclone.
    Ok(())
}

#[cfg(target_os = "windows")]
fn winfsp_instalado() -> bool {
    // Se mira el disco y no el registro para no arrastrar una dependencia mas: el
    // instalador de WinFsp siempre deja la biblioteca en una de estas rutas.
    const RUTAS: [&str; 4] = [
        r"C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll",
        r"C:\Program Files\WinFsp\bin\winfsp-x64.dll",
        r"C:\Program Files (x86)\WinFsp\bin\winfsp-a64.dll",
        r"C:\Windows\System32\winfsp-x64.dll",
    ];
    RUTAS.iter().any(|r| Path::new(r).exists())
}

#[cfg(target_os = "linux")]
fn fuse_disponible() -> bool {
    // Hace falta el dispositivo y el ayudante de montaje. Con uno solo no basta:
    // en contenedores es habitual tener el binario y no /dev/fuse.
    if !Path::new("/dev/fuse").exists() {
        return false;
    }
    ["fusermount3", "fusermount"].iter().any(|f| en_la_ruta(f))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn en_la_ruta(programa: &str) -> bool {
    std::env::var_os("PATH")
        .map(|rutas| std::env::split_paths(&rutas).any(|d| d.join(programa).exists()))
        .unwrap_or(false)
}

/// Como se llama el sitio donde aparecen los archivos, en cada plataforma. La UI
/// lo usa para no hablar de "punto de montaje", que no significa nada fuera de Unix.
pub fn nombre_del_destino() -> &'static str {
    if cfg!(target_os = "windows") {
        "Unidad"
    } else {
        "Carpeta"
    }
}

/// Comprueba que el destino tenga la forma que espera la plataforma.
pub fn validar_destino(destino: &str) -> Result<(), String> {
    if destino.trim().is_empty() {
        return Err("Indica donde quieres que aparezcan tus documentos.".into());
    }
    if cfg!(target_os = "windows") {
        // En Windows lo normal es una letra de unidad. Tambien vale una carpeta
        // vacia, pero la letra es lo que la gente espera ver en el Explorador.
        let d = destino.trim();
        let letra =
            d.len() == 2 && d.ends_with(':') && d.chars().next().unwrap().is_ascii_alphabetic();
        if !letra && !d.contains('\\') {
            return Err("Usa una letra de unidad como I: o una ruta de carpeta completa.".into());
        }
    } else if !destino.starts_with('/') && !destino.starts_with('~') {
        return Err("Usa una ruta completa, que empiece por /.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_destino_vacio_no_vale() {
        assert!(validar_destino("   ").is_err());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn en_unix_se_pide_ruta_absoluta() {
        assert!(validar_destino("/home/x/Iurefficient").is_ok());
        assert!(validar_destino("Iurefficient").is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn en_windows_vale_una_letra_de_unidad() {
        assert!(validar_destino("I:").is_ok());
        assert!(validar_destino(r"C:\Users\x\Iurefficient").is_ok());
        assert!(validar_destino("Iurefficient").is_err());
    }

    /// En esta maquina de desarrollo hay FUSE, asi que la comprobacion tiene que
    /// pasar. Si algun dia falla aqui, es que la deteccion se rompio.
    #[cfg(target_os = "linux")]
    #[test]
    fn en_esta_maquina_se_puede_montar() {
        assert!(
            comprobar().is_ok(),
            "FUSE debería estar disponible en el entorno de desarrollo"
        );
    }
}
