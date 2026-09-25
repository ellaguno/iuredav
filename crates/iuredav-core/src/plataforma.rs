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

use iurefficient_connect::lang::pick;
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
                por_que: pick(
                    "It's what lets Windows show your documents as a disk drive.",
                    "Es lo que permite a Windows mostrar tus documentos como una unidad de disco.",
                )
                .into(),
                como_instalar: pick(
                    "Download and install WinFsp; IureDav will detect it on its own when you come back to this window.",
                    "Descarga e instala WinFsp; IureDav lo detectará solo al volver a esta ventana.",
                )
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
                por_que: pick(
                    "It's what lets your documents show up as a system folder.",
                    "Es lo que permite mostrar tus documentos como una carpeta del sistema.",
                )
                .into(),
                como_instalar: pick(
                    "sudo apt install fuse3   (or your distribution's equivalent)",
                    "sudo apt install fuse3   (o el equivalente de tu distribución)",
                )
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

// ---------------------------------------------------------------------------
// Montajes huerfanos
//
// Si el proceso que servia un montaje muere sin desmontar —un cierre de sesion,
// un kill, un cuelgue, cerrar la aplicacion a la fuerza— la entrada se queda en
// la tabla del sistema sin nadie detras. Cualquier acceso a esa ruta devuelve
// ENOTCONN, y el usuario se encuentra con que su carpeta ya no se puede abrir
// **ni volver a montar**.
//
// Lo insidioso es como se manifiesta: `Path::exists()` hace un `stat`, el `stat`
// falla, y Rust responde `false`. Quien pregunte "existe esta carpeta?" recibe un
// "no" y concluye que hay que crearla; el `mkdir` choca con el montaje fantasma y
// el error que llega al usuario es «no se pudo crear», que es justo el
// diagnostico equivocado. Por eso esto se comprueba aparte y antes.
// ---------------------------------------------------------------------------

/// Si en `p` hay un montaje que quedo huerfano.
#[cfg(unix)]
pub fn montaje_muerto(p: &Path) -> bool {
    match std::fs::metadata(p) {
        Ok(_) => false,
        Err(e) => {
            // `NotConnected` es como std traduce ENOTCONN; se mira tambien el
            // codigo crudo por si esa correspondencia cambiara.
            #[cfg(target_os = "macos")]
            const ENOTCONN: i32 = 57;
            #[cfg(not(target_os = "macos"))]
            const ENOTCONN: i32 = 107;

            e.kind() == std::io::ErrorKind::NotConnected || e.raw_os_error() == Some(ENOTCONN)
        }
    }
}

/// En Windows el montaje es una letra de unidad: si el proceso muere, la unidad
/// se va con el y no queda nada huerfano que soltar.
#[cfg(windows)]
pub fn montaje_muerto(_p: &Path) -> bool {
    false
}

/// Suelta un montaje huerfano.
///
/// Solo actua si [`montaje_muerto`] lo confirma: la comprobacion va **dentro** a
/// proposito, para que esto no pueda desmontar por error algo que si esta vivo.
#[cfg(unix)]
pub fn soltar_montaje_muerto(p: &Path) -> Result<(), String> {
    if !montaje_muerto(p) {
        return Err(pick(
            "there's no stale mount there",
            "ahi no hay ningun montaje sin cerrar",
        )
        .into());
    }

    // En diferido (`-z` / `-l`): basta con que una terminal tenga su directorio
    // de trabajo dentro para que el desmontaje normal de "dispositivo ocupado",
    // y es un montaje muerto —no hay nada que perder soltandolo ya y que el
    // nucleo limpie cuando se suelte la ultima referencia.
    let intentos: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("umount", &["-f"]), ("diskutil", &["unmount", "force"])]
    } else {
        &[
            ("fusermount3", &["-uz"]),
            ("fusermount", &["-uz"]),
            ("umount", &["-l"]),
        ]
    };

    let mut ultimo = String::from(pick(
        "no unmount command could be run",
        "no se pudo ejecutar ninguna orden de desmontaje",
    ));
    for (orden, args) in intentos {
        match std::process::Command::new(orden)
            .args(*args)
            .arg(p)
            .output()
        {
            Err(e) => ultimo = format!("{orden}: {e}"),
            Ok(salida) if salida.status.success() => return Ok(()),
            Ok(salida) => {
                ultimo = String::from_utf8_lossy(&salida.stderr).trim().to_string();
            }
        }
    }
    Err(ultimo)
}

