//! Traductor de errores: de la jerga de rclone a lenguaje llano (ingles o espanol).
//!
//! Este modulo es lo que separa "una unidad que falla de forma incomprensible" de
//! "una unidad que explica sus límites". Cuando el usuario arrastra un documento a
//! la papelera, el gestor de archivos ensena un `Error 403` sin contexto; nosotros
//! leemos la misma linea del log y notificamos *por que* no se puede y *donde* si
//! se puede hacer.

use iurefficient_connect::lang::{self, Lang};
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

/// Como llamar al sitio donde el usuario si puede hacer lo que la unidad le niega.
///
/// Contra Iurefficient se puede nombrar; contra un WebDAV cualquiera no sabemos
/// como se llama su aplicacion web, y decir un nombre inventado seria peor que
/// hablar en generico.
pub fn donde(gestionar: Option<&str>) -> String {
    donde_en(gestionar, lang::current())
}

fn donde_en(gestionar: Option<&str>, idioma: Lang) -> String {
    match gestionar {
        Some(n) => tr_en!(idioma, "in {n}", "desde {n}"),
        None => tr_en!(
            idioma,
            "in your server's web app",
            "desde la aplicación web de tu servidor"
        ),
    }
}

/// Traduce una linea de log de rclone. Devuelve `None` si no es nada que merezca
/// molestar al usuario: la inmensa mayoria de las lineas no lo son.
///
/// `gestionar` es el nombre de la aplicacion web del servidor, si se conoce. El
/// mensaje sale en el idioma actual de la interfaz.
pub fn traducir(linea: &str, gestionar: Option<&str>) -> Option<MensajeAmistoso> {
    traducir_en(linea, gestionar, lang::current())
}

