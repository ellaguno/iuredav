//! Carpetas disponibles sin conexion.
//!
//! rclone **no tiene un "anclar" nativo**. Lo que si tiene es una cache de VFS con
//! caducidad y tamano maximo. Anclar una carpeta consiste en recorrerla y leer cada
//! archivo a traves del punto de montaje: eso obliga al VFS a bajarlos y dejarlos
//! en la cache, donde siguen disponibles cuando no hay red.
//!
//! El limite, que la interfaz dice sin adornos: si la cache se llena, el desalojo
//! por antiguedad **puede** expulsar contenido anclado. No es una garantia, es una
//! aproximacion buena. Se compensa recalentando al montar y dimensionando la cache
//! con holgura.
//!
//! Se descarta el camino alternativo —sincronizar a una carpeta local de verdad—
//! precisamente por lo que midio la sonda: sin huella de contenido y sin fecha
//! fiable, la comparacion caeria a mirar solo el tamano, que no detecta un archivo
//! editado que pesa igual.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Cuanto se lee de cada archivo. Leerlo entero es lo que lo mete en la cache.
const TROZO: usize = 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Resumen {
    pub archivos: usize,
    pub bytes: u64,
    /// Archivos que no se pudieron leer. No aborta: un documento bloqueado por
    /// otro programa no debe impedir que el resto quede disponible.
    pub fallidos: usize,
}

/// Comprueba que `relativa` no se salga del punto de montaje.
///
/// La ruta viene de un selector de carpetas, asi que en la practica es de fiar;
/// pero un `..` mal puesto haria que la aplicacion se pusiera a leer el disco
/// entero del usuario, y eso no puede depender de la buena fe de la entrada.
pub fn resolver(punto: &Path, relativa: &str) -> Result<PathBuf> {
    let destino = punto.join(relativa.trim_start_matches(['/', '\\']));

    let punto_real = punto
        .canonicalize()
        .with_context(|| format!("no se pudo resolver {}", punto.display()))?;
    let destino_real = destino
        .canonicalize()
        .with_context(|| format!("no existe {}", destino.display()))?;

    if !destino_real.starts_with(&punto_real) {
        bail!("{relativa} queda fuera de la unidad montada");
    }
    Ok(destino_real)
}

/// Lee todo lo que hay bajo `carpeta` para que quede en la cache local.
///
/// `avance` se llama con (archivos leidos, bytes leidos) para que la interfaz
/// pueda mostrar progreso: en una carpeta grande esto tarda minutos.
pub fn calentar(punto: &Path, relativa: &str, mut avance: impl FnMut(&Resumen)) -> Result<Resumen> {
    let raiz = resolver(punto, relativa)?;
    let mut r = Resumen::default();
    let mut pendientes = vec![raiz];

    while let Some(dir) = pendientes.pop() {
        let entradas = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                debug!(?dir, %e, "no se pudo listar");
                r.fallidos += 1;
                continue;
            }
        };

        for entrada in entradas.flatten() {
            let ruta = entrada.path();
            match entrada.file_type() {
                Ok(t) if t.is_dir() => pendientes.push(ruta),
                Ok(t) if t.is_file() => {
                    match leer_entero(&ruta) {
                        Ok(n) => {
                            r.archivos += 1;
                            r.bytes += n;
                        }
                        Err(e) => {
                            // Un archivo en uso o sin permiso no debe frustrar el resto.
                            debug!(?ruta, %e, "no se pudo precargar");
                            r.fallidos += 1;
                        }
                    }
                    avance(&r);
                }
                _ => {}
            }
        }
    }
    Ok(r)
}

/// Lee el archivo entero y descarta el contenido: el efecto que buscamos es que el
/// VFS de rclone lo baje y lo deje en su cache.
fn leer_entero(ruta: &Path) -> Result<u64> {
    let mut f = std::fs::File::open(ruta)?;
    let mut buf = vec![0u8; TROZO];
    let mut total = 0u64;
    loop {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => total += n as u64,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(total)
}

/// Espera a que el punto de montaje responda antes de calentar nada.
pub fn esperar_montaje(punto: &Path, limite: Duration) -> bool {
    let hasta = std::time::Instant::now() + limite;
    while std::time::Instant::now() < hasta {
        if std::fs::read_dir(punto).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporal(nombre: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("iuredav-anclaje-{}-{nombre}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn lee_todo_el_arbol_y_suma_los_bytes() {
        let punto = temporal("arbol");
        std::fs::create_dir_all(punto.join("Casos/Expediente 1")).unwrap();
        std::fs::write(punto.join("Casos/a.txt"), b"12345").unwrap();
        std::fs::write(punto.join("Casos/Expediente 1/b.txt"), b"1234567890").unwrap();

        let r = calentar(&punto, "Casos", |_| {}).unwrap();
        assert_eq!(r.archivos, 2);
        assert_eq!(r.bytes, 15);
        assert_eq!(r.fallidos, 0);

        let _ = std::fs::remove_dir_all(&punto);
    }

    /// Un `..` en la ruta no puede sacarnos de la unidad: si lo hiciera, anclar una
    /// carpeta pondria a la aplicacion a leer el disco entero del usuario.
    #[test]
    fn no_se_puede_salir_del_punto_de_montaje() {
        let punto = temporal("fuga");
        std::fs::create_dir_all(punto.join("dentro")).unwrap();

        assert!(resolver(&punto, "dentro").is_ok());
        let e = resolver(&punto, "../..").unwrap_err().to_string();
        assert!(e.contains("fuera de la unidad"), "{e}");

        let _ = std::fs::remove_dir_all(&punto);
    }

    #[test]
    fn un_archivo_ilegible_no_frustra_el_resto() {
        let punto = temporal("parcial");
        std::fs::create_dir_all(punto.join("c")).unwrap();
        std::fs::write(punto.join("c/bueno.txt"), b"hola").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let malo = punto.join("c/prohibido.txt");
            std::fs::write(&malo, b"x").unwrap();
            std::fs::set_permissions(&malo, std::fs::Permissions::from_mode(0o000)).unwrap();

            let r = calentar(&punto, "c", |_| {}).unwrap();
            assert_eq!(r.archivos, 1, "el bueno tiene que haberse leido");
            assert_eq!(r.fallidos, 1);

            std::fs::set_permissions(&malo, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        let _ = std::fs::remove_dir_all(&punto);
    }
}
