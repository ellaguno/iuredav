//! `iuredav` — herramienta de linea de comandos de IureDav.
//!
//! Existe antes que la interfaz grafica porque es lo que permite comprobar la
//! cadena entera —sonda, derivacion de opciones, sidecar, montaje— contra un
//! servidor real sin depender de la UI.

mod montar;
mod sonda;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "iuredav",
    version,
    about = "Monta un WebDAV descubriendo antes lo que sabe hacer de verdad"
)]
struct Cli {
    #[command(subcommand)]
    orden: Orden,
}

#[derive(Subcommand)]
enum Orden {
    /// Descubre las capacidades reales de un servidor WebDAV.
    Probe(sonda::Args),
    /// Monta un perfil guardado.
    Mount(montar::Args),
    /// Lista los perfiles guardados.
    Perfiles,
    /// Elimina un perfil y su contrasena del llavero.
    Olvidar {
        /// Perfil a eliminar.
        perfil: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        // A stderr, no a stdout: con --json, stdout tiene que llevar solo JSON.
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "iuredav_core=info,iuredav=info".into()),
        )
        .with_target(false)
        .without_time()
        .init();

    match Cli::parse().orden {
        Orden::Probe(a) => sonda::ejecutar(a).await,
        Orden::Mount(a) => montar::ejecutar(a).await,
        Orden::Perfiles => listar(),
        Orden::Olvidar { perfil } => olvidar(&perfil),
    }
}

fn listar() -> Result<()> {
    let perfiles = iuredav_core::perfiles::cargar()?;
    if perfiles.is_empty() {
        println!("No hay perfiles. Crea uno con:");
        println!("  iuredav probe --url ... --user ... --guardar-como mi-perfil");
        return Ok(());
    }
    for p in perfiles {
        let caps = match &p.capacidades {
            Some(c) => format!("sondeado {}", c.probed_at.format("%Y-%m-%d")),
            None => "sin sondear".into(),
        };
        println!("  {:<16} {}", p.id, p.url);
        println!(
            "  {:<16} monta en {} · {} · {}",
            "",
            p.punto_montaje.display(),
            if p.escritura {
                "edición"
            } else {
                "solo lectura"
            },
            caps
        );
    }
    Ok(())
}

/// Quita el perfil y su credencial. Se borra el secreto aunque el perfil ya no
/// exista: una credencial huerfana en el llavero no le sirve a nadie.
fn olvidar(id: &str) -> Result<()> {
    let usuario = iuredav_core::perfiles::buscar(id)?.map(|p| p.usuario);
    let habia = iuredav_core::perfiles::borrar(id)?;

    if let Some(u) = &usuario {
        iuredav_core::secretos::borrar(id, u)?;
    }
    if habia {
        println!("Perfil '{id}' eliminado, y su contraseña con el.");
    } else {
        println!("No habia ningún perfil '{id}'.");
    }
    Ok(())
}

/// Obtiene la contrasena sin que pase nunca por `argv`, que es legible por
/// cualquier otro proceso de la maquina.
pub fn contrasena(perfil_id: &str, usuario: &str, del_entorno: Option<String>) -> Result<String> {
    if let Some(p) = del_entorno {
        return Ok(p);
    }
    if let Some(p) = iuredav_core::secretos::leer(perfil_id, usuario)? {
        return Ok(p);
    }
    bail!(
        "no hay contrasena para {usuario}. Pasala en IUREDAV_PASS, o guardala en el llavero con:\n  \
         iuredav probe --url ... --user {usuario} --guardar-como {perfil_id}"
    )
}

pub fn confirmar(pregunta: &str) -> Result<bool> {
    use std::io::{stderr, stdin, Write};
    // La pregunta tambien va a stderr, por lo mismo.
    eprint!("{pregunta} [s/N] ");
    stderr().flush().ok();
    let mut r = String::new();
    stdin()
        .read_line(&mut r)
        .context("no se pudo leer la respuesta")?;
    Ok(matches!(
        r.trim().to_lowercase().as_str(),
        "s" | "si" | "sí"
    ))
}