#[cfg(windows)]
pub fn soltar_montaje_muerto(_p: &Path) -> Result<(), String> {
    Err(pick(
        "there's no stale mount there",
        "ahi no hay ningun montaje sin cerrar",
    )
    .into())
}

/// Como se llama el sitio donde aparecen los archivos, en cada plataforma. La UI
/// lo usa para no hablar de "punto de montaje", que no significa nada fuera de Unix.
pub fn nombre_del_destino() -> &'static str {
    if cfg!(target_os = "windows") {
        pick("Drive", "Unidad")
    } else {
        pick("Folder", "Carpeta")
    }
}

/// Comprueba que el destino tenga la forma que espera la plataforma.
pub fn validar_destino(destino: &str) -> Result<(), String> {
    if destino.trim().is_empty() {
        return Err(pick(
            "Choose where you want your documents to appear.",
            "Indica donde quieres que aparezcan tus documentos.",
        )
        .into());
    }
    if cfg!(target_os = "windows") {
        // En Windows lo normal es una letra de unidad. Tambien vale una carpeta
        // vacia, pero la letra es lo que la gente espera ver en el Explorador.
        let d = destino.trim();
        let letra =
            d.len() == 2 && d.ends_with(':') && d.chars().next().unwrap().is_ascii_alphabetic();
        if !letra && !d.contains('\\') {
            return Err(pick(
                "Use a drive letter such as I: or a full folder path.",
                "Usa una letra de unidad como I: o una ruta de carpeta completa.",
            )
            .into());
        }
    } else if !destino.starts_with('/') && !destino.starts_with('~') {
        return Err(pick(
            "Use a full path, starting with /.",
            "Usa una ruta completa, que empiece por /.",
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La salvaguarda que impide que esto desmonte algo vivo: soltar solo actua
    /// si la ruta esta de verdad muerta, y la comprobacion va dentro de la
    /// funcion para que no dependa de que cada llamante se acuerde.
    #[test]
    fn no_se_suelta_lo_que_no_esta_muerto() {
        let d = std::env::temp_dir().join(format!("iuredav-vivo-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();

        assert!(!montaje_muerto(&d), "una carpeta normal no esta muerta");
        assert!(soltar_montaje_muerto(&d).is_err(), "no puede desmontarla");
        assert!(d.is_dir(), "y la carpeta sigue ahi");

        // Lo que no existe tampoco es un montaje muerto: eso si es "no existe".
        assert!(!montaje_muerto(&d.join("no-existe")));

        let _ = std::fs::remove_dir_all(&d);
    }

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

// ---------------------------------------------------------------------------
// Montajes vivos: los que siguen en la tabla del sistema con alguien detras.
//
// El caso tipico: se actualiza IureDav instalando el paquete a mano mientras la
// version anterior sigue abierta en la bandeja con su rclone y su unidad. La
// instancia nueva no sabe nada de ese montaje (la tabla `montados` vive en
// memoria) y, al ver la carpeta con contenido, se negaba a montar con un mensaje
// que apuntaba al sitio equivocado. Ahora reconoce el montaje como suyo.
// ---------------------------------------------------------------------------

/// Que hay montado en `p` segun el sistema: `(origen, tipo)`. `None` si nada.
#[cfg(target_os = "linux")]
pub fn montaje_en(p: &Path) -> Option<(String, String)> {
    let tabla = std::fs::read_to_string("/proc/self/mounts").ok()?;
    let objetivo = p.to_string_lossy();
    for linea in tabla.lines() {
        let mut campos = linea.split_whitespace();
        let (Some(origen), Some(punto), Some(tipo)) = (campos.next(), campos.next(), campos.next())
        else {
            continue;
        };
        // /proc/mounts escapa espacios y otros caracteres en octal.
        let punto = punto
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\");
        if punto == objetivo {
            return Some((origen.to_string(), tipo.to_string()));
        }
    }
    None
}

/// En macOS la tabla la da `mount`: `origen on /ruta (tipo, opciones)`.
#[cfg(target_os = "macos")]
pub fn montaje_en(p: &Path) -> Option<(String, String)> {
    let salida = std::process::Command::new("mount").output().ok()?;
    let texto = String::from_utf8_lossy(&salida.stdout);
    let objetivo = format!(" on {} (", p.to_string_lossy());
    for linea in texto.lines() {
        if let Some(i) = linea.find(&objetivo) {
            let origen = linea[..i].to_string();
            let resto = &linea[i + objetivo.len()..];
            let tipo = resto
                .split(|c| c == ',' || c == ')')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            return Some((origen, tipo));
        }
    }
    None
}

/// En Windows la unidad es una letra ligada al proceso: no hay tabla que mirar.
#[cfg(windows)]
pub fn montaje_en(_p: &Path) -> Option<(String, String)> {
    None
}

/// Si lo que hay montado en `p` es la unidad de IureDav para la conexion `id`
/// (rclone monta con origen `<remoto>:`, y el remoto se llama como la conexion).
pub fn montaje_propio(id: &str, p: &Path) -> bool {
    match montaje_en(p) {
        Some((origen, tipo)) => {
            origen == format!("{id}:") && (tipo.contains("fuse") || tipo.contains("rclone"))
        }
        None => false,
    }
}

/// Desmonta un montaje vivo que no controla nuestro rclone (por ejemplo, el de
/// una IureDav anterior). Primero de forma normal; si esta ocupado, en diferido.
#[cfg(unix)]
pub fn soltar_montaje(p: &Path) -> Result<(), String> {
    let intentos: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[
            ("umount", &[]),
            ("diskutil", &["unmount"]),
            ("umount", &["-f"]),
            ("diskutil", &["unmount", "force"]),
        ]
    } else {
        &[
            ("fusermount3", &["-u"]),
            ("fusermount", &["-u"]),
            ("umount", &[]),
            ("fusermount3", &["-uz"]),
            ("fusermount", &["-uz"]),
            ("umount", &["-l"]),
        ]
    };
    let mut ultimo = String::from(pick(
        "no unmount command could be run",
        "no se pudo ejecutar ninguna orden de desmontaje",
    ));
    for (orden, args) in intentos {
        match std::process::Command::new(orden)
            .args(*args)
            .arg(p)
            .output()
        {
            Err(e) => ultimo = format!("{orden}: {e}"),
            Ok(salida) if salida.status.success() => return Ok(()),
            Ok(salida) => ultimo = String::from_utf8_lossy(&salida.stderr).trim().to_string(),
        }
    }
    Err(ultimo)
}

#[cfg(windows)]
pub fn soltar_montaje(_p: &Path) -> Result<(), String> {
    Err(pick(
        "on Windows the drive closes with the process that opened it",
        "en Windows la unidad se cierra con el proceso que la abrio",
    )
    .into())
}

#[cfg(all(test, target_os = "linux"))]
mod tests_montajes {
    use super::*;

    #[test]
    fn la_raiz_esta_en_la_tabla_y_una_ruta_inventada_no() {
        assert!(montaje_en(Path::new("/")).is_some());
        assert!(montaje_en(Path::new("/no/existe/iuredav-test")).is_none());
        assert!(!montaje_propio("x", Path::new("/")));
    }
}
