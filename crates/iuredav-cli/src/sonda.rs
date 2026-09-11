//! Subcomando `probe`.

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use iuredav_core::caps::{opciones_de_montaje, MountOptions, SemanticaSobrescritura, SoporteRango};
use iuredav_core::perfiles::{self, Perfil};
use iuredav_core::presets::Preset;
use iuredav_core::probe::Probe;
use iuredav_core::{secretos, ServerCapabilities};

#[derive(ClapArgs)]
pub struct Args {
    /// URL del servidor. Con el perfil de Iurefficient basta el dominio.
    #[arg(long)]
    pub url: String,

    /// Tipo de servidor: `iurefficient` u `otro` para cualquier WebDAV.
    #[arg(long, default_value = "iurefficient", value_parser = ["iurefficient", "otro", "generico"])]
    pub servidor: String,

    /// Usuario (en Iurefficient, tu correo).
    #[arg(long)]
    pub user: String,

    /// Contrasena de aplicacion. Preferible por entorno: argv lo ve todo el sistema.
    #[arg(long, env = "IUREDAV_PASS", hide_env_values = true)]
    pub pass: String,

    /// Prueba tambien PUT, MKCOL, MOVE, DELETE, PROPPATCH y LOCK.
    ///
    /// AVISO: si el servidor rechaza DELETE, el fichero de prueba NO se puede
    /// borrar y queda en el servidor.
    #[arg(long)]
    pub escritura: bool,

    /// Guarda un perfil con este nombre y mete la contrasena en el llavero.
    #[arg(long, value_name = "ID")]
    pub guardar_como: Option<String>,

    /// Vuelca el informe completo como JSON. stdout lleva solo JSON.
    #[arg(long)]
    pub json: bool,

    /// No preguntar antes de la fase de escritura. Para automatizacion.
    #[arg(long)]
    pub si: bool,
}

pub async fn ejecutar(args: Args) -> Result<()> {
    let preset = Preset::por_id(if args.servidor == "iurefficient" {
        "iurefficient"
    } else {
        "generico"
    });
    let url = preset.normalizar_url(&args.url);

    if args.escritura {
        eprintln!("AVISO: se creara {} en el servidor.", preset.ruta_selftest);
        eprintln!("       Si el servidor rechaza DELETE, ese fichero quedará ahi.");
        if !args.si && !crate::confirmar("¿Continuar?")? {
            eprintln!("Cancelado. Sin --escritura la sonda no deja rastro.");
            return Ok(());
        }
    }

    let probe = Probe::con_preset(&url, &args.user, &args.pass, preset.clone())?;
    let caps = probe
        .ejecutar(args.escritura)
        .await
        .context("no se pudo completar la sonda")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&caps)?);
    } else {
        imprimir(&caps);
    }

    if let Some(id) = &args.guardar_como {
        let mut p =
            perfiles::buscar(id)?.unwrap_or_else(|| Perfil::nuevo(id, &args.url, &args.user));
        p.url = caps.url.clone();
        p.usuario = args.user.clone();
        p.capacidades = Some(caps.clone());
        perfiles::upsert(p)?;
        secretos::guardar(id, &args.user, &args.pass)?;
        println!("Perfil '{id}' guardado. La contraseña esta en el llavero del sistema.");
        println!("Ya puedes montarlo con:  iuredav mount {id}");
    }

    // Codigo 1 si el servidor miente: util para vigilarlo desde CI y enterarse el
    // dia que alguien arregle (o rompa) el proveedor.
    if !caps.discrepancias().is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

