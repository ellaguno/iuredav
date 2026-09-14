//! Aviso de versiones nuevas.
//!
//! Solo avisa: consulta la ultima release publicada en GitHub, la compara con la
//! version propia y, si hay una mas nueva, devuelve donde descargarla. No instala
//! nada. Instalar desde dentro exigiria firmar los paquetes y reemplazar un
//! binario que puede tener montajes activos; sin firma de codigo, la mitad de la
//! gracia se pierde, asi que eso queda para mas adelante.
//!
//! La consulta es anonima y sin credenciales: GitHub permite sesenta por hora y
//! direccion, y aqui se hace una al arrancar y otra al dia mientras el programa
//! siga vivo en la bandeja, que es donde pasa las semanas.

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Cada cuanto se vuelve a mirar mientras el programa esta en marcha.
pub const CADA: Duration = Duration::from_secs(24 * 60 * 60);

/// Una version mas nueva que la que corre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actualizacion {
    /// Sin la `v` delante: `0.4.0`.
    pub version: String,
    /// Pagina de la release, con los instaladores de las tres plataformas.
    pub url: String,
}

/// Lo poco que se lee de la respuesta de GitHub.
#[derive(Deserialize)]
struct ReleaseGitHub {
    tag_name: String,
    html_url: String,
}

/// `https://api.github.com/repos/{duenyo}/{repo}/releases/latest`, sacada de la
/// URL del repositorio que declara Cargo, para no tener el nombre escrito dos
/// veces. `latest` ya excluye borradores y prelanzamientos.
pub fn url_api() -> String {
    url_api_de(env!("CARGO_PKG_REPOSITORY"))
}

fn url_api_de(repositorio: &str) -> String {
    let ruta = repositorio
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .split("github.com/")
        .nth(1)
        .unwrap_or("ellaguno/iuredav");
    format!("https://api.github.com/repos/{ruta}/releases/latest")
}

/// `true` si `ultima` es estrictamente mas nueva que `actual`. Las dos con forma
/// `MAYOR.MENOR.PARCHE`, con o sin `v` delante; si alguna no se entiende, `false`:
/// ante la duda no se molesta a nadie.
pub fn es_mas_nueva(actual: &str, ultima: &str) -> bool {
    match (numeros(actual), numeros(ultima)) {
        (Some(a), Some(u)) => u > a,
        _ => false,
    }
}

fn numeros(v: &str) -> Option<[u64; 3]> {
    let v = v.trim().trim_start_matches('v');
    // Se ignora lo que vaya tras un guion (`0.4.0-beta`): no se publican
    // prelanzamientos, y si alguna vez se hiciera, no hay que anunciarlos.
    let v = v.split('-').next()?;
    let mut partes = v.split('.').map(|p| p.parse::<u64>().ok());
    let n = [partes.next()??, partes.next()??, partes.next()??];
    if partes.next().is_some() {
        return None;
    }
    Some(n)
}

/// Pregunta a GitHub. Devuelve `None` si la ultima release no es mas nueva que
/// `actual`.
pub async fn consultar(actual: &str) -> Result<Option<Actualizacion>> {
    consultar_en(&url_api(), actual).await
}

async fn consultar_en(url: &str, actual: &str) -> Result<Option<Actualizacion>> {
    let cliente = Client::builder()
        // GitHub rechaza las peticiones sin User-Agent.
        .user_agent(concat!("iuredav/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(15))
        .build()?;

    let r = cliente
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("no se pudo consultar si hay una versión nueva")?
        .error_for_status()
        .context("GitHub no respondió a la consulta de versiones")?
        .json::<ReleaseGitHub>()
        .await
        .context("la respuesta de GitHub no tiene la forma esperada")?;

    Ok(interpretar(actual, r))
}

fn interpretar(actual: &str, r: ReleaseGitHub) -> Option<Actualizacion> {
    let version = r.tag_name.trim_start_matches('v').to_string();
    es_mas_nueva(actual, &version).then_some(Actualizacion {
        version,
        url: r.html_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compara_como_semver_y_no_como_texto() {
        assert!(es_mas_nueva("0.3.0", "0.4.0"));
        assert!(es_mas_nueva("0.3.0", "v0.3.1"));
        assert!(es_mas_nueva("0.9.0", "0.10.0"), "10 > 9 aunque '1' < '9'");
        assert!(es_mas_nueva("0.3.0", "1.0.0"));
        assert!(!es_mas_nueva("0.3.0", "0.3.0"));
        assert!(!es_mas_nueva("0.4.0", "0.3.0"), "ir hacia atras no es actualizar");
    }

    /// Un prelanzamiento no cuenta como novedad, y una etiqueta rara no avisa.
    #[test]
    fn ante_la_duda_no_avisa() {
        assert!(!es_mas_nueva("0.3.0", "0.3.0-beta.1"));
        assert!(!es_mas_nueva("0.3.0", "nightly"));
        assert!(!es_mas_nueva("", "0.4.0"));
        assert!(!es_mas_nueva("0.3.0", "0.4"));
    }

    #[test]
    fn la_url_sale_del_repositorio_de_cargo() {
        assert_eq!(
            url_api_de("https://github.com/ellaguno/iuredav"),
            "https://api.github.com/repos/ellaguno/iuredav/releases/latest"
        );
        assert_eq!(
            url_api_de("https://github.com/ellaguno/iuredav.git/"),
            "https://api.github.com/repos/ellaguno/iuredav/releases/latest"
        );
        // Y la real, la que compila, apunta a este proyecto.
        assert!(url_api().contains("/repos/ellaguno/iuredav/"));
    }

    /// La forma exacta que devuelve GitHub: la etiqueta lleva la `v` y la URL es
    /// la de la pagina, no la de la API.
    #[test]
    fn entiende_la_respuesta_de_github() {
        let r: ReleaseGitHub = serde_json::from_str(
            r#"{"url":"https://api.github.com/repos/ellaguno/iuredav/releases/1",
                "html_url":"https://github.com/ellaguno/iuredav/releases/tag/v0.4.0",
                "tag_name":"v0.4.0","name":"IureDav v0.4.0","draft":false,
                "prerelease":false,"assets":[]}"#,
        )
        .unwrap();
        let a = interpretar("0.3.0", r).expect("0.4.0 es mas nueva que 0.3.0");
        assert_eq!(a.version, "0.4.0");
        assert_eq!(a.url, "https://github.com/ellaguno/iuredav/releases/tag/v0.4.0");
    }

    #[test]
    fn la_misma_version_no_es_novedad() {
        let r = ReleaseGitHub {
            tag_name: "v0.3.0".into(),
            html_url: "x".into(),
        };
        assert!(interpretar("0.3.0", r).is_none());
    }
}
