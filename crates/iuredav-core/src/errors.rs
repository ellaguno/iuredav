//! Traductor de errores: de la jerga de rclone a espanol llano.
//!
//! Este modulo es lo que separa "una unidad que falla de forma incomprensible" de
//! "una unidad que explica sus limites". Cuando el usuario arrastra un documento a
//! la papelera, el gestor de archivos ensena un `Error 403` sin contexto; nosotros
//! leemos la misma linea del log y notificamos *por que* no se puede y *donde* si
//! se puede hacer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severidad {
    /// Limite conocido del servidor. No es una averia: es como funciona.
    Limite,
    /// Algo va mal y el usuario deberia actuar.
    Aviso,
    /// La conexion esta rota.
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MensajeAmistoso {
    pub titulo: String,
    pub detalle: String,
    pub severidad: Severidad,
    /// Ruta afectada, si se pudo extraer de la linea de log.
    pub ruta: Option<String>,
}

/// Verbo WebDAV deducido del texto que escribe rclone.
fn verbo_de(linea: &str) -> Option<&'static str> {
    let l = linea.to_ascii_lowercase();
    // El orden importa: "failed to move" contiene "move", pero tambien queremos
    // cazar el texto que rclone usa para renombrar, que es el mismo verbo.
    if l.contains("failed to remove")
        || l.contains("failed to delete")
        || l.contains("couldn't delete")
    {
        Some("DELETE")
    } else if l.contains("failed to mkdir")
        || l.contains("failed to make directory")
        || l.contains("mkcol")
    {
        Some("MKCOL")
    } else if l.contains("failed to move")
        || l.contains("failed to rename")
        || l.contains("dirmove")
    {
        Some("MOVE")
    } else if l.contains("failed to copy") {
        Some("COPY")
    } else if l.contains("failed to set modification time") || l.contains("proppatch") {
        Some("PROPPATCH")
    } else if l.contains("failed to open")
        || l.contains("failed to upload")
        || l.contains("failed to update")
    {
        Some("PUT")
    } else {
        None
    }
}