pub fn imprimir(caps: &ServerCapabilities) {
    let linea = "-".repeat(72);
    println!("\n{linea}");
    println!("  {}", caps.url);
    if let Some(s) = &caps.anunciado.server {
        println!("  Servidor: {s}");
    }
    println!(
        "  Sondeado: {}",
        caps.probed_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("{linea}\n");

    println!("ANUNCIADO POR EL SERVIDOR");
    println!("  Allow: {}", si_vacio(&caps.anunciado.allow.join(", ")));
    println!("  DAV:   {}\n", si_vacio(&caps.anunciado.dav.join(", ")));

    let r = &caps.real;
    println!("MEDIDO DE VERDAD");
    fila("PROPFIND Depth 0", &r.propfind_depth0.descripcion());
    fila("PROPFIND Depth 1", &r.propfind_depth1.descripcion());
    fila("GET", &r.get.descripcion());
    fila(
        "GET con Range",
        match r.rangos {
            SoporteRango::Soportado => "soportado (206)",
            SoporteRango::Ignorado => "ignorado: devuelve el fichero entero",
            SoporteRango::Desconocido => "no se pudo probar",
        },
    );
    fila(
        "ETag",
        if r.etag {
            "presente"
        } else {
            "AUSENTE: sin detección de cambios por contenido"
        },
    );
    fila(
        "Last-Modified",
        if r.last_modified {
            "presente"
        } else {
            "ausente"
        },
    );
    fila("PUT (crear)", &r.put_crear.descripcion());
    fila(
        "PUT (sobre existente)",
        match r.put_sobrescribir {
            SemanticaSobrescritura::Sobrescribe => "sobrescribe",
            SemanticaSobrescritura::CreaVersion => "CREA UNA VERSIÓN NUEVA (no sobrescribe)",
            SemanticaSobrescritura::SinEfecto => "SIN EFECTO: el GET sigue devolviendo lo viejo",
            SemanticaSobrescritura::Desconocido => "no se pudo probar",
        },
    );
    fila("MKCOL", &r.mkcol.descripcion());
    fila("MOVE", &r.mover.descripcion());
    fila("DELETE", &r.borrar.descripcion());
    fila("PROPPATCH (fecha)", &r.proppatch_modtime.descripcion());
    fila("LOCK", &r.locks.lock.descripcion());
    if let Some(cruza) = r.locks.cruza_procesos {
        fila(
            "LOCK entre procesos",
            if cruza {
                "fiable"
            } else {
                "NO FIABLE: otro proceso pudo bloquear lo ya bloqueado"
            },
        );
    }
    if !r.raiz.is_empty() {
        fila("Raíz", &r.raiz.join(", "));
    }

    // Lo que no se puede hacer va primero, porque es lo que le importa a quien
    // va a usar la unidad. Que ademas estuviera prometido es un agravante, no el
    // titular: un servidor honesto sobre sus limites sigue teniendolos.
    let l = caps.limitaciones();
    let mentidas = l.iter().filter(|x| x.anunciado).count();
    println!("\n{linea}");
    if l.is_empty() {
        println!("  Esta unidad no tiene límites conocidos.");
    } else {
        println!("  LO QUE ESTA UNIDAD NO PUEDE HACER ({})", l.len());
        if mentidas > 0 {
            println!("  {mentidas} de ellas anunciadas por el servidor en su Allow:");
        }
        println!("{linea}");
        for x in &l {
            let como = if x.anunciado {
                "anunciado en Allow:, pero "
            } else {
                ""
            };
            println!("\n  {} — {como}{}", x.verbo, x.real);
            println!("     {}", x.explicacion);
        }
        if mentidas > 0 {
            println!("\n  Un cliente que se crea ese anuncio (rclone incluido) planifica");
            println!("  con esas capacidades y falla al ejecutar. IureDav las retira.");
        }
    }
    println!("{linea}\n");

    let o = opciones_de_montaje(caps, &MountOptions::default());
    println!("MONTAJE QUE SE DEDUCE DE ESTA MEDICIÓN\n");
    println!(
        "{}\n",
        o.linea_equivalente("iurefficient", "~/Iurefficient")
    );
}

fn fila(k: &str, v: &str) {
    println!("  {k:<24} {v}");
}

fn si_vacio(s: &str) -> String {
    if s.is_empty() {
        "(no lo declara)".into()
    } else {
        s.to_string()
    }
}
