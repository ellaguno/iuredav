//! Perfiles de servidor: lo poco que es especifico de cada tipo de instalacion.
//!
//! El nucleo habla WebDAV puro y deduce todo de lo que la sonda mide, asi que casi
//! nada depende del fabricante. Lo que si depende cabe aqui: donde escribir el
//! fichero de diagnostico, en que carpeta buscar una muestra para probar la lectura
//! por trozos, y como llamar al sitio donde el usuario debe ir cuando la unidad no
//! le deja hacer algo.
//!
//! Tener esto separado permite que la misma aplicacion sirva para un WebDAV
//! cualquiera sin que el codigo se llene de casos particulares.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preset {
    pub id: String,
    pub nombre: String,
    pub descripcion: String,
    /// Sufijo que se anade si el usuario escribe solo el dominio.
    pub sufijo_url: Option<String>,
    /// Donde escribe la sonda de escritura. Fija a proposito: si el servidor no
    /// permite borrar, reutilizarla evita acumular ficheros huerfanos.
    pub ruta_selftest: String,
    /// Carpeta donde buscar un fichero con el que probar GET y Range. Se elige una
    /// comun a proposito, para no tocar documentos de clientes.
    pub carpeta_muestra: Option<String>,
    pub nombre_volumen: String,
    /// Como se llama el sitio al que mandar al usuario cuando la unidad no permite
    /// algo. `None` si no lo sabemos: entonces se habla en generico.
    pub donde_gestionar: Option<String>,
    /// Pista sobre la contrasena, para el formulario.
    pub pista_password: String,
}

impl Preset {
    /// Instalaciones de Iurefficient.
    pub fn iurefficient() -> Self {
        Self {
            id: "iurefficient".into(),
            nombre: "Iurefficient".into(),
            descripcion: "Tus casos y documentos de una instancia de Iurefficient.".into(),
            sufijo_url: Some("webdav/".into()),
            ruta_selftest: "General/.iuredav-selftest.txt".into(),
            carpeta_muestra: Some("General/".into()),
            nombre_volumen: "Iurefficient".into(),
            donde_gestionar: Some("Iurefficient".into()),
            pista_password: "Una contraseña de aplicación: empieza por iurdav_ y la generas \
                             —y puedes revocarla— desde tu perfil. No es la contraseña con la \
                             que entras a Iurefficient."
                .into(),
        }
    }

    /// Cualquier otro servidor WebDAV sobre HTTPS con autenticacion basica:
    /// Nextcloud, ownCloud, Synology, Seafile, un mod_dav de Apache o nginx.
    pub fn generico() -> Self {
        Self {
            id: "generico".into(),
            nombre: "Otro servidor WebDAV".into(),
            descripcion: "Nextcloud, ownCloud, Synology, Seafile o cualquier WebDAV sobre HTTPS."
                .into(),
            sufijo_url: None,
            ruta_selftest: ".iuredav-selftest.txt".into(),
            // Sin carpeta conocida: se busca en la raiz.
            carpeta_muestra: None,
            nombre_volumen: "WebDAV".into(),
            donde_gestionar: None,
            pista_password: "Tu contraseña, o mejor una contraseña de aplicación si tu servidor \
                             las ofrece. Se guarda en el llavero de tu sistema."
                .into(),
        }
    }

    pub fn todos() -> Vec<Preset> {
        vec![Self::iurefficient(), Self::generico()]
    }

    pub fn por_id(id: &str) -> Preset {
        Self::todos()
            .into_iter()
            .find(|p| p.id == id)
            // Un id desconocido cae en el generico, que no asume nada.
            .unwrap_or_else(Self::generico)
    }

    /// Carpeta donde buscar la muestra de lectura; la raiz si no hay una conocida.
    pub fn carpeta_muestra(&self) -> &str {
        self.carpeta_muestra.as_deref().unwrap_or("")
    }

    /// Completa la URL si el usuario escribio solo el dominio.
    pub fn normalizar_url(&self, entrada: &str) -> String {
        let mut u = entrada.trim().to_string();
        if u.is_empty() {
            return u;
        }
        if !u.contains("://") {
            u = format!("https://{u}");
        }
        if !u.ends_with('/') {
            u.push('/');
        }
        // Solo se anade el sufijo si el usuario no lo escribio ya.
        if let Some(sufijo) = &self.sufijo_url {
            let base = sufijo.trim_end_matches('/');
            if !u.trim_end_matches('/').ends_with(base) {
                u.push_str(sufijo);
            }
        }
        u
    }
}

impl Default for Preset {
    fn default() -> Self {
        Self::iurefficient()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completa_la_url_de_iurefficient() {
        let p = Preset::iurefficient();
        assert_eq!(
            p.normalizar_url("2.ds.ejemplo.com"),
            "https://2.ds.ejemplo.com/webdav/"
        );
        assert_eq!(
            p.normalizar_url("https://x.ejemplo.com"),
            "https://x.ejemplo.com/webdav/"
        );
    }

    /// Si el usuario ya escribio /webdav/, no se puede duplicar.
    #[test]
    fn no_duplica_el_sufijo() {
        let p = Preset::iurefficient();
        assert_eq!(
            p.normalizar_url("https://x.ejemplo.com/webdav"),
            "https://x.ejemplo.com/webdav/"
        );
        assert_eq!(
            p.normalizar_url("https://x.ejemplo.com/webdav/"),
            "https://x.ejemplo.com/webdav/"
        );
    }

    /// En un servidor ajeno no se puede suponer la ruta: se usa la que den.
    #[test]
    fn el_generico_no_inventa_rutas() {
        let p = Preset::generico();
        assert_eq!(
            p.normalizar_url("https://nube.ejemplo.com/remote.php/dav/files/ana"),
            "https://nube.ejemplo.com/remote.php/dav/files/ana/"
        );
        assert!(
            p.donde_gestionar.is_none(),
            "no sabemos como se llama su aplicación web"
        );
    }

    #[test]
    fn un_id_desconocido_no_asume_nada() {
        assert_eq!(Preset::por_id("lo-que-sea"), Preset::generico());
        assert_eq!(Preset::por_id("iurefficient").id, "iurefficient");
    }
}
