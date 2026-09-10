//! Utilidades minimas de WebDAV: metodos no estandar y lectura de `multistatus`.

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Method;

pub fn metodo(nombre: &str) -> Method {
    Method::from_bytes(nombre.as_bytes()).expect("nombre de metodo HTTP valido")
}

/// Una entrada `<response>` de un `multistatus`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DavEntry {
    pub href: String,
    pub es_coleccion: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_length: Option<u64>,
}

impl DavEntry {
    /// Ultimo segmento del href, decodificado. `/webdav/Casos/` -> `Casos`.
    pub fn nombre(&self) -> String {
        let s = self.href.trim_end_matches('/');
        let bruto = s.rsplit('/').next().unwrap_or(s);
        percent_decode(bruto)
    }
}

/// Decodifica `%XX` en un segmento de ruta. Los nombres de documento de
/// Iurefficient llevan acentos y enes, asi que esto no es opcional.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn nombre_local(e: &[u8]) -> String {
    String::from_utf8_lossy(e).to_ascii_lowercase()
}

/// Extrae las entradas de un cuerpo `<multistatus>`, ignorando los espacios de
/// nombres (cada servidor usa su propio prefijo: `D:`, `d:`, `ns0:`...).
pub fn parse_multistatus(xml: &str) -> Result<Vec<DavEntry>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut salida = Vec::new();
    let mut actual: Option<DavEntry> = None;
    let mut pila: Vec<String> = Vec::new();

    loop {
        match reader.read_event().context("XML multistatus mal formado")? {
            Event::Start(e) => {
                let n = nombre_local(e.local_name().as_ref());
                if n == "response" {
                    actual = Some(DavEntry::default());
                } else if n == "collection" {
                    if let Some(c) = actual.as_mut() {
                        c.es_coleccion = true;
                    }
                }
                pila.push(n);
            }
            Event::Empty(e) => {
                let n = nombre_local(e.local_name().as_ref());
                if n == "collection" {
                    if let Some(c) = actual.as_mut() {
                        c.es_coleccion = true;
                    }
                }
            }
            Event::Text(t) => {
                let texto = t.unescape().unwrap_or_default().trim().to_string();
                if texto.is_empty() {
                    continue;
                }
                if let (Some(c), Some(n)) = (actual.as_mut(), pila.last()) {
                    match n.as_str() {
                        "href" if c.href.is_empty() => c.href = texto,
                        "getetag" => c.etag = Some(texto),
                        "getlastmodified" => c.last_modified = Some(texto),
                        "getcontentlength" => c.content_length = texto.parse().ok(),
                        _ => {}
                    }
                }
            }
            Event::End(e) => {
                let n = nombre_local(e.local_name().as_ref());
                pila.pop();
                if n == "response" {
                    if let Some(c) = actual.take() {
                        salida.push(c);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(salida)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Respuesta con la forma que devuelve WsgiDAV, con prefijo `D:`.
    const MULTISTATUS: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/webdav/</D:href>
    <D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop>
    <D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  </D:response>
  <D:response>
    <D:href>/webdav/Casos/</D:href>
    <D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop>
    <D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  </D:response>
  <D:response>
    <D:href>/webdav/General/An%C3%A1lisis%20de%20Pe%C3%B1a.docx</D:href>
    <D:propstat><D:prop>
      <D:resourcetype/>
      <D:getcontentlength>24576</D:getcontentlength>
      <D:getlastmodified>Tue, 09 Sep 2026 12:00:00 GMT</D:getlastmodified>
    </D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  </D:response>
</D:multistatus>"#;

    #[test]
    fn lee_colecciones_y_ficheros() {
        let e = parse_multistatus(MULTISTATUS).unwrap();
        assert_eq!(e.len(), 3);
        assert!(e[0].es_coleccion);
        assert!(e[1].es_coleccion);
        assert!(!e[2].es_coleccion);
        assert_eq!(e[2].content_length, Some(24576));
        assert!(e[2].last_modified.is_some());
        assert!(e[2].etag.is_none(), "este servidor no da etag");
    }

    /// Los nombres con acentos son justo los que rompieron la subida antes de que
    /// se arreglara `secure_filename`, asi que el decodificador tiene que aguantarlos.
    #[test]
    fn decodifica_acentos_en_los_nombres() {
        let e = parse_multistatus(MULTISTATUS).unwrap();
        assert_eq!(e[2].nombre(), "Análisis de Peña.docx");
        assert_eq!(e[1].nombre(), "Casos");
    }
}
