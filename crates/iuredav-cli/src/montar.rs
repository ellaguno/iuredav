//! Subcomando `mount`. Ejercita la cadena entera: perfil, llavero, sonda,
//! derivacion de opciones, sidecar y montaje.

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use iuredav_core::caps::{opciones_de_montaje, MountOptions};
use iuredav_core::errors::Severidad;
use iuredav_core::perfiles::{self, Perfil};
use iuredav_core::plataforma;
use iuredav_core::probe::Probe;
use iuredav_core::rclone::{ruta_binario, Rclone};

#[derive(ClapArgs)]
pub struct Args {
    /// Perfil a montar (ver `iuredav perfiles`).
    pub perfil: String,

    /// Contrasena. Si falta, se busca en el llavero del sistema.
    #[arg(long, env = "IUREDAV_PASS", hide_env_values = true)]
    pub pass: Option<String>,

    /// Monta en modo edicion.
    ///
    /// Solo surte efecto si la sonda confirmo que el servidor acepta PUT. Ten en
    /// cuenta que en Iurefficient cada guardado crea una version nueva.
    #[arg(long)]
    pub escritura: bool,

    /// Punto de montaje, si quieres uno distinto al del perfil.
    #[arg(long)]
    pub en: Option<std::path::PathBuf>,

    /// Ruta al binario de rclone.
    #[arg(long)]
    pub rclone: Option<String>,
}

pub async fn ejecutar(args: Args) -> Result<()> {
    let mut perfil = perfiles::buscar(&args.perfil)?.with_context(|| {
        format!(
            "no existe el perfil '{}'. Mira `iuredav perfiles`",
            args.perfil
        )
    })?;

    if let Some(p) = args.en {
        perfil.punto_montaje = p;
    }
    let password = crate::contrasena(&perfil.id, &perfil.usuario, args.pass)?;

    // Las capacidades mandan sobre todo lo demas. Si el perfil no las tiene, se
    // miden ahora, pero solo la fase de lectura: la de escritura deja ficheros que
    // no se pueden borrar, asi que nunca se ejecuta sin que el usuario lo pida.
    let caps = match perfil.capacidades.clone() {
        Some(c) => c,
        None => {
            println!("El perfil no esta sondeado. Midiendo capacidades (solo lectura)...");
            let c = Probe::con_preset(&perfil.url, &perfil.usuario, &password, perfil.preset())?
                .ejecutar(false)
                .await
                .context("no se pudo sondear el servidor")?;
            perfil.capacidades = Some(c.clone());
            perfiles::upsert(perfil.clone())?;
            c
        }
    };

    let quiere_escribir = args.escritura || perfil.escritura;
    if quiere_escribir && !caps.real.put_crear.usable() {
        println!("Aviso: pediste modo edicion, pero la sonda no ha confirmado que el");
        println!("       servidor acepte PUT. Se monta en solo lectura.");
        println!(
            "       Para comprobarlo:  iuredav probe --url {} --user {} --escritura",
            perfil.url, perfil.usuario
        );
    }

    let opts = MountOptions {
        escritura: quiere_escribir,
        ..Default::default()
    };
    let opciones = opciones_de_montaje(&caps, &opts);
    let solo_lectura = opciones.vfs.get("ReadOnly") == Some(&serde_json::json!(true));

    if let Err(r) = plataforma::comprobar() {
        println!("Falta {} para poder montar.", r.que_falta);
        println!("  {}", r.por_que);
        println!("  {}", r.como_instalar);
        if let Some(u) = r.url {
            println!("  {u}");
        }
        anyhow::bail!("requisito del sistema sin cumplir");
    }

    perfiles::preparar_punto(&perfil.punto_montaje)?;

    let binario = args.rclone.unwrap_or_else(ruta_binario);
    let (rclone, mut avisos) = Rclone::arrancar(&binario).await?;

    rclone
        .crear_remoto(&perfil.id, &perfil.url, &perfil.usuario, &password)
        .await
        .context("no se pudo configurar el remoto en rclone")?;

    let punto = perfil.punto_montaje.to_string_lossy().to_string();
    rclone
        .montar(&perfil.id, &punto, &opciones)
        .await
        .with_context(|| format!("no se pudo montar en {punto}"))?;

    resumen(&perfil, &punto, solo_lectura, &caps);

    // Las notificaciones que en la interfaz grafica saldran como avisos del sistema.
    tokio::spawn(async move {
        while let Some(m) = avisos.recv().await {
            let etiqueta = match m.severidad {
                Severidad::Limite => "LIMITE",
                Severidad::Aviso => "AVISO ",
                Severidad::Error => "ERROR ",
            };
            match &m.ruta {
                Some(r) => println!(
                    "\n[{etiqueta}] {} — {}\n         {}",
                    m.titulo, r, m.detalle
                ),
                None => println!("\n[{etiqueta}] {}\n         {}", m.titulo, m.detalle),
            }
        }
    });

    tokio::signal::ctrl_c()
        .await
        .context("fallo al esperar Ctrl+C")?;
    println!("\nDesmontando...");

    if let Err(e) = rclone.desmontar(&punto).await {
        eprintln!("aviso: {e}");
    }
    rclone.apagar().await?;
    println!("Desmontado.");
    Ok(())
}

fn resumen(
    perfil: &Perfil,
    punto: &str,
    solo_lectura: bool,
    caps: &iuredav_core::ServerCapabilities,
) {
    println!("\n  {} montado en {punto}", perfil.nombre);
    println!(
        "  Modo: {}",
        if solo_lectura {
            "solo lectura"
        } else {
            "EDICION"
        }
    );

    let d = caps.discrepancias();
    if !d.is_empty() {
        let verbos: Vec<_> = d.iter().map(|x| x.verbo.as_str()).collect();
        println!(
            "\n  Este servidor anuncia {} pero no los cumple,",
            verbos.join(", ")
        );
        println!("  asi que se le han retirado a rclone para que no planifique con ellos.");
    }
    if !solo_lectura {
        println!("\n  En modo edicion, cada guardado crea una version nueva en el servidor.");
        println!("  Desde la unidad no se puede borrar ni renombrar.");
    }
    println!("\n  Ctrl+C para desmontar.\n");
}