/// Primer codigo HTTP de tres cifras que aparezca en la linea.
fn status_de(linea: &str) -> Option<u16> {
    let b = linea.as_bytes();
    for i in 0..b.len().saturating_sub(2) {
        if b[i].is_ascii_digit() && b[i + 1].is_ascii_digit() && b[i + 2].is_ascii_digit() {
            // Debe ir precedido y seguido de algo que no sea digito, para no
            // confundirlo con un tamano de fichero o una marca de tiempo.
            let antes_ok = i == 0 || !b[i - 1].is_ascii_digit();
            let despues_ok = i + 3 >= b.len() || !b[i + 3].is_ascii_digit();
            if antes_ok && despues_ok {
                let n: u16 = linea[i..i + 3].parse().ok()?;
                if (100..=599).contains(&n) {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Extrae la ruta del formato de log de rclone: `NIVEL : ruta/al/fichero: mensaje`.
fn ruta_de(linea: &str) -> Option<String> {
    let resto = linea.split(" : ").nth(1)?;
    let ruta = resto.split(": ").next()?.trim();

    if ruta.is_empty() || ruta.len() > 260 {
        return None;
    }
    // Si lleva espacios y ningun separador, no es una ruta: es prosa del mensaje.
    // Ensenarla como si fuera un archivo confundiria mas que ayudar.
    let parece_prosa = ruta.contains(' ') && !ruta.contains('/') && !ruta.contains('\\');
    if parece_prosa {
        return None;
    }
    Some(ruta.to_string())
}

/// Traduce una linea de log de rclone. Devuelve `None` si no es nada que merezca
/// molestar al usuario: la inmensa mayoria de las lineas no lo son.
pub fn traducir(linea: &str) -> Option<MensajeAmistoso> {
    let l = linea.to_ascii_lowercase();
    if !l.contains("error") && !l.contains("failed") && !l.contains("403") && !l.contains("401") {
        return None;
    }

    let ruta = ruta_de(linea);
    let status = status_de(linea);
    let verbo = verbo_de(linea);

    let (titulo, detalle, severidad) = match (verbo, status) {
        (_, Some(401)) | (_, Some(403)) if l.contains("unauthor") || l.contains("credential") => (
            "Tu contrasena de aplicacion ya no sirve",
            "Genera una nueva contrasena iurdav_... en Iurefficient y vuelve a conectar.",
            Severidad::Aviso,
        ),
        (Some("DELETE"), _) => (
            "Los documentos no se eliminan desde la unidad",
            "Este servidor no permite borrar por WebDAV. Elimina el documento desde Iurefficient.",
            Severidad::Limite,
        ),
        (Some("MKCOL"), _) => (
            "Las carpetas no se crean desde la unidad",
            "La estructura de Casos y General se gestiona en Iurefficient.",
            Severidad::Limite,
        ),
        (Some("MOVE"), _) => (
            "No se puede mover ni renombrar",
            "Este servidor no soporta mover archivos. Sube el documento con el nombre definitivo.",
            Severidad::Limite,
        ),
        (Some("COPY"), _) => (
            "No se puede copiar dentro de la unidad",
            "Copia el archivo a tu equipo y vuelve a subirlo en la carpeta de destino.",
            Severidad::Limite,
        ),
        (Some("PROPPATCH"), _) => (
            "La fecha del archivo no se conserva",
            "El servidor no permite fijar la fecha de modificacion. No afecta al contenido.",
            Severidad::Limite,
        ),
        (_, Some(423)) => (
            "El archivo esta en uso",
            "Otra persona o programa lo tiene abierto. Vuelve a intentarlo en unos segundos.",
            Severidad::Aviso,
        ),
        (_, Some(401)) => (
            "Tu contrasena de aplicacion ya no sirve",
            "Genera una nueva contrasena iurdav_... en Iurefficient y vuelve a conectar.",
            Severidad::Aviso,
        ),
        (_, Some(507)) => (
            "No hay espacio en el servidor",
            "El documento no se pudo guardar. Contacta con el administrador de tu instancia.",
            Severidad::Error,
        ),
        (Some("PUT"), _) => (
            "No se pudo guardar el documento",
            "El cambio sigue en la cache local y se reintentara. No cierres la aplicacion.",
            Severidad::Aviso,
        ),
        (_, Some(s)) if (500..=599).contains(&s) => (
            "El servidor no responde bien",
            "Puede ser una interrupcion temporal. La unidad reintentara automaticamente.",
            Severidad::Error,
        ),
        _ => return None,
    };

    Some(MensajeAmistoso {
        titulo: titulo.to_string(),
        detalle: detalle.to_string(),
        severidad,
        ruta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lineas con la forma exacta que escribe rclone al chocar con los limites de
    /// este servidor.
    #[test]
    fn traduce_el_403_al_borrar() {
        let l = r#"2026/09/09 19:10:00 ERROR : General/contrato.docx: Failed to remove: Delete "https://x/webdav/General/contrato.docx": 403 Forbidden"#;
        let m = traducir(l).expect("deberia traducirse");
        assert_eq!(m.severidad, Severidad::Limite);
        assert!(m.titulo.contains("no se eliminan"));
        assert_eq!(m.ruta.as_deref(), Some("General/contrato.docx"));
    }

    #[test]
    fn traduce_el_502_al_mover() {
        let l = "2026/09/09 19:10:00 ERROR : Casos/A/x.pdf: Failed to move: 502 Bad Gateway";
        let m = traducir(l).expect("deberia traducirse");
        assert!(m.titulo.contains("mover"));
        assert_eq!(m.severidad, Severidad::Limite);
    }

    #[test]
    fn traduce_el_403_al_crear_carpeta() {
        let l = "2026/09/09 19:10:00 ERROR : General/Nueva: Failed to mkdir: 403 Forbidden";
        let m = traducir(l).expect("deberia traducirse");
        assert!(m.titulo.contains("carpetas"));
    }

    #[test]
    fn el_401_pide_una_contrasena_nueva() {
        let l = "2026/09/09 19:10:00 ERROR : couldn't list files: 401 Unauthorized";
        let m = traducir(l).expect("deberia traducirse");
        assert_eq!(m.severidad, Severidad::Aviso);
        assert!(m.detalle.contains("iurdav_"));
    }

    /// Linea capturada tal cual de rclone 1.60 al chocar contra el servidor falso.
    /// Es la forma real del fallo que motiva todo el diseno, asi que el traductor
    /// tiene que reconocerla sin retoques.
    #[test]
    fn traduce_el_fallo_real_de_dirmove() {
        let l = "2026/09/09 19:26:20 ERROR : webdav root 'General/demo2.txt': Server side directory move failed: DirMove MOVE call failed: puerta de enlace incorrecta: 502 Bad Gateway";
        let m = traducir(l).expect("deberia traducirse");
        assert!(
            m.titulo.contains("mover"),
            "titulo inesperado: {}",
            m.titulo
        );
        assert_eq!(m.severidad, Severidad::Limite);
    }

    #[test]
    fn el_ruido_normal_no_molesta_al_usuario() {
        assert!(traducir("2026/09/09 19:10:00 INFO  : General/x.docx: Copied (new)").is_none());
        assert!(traducir("2026/09/09 19:10:00 DEBUG : vfs cache: cleaned").is_none());
        assert!(traducir("Transferred: 12.4 MiB / 12.4 MiB, 100%").is_none());
    }

    #[test]
    fn no_confunde_un_tamano_con_un_codigo_http() {
        // 4096 no debe leerse como "409".
        assert_eq!(status_de("size 4096 bytes"), None);
        assert_eq!(status_de("returned 403 Forbidden"), Some(403));
    }
}
