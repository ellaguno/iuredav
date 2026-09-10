//! Aplicacion de escritorio de IureDav.
//!
//! Esta capa es deliberadamente delgada: toda la logica —sondear, derivar
//! opciones, montar— vive en `iuredav-core` y ya se puede ejercitar desde la
//! linea de ordenes. Aqui solo se expone al frontend y se mantiene vivo el
//! sidecar de rclone mientras la ventana esta abierta.

use std::collections::HashMap;

use iuredav_core::caps::{opciones_de_montaje, MountOptions, ServerCapabilities};
use iuredav_core::errors::MensajeAmistoso;
use iuredav_core::perfiles::{self, Perfil};
use iuredav_core::plataforma::{self, Requisito};
use iuredav_core::presets::Preset;
use iuredav_core::probe::Probe;
use iuredav_core::rclone::{ruta_binario, Rclone};
use iuredav_core::secretos;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

/// Un unico sidecar para todos los montajes, arrancado la primera vez que hace
/// falta y apagado al cerrar la ventana.
#[derive(Default)]
pub struct Estado {
    rclone: Mutex<Option<Rclone>>,
    /// id de perfil -> punto de montaje.
    montados: Mutex<HashMap<String, String>>,
}

/// Los errores cruzan a JavaScript como texto: el frontend los ensena tal cual,
/// asi que tienen que estar redactados para una persona, no para un log.
type Resp<T> = Result<T, String>;

fn texto(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
pub struct VistaConexion {
    #[serde(flatten)]
    perfil: Perfil,
    montado: bool,
    /// Verbos que el servidor anuncia y no cumple. Vacio si nunca se sondeo.
    incumple: Vec<String>,
}

#[tauri::command]
async fn listar_conexiones(estado: State<'_, Estado>) -> Resp<Vec<VistaConexion>> {
    let montados = estado.montados.lock().await;
    Ok(perfiles::cargar()
        .map_err(texto)?
        .into_iter()
        .map(|p| VistaConexion {
            montado: montados.contains_key(&p.id),
            incumple: p
                .capacidades
                .as_ref()
                .map(|c| c.discrepancias().into_iter().map(|d| d.verbo).collect())
                .unwrap_or_default(),
            perfil: p,
        })
        .collect())
}

/// Sonda. `escritura` deja un fichero que en Iurefficient no se puede borrar, asi
/// que el frontend solo la ofrece tras avisar de eso explicitamente.
#[tauri::command]
async fn probar(
    url: String,
    usuario: String,
    password: String,
    escritura: bool,
    preset: String,
) -> Resp<ServerCapabilities> {
    let p = Preset::por_id(&preset);
    Probe::con_preset(&p.normalizar_url(&url), &usuario, &password, p)
        .map_err(texto)?
        .ejecutar(escritura)
        .await
        .map_err(texto)
}

/// Tipos de servidor entre los que puede elegir el usuario.
#[tauri::command]
fn listar_presets() -> Vec<Preset> {
    Preset::todos()
}

/// Los datos de una conexion, tal y como los manda el formulario.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatosConexion {
    id: String,
    nombre: String,
    url: String,
    usuario: String,
    password: String,
    punto_montaje: String,
    capacidades: Option<ServerCapabilities>,
    preset: String,
}

#[tauri::command]
async fn guardar_conexion(datos: DatosConexion) -> Resp<()> {
    let tipo = Preset::por_id(&datos.preset);
    let url = tipo.normalizar_url(&datos.url);

    let mut p = perfiles::buscar(&datos.id)
        .map_err(texto)?
        .unwrap_or_else(|| Perfil::con_preset(&datos.id, &url, &datos.usuario, &tipo));
    p.nombre = datos.nombre;
    p.preset = tipo.id.clone();
    p.url = url;
    p.usuario = datos.usuario.clone();
    p.punto_montaje = datos.punto_montaje.into();
    if datos.capacidades.is_some() {
        p.capacidades = datos.capacidades;
    }
    perfiles::upsert(p).map_err(texto)?;

    // La contrasena nunca entra en el perfil: va al llavero del sistema.
    secretos::guardar(&datos.id, &datos.usuario, &datos.password).map_err(texto)
}

#[tauri::command]
async fn olvidar_conexion(id: String) -> Resp<()> {
    if let Some(p) = perfiles::buscar(&id).map_err(texto)? {
        secretos::borrar(&id, &p.usuario).map_err(texto)?;
    }
    perfiles::borrar(&id).map_err(texto)?;
    Ok(())
}

