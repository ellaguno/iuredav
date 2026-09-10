//! `iuredav-probe` — ensena lo que un servidor WebDAV hace de verdad.
//!
//! Contrasta la cabecera `Allow:` que anuncia el servidor con el resultado de
//! probar cada verbo. Contra Iurefficient, la prueba de que funciona es que los dos
//! lados de la tabla **no coincidan**.
//!
//! La contrasena nunca se pasa por la linea de comandos si se puede evitar: usa la
//! variable de entorno `IUREDAV_PASS`, porque `argv` es legible por cualquier otro
//! proceso de la maquina.

use anyhow::{Context, Result};
use clap::Parser;
use iuredav_core::caps::{opciones_de_montaje, MountOptions, SemanticaSobrescritura, SoporteRango};
use iuredav_core::probe::{Probe, RUTA_SELFTEST};
use iuredav_core::ServerCapabilities;

#[derive(Parser)]
#[command(name = "iuredav-probe", about = "Descubre las capacidades reales de un servidor WebDAV")]
struct Args {
    /// URL base del WebDAV, p. ej. https://instancia.iurefficient.com/webdav/
    #[arg(long)]
    url: String,

    /// Usuario (en Iurefficient, tu correo).
    #[arg(long)]
    user: String,

    /// Contrasena de aplicacion. Preferible por entorno: argv lo ve todo el sistema.
    #[arg(long, env = "IUREDAV_PASS", hide_env_values = true)]
    pass: String,

    /// Ejecuta tambien la fase de escritura.
    ///
    /// AVISO: si el servidor no permite DELETE (Iurefficient no lo permite), el
    /// fichero de prueba NO se puede borrar y queda en el servidor.
    #[arg(long)]
    escritura: bool,

    /// Vuelca el informe completo como JSON.
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "iuredav_core=info".into()),
        )
        .with_target(false)
        .without_time()
        .init();

    let args = Args::parse();

    if args.escritura {
        eprintln!("AVISO: la fase de escritura creara {RUTA_SELFTEST}.");
        eprintln!("       Si el servidor rechaza DELETE, ese fichero quedara ahi.\n");
    }

    let probe = Probe::nuevo(&args.url, &args.user, &args.pass)?;
    let caps = probe
        .ejecutar(args.escritura)
        .await
        .context("no se pudo completar la sonda")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&caps)?);
    } else {
        imprimir(&caps);
    }

    // Codigo de salida 1 si el servidor miente: util para vigilarlo desde CI y
    // enterarse el dia que alguien arregle (o rompa) el proveedor.
    if !caps.discrepancias().is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

fn imprimir(caps: &ServerCapabilities) {
    let linea = "-".repeat(72);
    println!("\n{linea}");
    println!("  {}", caps.url);
    if let Some(s) = &caps.anunciado.server {
        println!("  Servidor: {s}");
    }
    println!("  Sondeado: {}", caps.probed_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("{linea}\n");

    println!("ANUNCIADO POR EL SERVIDOR");
    println!("  Allow: {}", si_vacio(&caps.anunciado.allow.join(", ")));
    println!("  DAV:   {}\n", si_vacio(&caps.anunciado.dav.join(", ")));

    let r = &caps.real;
    println!("MEDIDO DE VERDAD");
    fila("PROPFIND Depth 0", &r.propfind_depth0.descripcion());
    fila("PROPFIND Depth 1", &r.propfind_depth1.descripcion());
    fila("GET", &r.get.descripcion());
    fila("GET con Range", match r.rangos {
        SoporteRango::Soportado => "soportado (206)",
        SoporteRango::Ignorado => "ignorado: devuelve el fichero entero",
        SoporteRango::Desconocido => "no se pudo probar",
    });
    fila("ETag", if r.etag { "presente" } else { "AUSENTE: sin deteccion de cambios por contenido" });
    fila("Last-Modified", if r.last_modified { "presente" } else { "ausente" });
    fila("PUT (crear)", &r.put_crear.descripcion());
    fila("PUT (sobre existente)", match r.put_sobrescribir {
        SemanticaSobrescritura::Sobrescribe => "sobrescribe",
        SemanticaSobrescritura::CreaVersion => "CREA UNA VERSION NUEVA (no sobrescribe)",
        SemanticaSobrescritura::SinEfecto => "SIN EFECTO: el GET sigue devolviendo lo viejo",
        SemanticaSobrescritura::Desconocido => "no se pudo probar",
    });
    fila("MKCOL", &r.mkcol.descripcion());
    fila("MOVE", &r.mover.descripcion());
    fila("DELETE", &r.borrar.descripcion());
    fila("PROPPATCH (fecha)", &r.proppatch_modtime.descripcion());
    fila("LOCK", &r.locks.lock.descripcion());
    if let Some(cruza) = r.locks.cruza_procesos {
        fila("LOCK entre procesos", if cruza { "fiable" } else { "NO FIABLE: otro proceso pudo bloquear lo ya bloqueado" });
    }
    if !r.raiz.is_empty() {
        fila("Raiz", &r.raiz.join(", "));
    }

    let d = caps.discrepancias();
    println!("\n{linea}");
    if d.is_empty() {
        println!("  El servidor cumple lo que anuncia.");
    } else {
        println!("  EL SERVIDOR ANUNCIA {} CAPACIDAD(ES) QUE NO TIENE", d.len());
        println!("{linea}");
        for x in &d {
            println!("\n  {} — anunciado en Allow:, pero {}", x.verbo, x.real);
            println!("     {}", x.explicacion);
        }
        println!("\n  Un cliente que se crea ese anuncio (rclone incluido) planifica");
        println!("  con esas capacidades y falla al ejecutar. IureDav las retira.");
    }
    println!("{linea}\n");

    let o = opciones_de_montaje(caps, &MountOptions::default());
    println!("MONTAJE QUE SE DEDUCE DE ESTA MEDICION\n");
    println!("{}\n", o.linea_equivalente("iurefficient", "~/Iurefficient"));
}

fn fila(k: &str, v: &str) {
    println!("  {k:<24} {v}");
}

fn si_vacio(s: &str) -> String {
    if s.is_empty() { "(no lo declara)".into() } else { s.to_string() }
}