/// [`traducir`] con el idioma explicito: el global lo comparten todas las pruebas.
fn traducir_en(linea: &str, gestionar: Option<&str>, idioma: Lang) -> Option<MensajeAmistoso> {
    let l = linea.to_ascii_lowercase();
    if !l.contains("error") && !l.contains("failed") && !l.contains("403") && !l.contains("401") {
        return None;
    }

    let ruta = ruta_de(linea);
    let status = status_de(linea);
    let verbo = verbo_de(linea);

    let sitio = donde_en(gestionar, idioma);
    let cred_nueva = match gestionar {
        Some(n) if n == "Iurefficient" => tr_en!(
            idioma,
            "Generate a new iurdav_… password in {n} and connect again.",
            "Genera una nueva contraseña iurdav_… en {n} y vuelve a conectar."
        ),
        Some(n) => tr_en!(
            idioma,
            "Generate a new password in {n} and connect again.",
            "Genera una contraseña nueva en {n} y vuelve a conectar."
        ),
        None => tr_en!(
            idioma,
            "Generate a new password on your server and connect again.",
            "Genera una contraseña nueva en tu servidor y vuelve a conectar."
        ),
    };
    let t = |en: &'static str, es: &'static str| -> String {
        match idioma {
            Lang::En => en.to_string(),
            Lang::Es => es.to_string(),
        }
    };

    let (titulo, detalle, severidad): (String, String, Severidad) = match (verbo, status) {
        (_, Some(401)) | (_, Some(403)) if l.contains("unauthor") || l.contains("credential") => (
            t(
                "Your app password no longer works",
                "Tu contraseña de aplicación ya no sirve",
            ),
            cred_nueva,
            Severidad::Aviso,
        ),
        (Some("DELETE"), _) => (
            t(
                "Documents can't be deleted from the drive",
                "Los documentos no se eliminan desde la unidad",
            ),
            tr_en!(
                idioma,
                "This server doesn't allow deleting over WebDAV. Delete the document {sitio}.",
                "Este servidor no permite borrar por WebDAV. Elimina el documento {sitio}."
            ),
            Severidad::Limite,
        ),
        (Some("MKCOL"), _) => (
            t(
                "Folders can't be created from the drive",
                "Las carpetas no se crean desde la unidad",
            ),
            tr_en!(
                idioma,
                "The folder structure is managed {sitio}.",
                "La estructura de carpetas se gestiona {sitio}."
            ),
            Severidad::Limite,
        ),
        (Some("MOVE"), _) => (
            t("Can't move or rename", "No se puede mover ni renombrar"),
            t(
                "This server doesn't support moving files. Upload the document with its final name.",
                "Este servidor no soporta mover archivos. Sube el documento con el nombre definitivo.",
            ),
            Severidad::Limite,
        ),
        (Some("COPY"), _) => (
            t(
                "Can't copy within the drive",
                "No se puede copiar dentro de la unidad",
            ),
            t(
                "Copy the file to your computer and upload it again into the destination folder.",
                "Copia el archivo a tu equipo y vuelve a subirlo en la carpeta de destino.",
            ),
            Severidad::Limite,
        ),
        (Some("PROPPATCH"), _) => (
            t(
                "The file date isn't preserved",
                "La fecha del archivo no se conserva",
            ),
            t(
                "The server doesn't allow setting the modification date. The content isn't affected.",
                "El servidor no permite fijar la fecha de modificación. No afecta al contenido.",
            ),
            Severidad::Limite,
        ),
        (_, Some(423)) => (
            t("The file is in use", "El archivo esta en uso"),
            t(
                "Another person or program has it open. Try again in a few seconds.",
                "Otra persona o programa lo tiene abierto. Vuelve a intentarlo en unos segundos.",
            ),
            Severidad::Aviso,
        ),
        (_, Some(401)) => (
            t(
                "Your app password no longer works",
                "Tu contraseña de aplicación ya no sirve",
            ),
            cred_nueva,
            Severidad::Aviso,
        ),
        (_, Some(507)) => (
            t("The server is out of space", "No hay espacio en el servidor"),
            t(
                "The document couldn't be saved. Contact your instance's administrator.",
                "El documento no se pudo guardar. Contacta con el administrador de tu instancia.",
            ),
            Severidad::Error,
        ),
        (Some("PUT"), _) => (
            t(
                "The document couldn't be saved",
                "No se pudo guardar el documento",
            ),
            t(
                "The change is still in the local cache and will be retried. Don't close the app.",
                "El cambio sigue en la caché local y se reintentará. No cierres la aplicación.",
            ),
            Severidad::Aviso,
        ),
        (_, Some(s)) if (500..=599).contains(&s) => (
            t(
                "The server isn't responding properly",
                "El servidor no responde bien",
            ),
            t(
                "It may be a temporary outage. The drive will retry automatically.",
                "Puede ser una interrupcion temporal. La unidad reintentará automáticamente.",
            ),
            Severidad::Error,
        ),
        _ => return None,
    };

    Some(MensajeAmistoso {
        titulo,
        detalle,
        severidad,
        ruta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ES: Lang = Lang::Es;

    /// Lineas con la forma exacta que escribe rclone al chocar con los limites de
    /// este servidor.
    #[test]
    fn traduce_el_403_al_borrar() {
        let l = r#"2026/09/09 19:10:00 ERROR : General/contrato.docx: Failed to remove: Delete "https://x/webdav/General/contrato.docx": 403 Forbidden"#;
        let m = traducir_en(l, Some("Iurefficient"), ES).expect("debería traducirse");
        assert_eq!(m.severidad, Severidad::Limite);
        assert!(m.titulo.contains("no se eliminan"));
        assert_eq!(m.ruta.as_deref(), Some("General/contrato.docx"));
    }

    #[test]
    fn traduce_el_502_al_mover() {
        let l = "2026/09/09 19:10:00 ERROR : Casos/A/x.pdf: Failed to move: 502 Bad Gateway";
        let m = traducir_en(l, Some("Iurefficient"), ES).expect("debería traducirse");
        assert!(m.titulo.contains("mover"));
        assert_eq!(m.severidad, Severidad::Limite);
    }

    #[test]
    fn traduce_el_403_al_crear_carpeta() {
        let l = "2026/09/09 19:10:00 ERROR : General/Nueva: Failed to mkdir: 403 Forbidden";
        let m = traducir_en(l, Some("Iurefficient"), ES).expect("debería traducirse");
        assert!(m.titulo.contains("carpetas"));
    }

    #[test]
    fn el_401_pide_una_contrasena_nueva() {
        let l = "2026/09/09 19:10:00 ERROR : couldn't list files: 401 Unauthorized";
        let m = traducir_en(l, Some("Iurefficient"), ES).expect("debería traducirse");
        assert_eq!(m.severidad, Severidad::Aviso);
        assert!(m.detalle.contains("iurdav_"));
    }

    /// Linea capturada tal cual de rclone 1.60 al chocar contra el servidor falso.
    /// Es la forma real del fallo que motiva todo el diseno, asi que el traductor
    /// tiene que reconocerla sin retoques.
    #[test]
    fn traduce_el_fallo_real_de_dirmove() {
        let l = "2026/09/09 19:26:20 ERROR : webdav root 'General/demo2.txt': Server side directory move failed: DirMove MOVE call failed: puerta de enlace incorrecta: 502 Bad Gateway";
        let m = traducir_en(l, Some("Iurefficient"), ES).expect("debería traducirse");
        assert!(
            m.titulo.contains("mover"),
            "titulo inesperado: {}",
            m.titulo
        );
        assert_eq!(m.severidad, Severidad::Limite);
    }

    #[test]
    fn el_ruido_normal_no_molesta_al_usuario() {
        assert!(traducir_en(
            "2026/09/09 19:10:00 INFO  : General/x.docx: Copied (new)",
            None,
            ES
        )
        .is_none());
        assert!(traducir_en("2026/09/09 19:10:00 DEBUG : vfs caché: cleaned", None, ES).is_none());
        assert!(traducir_en("Transferred: 12.4 MiB / 12.4 MiB, 100%", None, ES).is_none());
    }

    /// Contra un WebDAV cualquiera no sabemos como se llama su aplicacion web, asi
    /// que nombrar Iurefficient seria desconcertante.
    #[test]
    fn en_un_servidor_ajeno_no_se_nombra_iurefficient() {
        let l = r#"2026/09/09 19:10:00 ERROR : doc.pdf: Failed to remove: 403 Forbidden"#;
        let generico = traducir_en(l, None, ES).unwrap();
        assert!(
            !generico.detalle.contains("Iurefficient"),
            "{}",
            generico.detalle
        );
        assert!(
            generico.detalle.contains("tu servidor"),
            "{}",
            generico.detalle
        );

        let propio = traducir_en(l, Some("Iurefficient"), ES).unwrap();
        assert!(
            propio.detalle.contains("Iurefficient"),
            "{}",
            propio.detalle
        );
    }

    #[test]
    fn no_confunde_un_tamano_con_un_codigo_http() {
        // 4096 no debe leerse como "409".
        assert_eq!(status_de("size 4096 bytes"), None);
        assert_eq!(status_de("returned 403 Forbidden"), Some(403));
    }

    /// Por defecto (y con el sistema en otro idioma) los avisos salen en ingles.
    #[test]
    fn en_ingles_se_traduce_igual() {
        let l = r#"2026/09/09 19:10:00 ERROR : doc.pdf: Failed to remove: 403 Forbidden"#;
        let m = traducir_en(l, Some("Iurefficient"), Lang::En).unwrap();
        assert_eq!(m.severidad, Severidad::Limite);
        assert!(m.titulo.contains("can't be deleted"), "{}", m.titulo);
        assert!(m.detalle.contains("in Iurefficient"), "{}", m.detalle);
        let generico = traducir_en(l, None, Lang::En).unwrap();
        assert!(
            generico.detalle.contains("your server"),
            "{}",
            generico.detalle
        );
    }
}