/// Arranca el sidecar si aun no lo esta, y conecta su flujo de avisos a la ventana.
async fn asegurar_sidecar(app: &AppHandle, estado: &State<'_, Estado>) -> Result<(), String> {
    let mut guard = estado.rclone.lock().await;
    if guard.is_some() {
        return Ok(());
    }
    let (rc, mut avisos) = Rclone::arrancar(&ruta_binario()).await.map_err(texto)?;

    let app = app.clone();
    tokio::spawn(async move {
        while let Some(m) = avisos.recv().await {
            // El frontend los ensena como notificaciones. Son la traduccion de los
            // limites del servidor a algo que una persona entiende.
            let _ = app.emit("iuredav://aviso", &m);
        }
    });

    *guard = Some(rc);
    Ok(())
}

#[tauri::command]
async fn montar(
    app: AppHandle,
    estado: State<'_, Estado>,
    id: String,
    escritura: bool,
) -> Resp<String> {
    let mut perfil = perfiles::buscar(&id)
        .map_err(texto)?
        .ok_or_else(|| format!("no existe la conexion '{id}'"))?;

    let password = secretos::leer(&id, &perfil.usuario)
        .map_err(texto)?
        .ok_or("no hay contrasena guardada para esta conexion")?;

    // Sin medicion no hay montaje: solo la fase de lectura, que no deja rastro.
    let caps = match perfil.capacidades.clone() {
        Some(c) => c,
        None => {
            let c = Probe::con_preset(&perfil.url, &perfil.usuario, &password, perfil.preset())
                .map_err(texto)?
                .ejecutar(false)
                .await
                .map_err(texto)?;
            perfil.capacidades = Some(c.clone());
            perfiles::upsert(perfil.clone()).map_err(texto)?;
            c
        }
    };

    // Antes de nada: si a la maquina le falta la pieza que permite montar, el
    // error de rclone no diria nada util. Mejor explicarlo aqui.
    if let Err(r) = plataforma::comprobar() {
        return Err(format!(
            "Falta {}. {} {}",
            r.que_falta, r.por_que, r.como_instalar
        ));
    }

    let opts = MountOptions {
        escritura,
        ..Default::default()
    };
    let opciones = opciones_de_montaje(&caps, &opts);
    perfiles::preparar_punto(&perfil.punto_montaje).map_err(texto)?;

    asegurar_sidecar(&app, &estado).await?;
    let guard = estado.rclone.lock().await;
    let rc = guard
        .as_ref()
        .ok_or("el sidecar de rclone no esta disponible")?;

    rc.fijar_gestor(perfil.preset().donde_gestionar.clone());
    rc.crear_remoto(&id, &perfil.url, &perfil.usuario, &password)
        .await
        .map_err(texto)?;

    let punto = perfil.punto_montaje.to_string_lossy().to_string();
    rc.montar(&id, &punto, &opciones).await.map_err(texto)?;

    estado.montados.lock().await.insert(id, punto.clone());
    Ok(punto)
}

#[tauri::command]
async fn desmontar(estado: State<'_, Estado>, id: String) -> Resp<()> {
    let punto = estado
        .montados
        .lock()
        .await
        .remove(&id)
        .ok_or("esa conexion no esta montada")?;

    let guard = estado.rclone.lock().await;
    if let Some(rc) = guard.as_ref() {
        rc.desmontar(&punto).await.map_err(texto)?;
    }
    Ok(())
}

/// Olvida la cache de directorios: el boton "Actualizar". Hace falta porque el
/// proveedor no notifica cambios y se monta con `--poll-interval 0`.
#[tauri::command]
async fn refrescar(estado: State<'_, Estado>, id: String, ruta: String) -> Resp<()> {
    let guard = estado.rclone.lock().await;
    let rc = guard.as_ref().ok_or("no hay ninguna conexion activa")?;
    rc.refrescar(&id, &ruta).await.map_err(texto)
}

#[tauri::command]
async fn estadisticas(estado: State<'_, Estado>) -> Resp<serde_json::Value> {
    let guard = estado.rclone.lock().await;
    match guard.as_ref() {
        Some(rc) => rc.estadisticas().await.map_err(texto),
        None => Ok(serde_json::json!({})),
    }
}

/// Lista una carpeta del servidor sin pasar por el punto de montaje: es lo que
/// alimenta el explorador integrado.
#[tauri::command]
async fn listar_remoto(
    estado: State<'_, Estado>,
    id: String,
    ruta: String,
) -> Resp<serde_json::Value> {
    let guard = estado.rclone.lock().await;
    let rc = guard.as_ref().ok_or("no hay ninguna conexion activa")?;
    rc.llamar(
        "operations/list",
        serde_json::json!({ "fs": format!("{id}:"), "remote": ruta }),
    )
    .await
    .map_err(texto)
}

/// `None` si esta maquina puede montar. Si no, que falta y como conseguirlo.
#[tauri::command]
fn comprobar_sistema() -> Option<Requisito> {
    plataforma::comprobar().err()
}

/// Nombre del destino segun la plataforma: en Windows es una unidad, no una carpeta.
#[tauri::command]
fn nombre_destino() -> &'static str {
    plataforma::nombre_del_destino()
}

/// Cambia entre solo lectura y edicion. Se guarda en el perfil porque es una
/// decision del usuario sobre esa conexion, no de una sesion suelta.
#[tauri::command]
async fn cambiar_modo(id: String, escritura: bool) -> Resp<()> {
    let mut p = perfiles::buscar(&id)
        .map_err(texto)?
        .ok_or_else(|| format!("no existe la conexion '{id}'"))?;
    p.escritura = escritura;
    perfiles::upsert(p).map_err(texto)
}

#[tauri::command]
fn punto_sugerido(id: String) -> String {
    perfiles::punto_por_defecto(&id)
        .to_string_lossy()
        .into_owned()
}

/// Se declara para que el tipo del evento llegue a TypeScript por la misma via que
/// el resto; el frontend lo recibe por `listen`, no por `invoke`.
#[allow(dead_code)]
fn _tipo_aviso(_: MensajeAmistoso) {}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "iuredav_core=info,iuredav_app=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Estado::default())
        .invoke_handler(tauri::generate_handler![
            listar_conexiones,
            probar,
            guardar_conexion,
            olvidar_conexion,
            montar,
            desmontar,
            refrescar,
            estadisticas,
            listar_remoto,
            punto_sugerido,
            comprobar_sistema,
            nombre_destino,
            cambiar_modo,
            listar_presets,
        ])
        .on_window_event(|ventana, evento| {
            // Cerrar la ventana tiene que desmontar: dejar un punto de montaje
            // colgado obliga al usuario a arreglarlo desde una terminal, que es
            // justo lo que esta aplicacion existe para evitar.
            if let tauri::WindowEvent::Destroyed = evento {
                let estado = ventana.state::<Estado>();
                let rclone = estado.rclone.blocking_lock().take();
                if let Some(rc) = rclone {
                    tauri::async_runtime::block_on(async {
                        let _ = rc.apagar().await;
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("no se pudo arrancar IureDav");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El puente con TypeScript no lo comprueba ningun compilador: si un nombre de
    /// campo se desvia, guardar una conexion falla solo en tiempo de ejecucion, y
    /// solo al pulsar el boton. Esta prueba fija la forma exacta del objeto que
    /// manda `src/api.ts`.
    #[test]
    fn el_formulario_encaja_con_lo_que_espera_rust() {
        let del_frontend = serde_json::json!({
            "id": "trabajo",
            "nombre": "Iurefficient",
            "url": "https://x.ejemplo.com",
            "usuario": "ana@despacho.com",
            "password": "iurdav_secreto",
            "puntoMontaje": "/home/ana/Iurefficient",
            "capacidades": null,
            "preset": "iurefficient"
        });

        let d: DatosConexion =
            serde_json::from_value(del_frontend).expect("api.ts y DatosConexion no encajan");
        assert_eq!(d.punto_montaje, "/home/ana/Iurefficient");
        assert_eq!(d.preset, "iurefficient");
        assert!(d.capacidades.is_none());
    }

    /// La estructura que viaja de vuelta tiene que llevar el campo `preset`, o la
    /// interfaz no sabria de que tipo es cada conexion.
    #[test]
    fn la_vista_de_conexion_lleva_el_tipo_de_servidor() {
        let v = VistaConexion {
            perfil: Perfil::nuevo("x", "https://a.test/", "u@e.c"),
            montado: false,
            incumple: vec!["DELETE".into()],
        };
        let j = serde_json::to_value(&v).unwrap();
        assert_eq!(
            j["preset"], "iurefficient",
            "el perfil se aplana en la vista"
        );
        assert_eq!(j["montado"], false);
        assert_eq!(j["incumple"][0], "DELETE");
        // Y no puede llevar la contrasena, que vive en el llavero.
        assert!(j.get("password").is_none());
    }
}
