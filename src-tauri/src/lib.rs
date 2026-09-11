//! Aplicacion de escritorio de IureDav.
//!
//! Esta capa es deliberadamente delgada: toda la logica —sondear, derivar
//! opciones, montar— vive en `iuredav-core` y ya se puede ejercitar desde la
//! linea de ordenes. Aqui solo se expone al frontend y se mantiene vivo el
//! sidecar de rclone mientras la ventana esta abierta.

mod bandeja;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use iuredav_core::anclajes::{self, Resumen};
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
    pub montados: Mutex<HashMap<String, String>>,
    /// Si hay icono de bandeja. Decide que hace cerrar la ventana: si no lo hay,
    /// esconderla dejaria al usuario sin ninguna forma de salir del programa.
    pub hay_bandeja: AtomicBool,
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
    /// Lo que esta unidad no puede hacer, lo anuncie el servidor o no. Vacio si
    /// nunca se sondeo. No sale de las discrepancias: un servidor honesto sobre
    /// sus limites sigue teniendolos.
    limites: Vec<String>,
}

#[tauri::command]
async fn listar_conexiones(estado: State<'_, Estado>) -> Resp<Vec<VistaConexion>> {
    let montados = estado.montados.lock().await;
    Ok(perfiles::cargar()
        .map_err(texto)?
        .into_iter()
        .map(|p| VistaConexion {
            montado: montados.contains_key(&p.id),
            limites: p
                .capacidades
                .as_ref()
                .map(|c| c.limitaciones().into_iter().map(|l| l.verbo).collect())
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

/// Vuelve a medir una conexion ya guardada y actualiza su informe.
///
/// Hace falta por dos motivos, y ninguno es cosmetico. Uno: la medicion se guarda
/// en el perfil y no caduca, asi que sin esto un servidor que manana arregle
/// DELETE seguiria mutilado para siempre. Y dos: el formulario sondea **solo la
/// lectura** a proposito, para no dejar rastro en un servidor que el usuario
/// quiza ni llegue a guardar; pero eso deja `put_crear` en «sin probar», y con
/// eso [`opciones_de_montaje`] fuerza el montaje a solo lectura. Sin esta orden,
/// activar el modo edicion desde la ventana no podria surtir efecto nunca.
///
/// La contrasena sale del llavero: aqui ya no la tiene el frontend.
#[tauri::command]
async fn resondear(id: String, escritura: bool) -> Resp<ServerCapabilities> {
    let mut perfil = perfiles::buscar(&id)
        .map_err(texto)?
        .ok_or_else(|| format!("no existe la conexión '{id}'"))?;

    let password = secretos::leer(&id, &perfil.usuario)
        .map_err(texto)?
        .ok_or("no hay contraseña guardada para esta conexión")?;

    let caps = Probe::con_preset(&perfil.url, &perfil.usuario, &password, perfil.preset())
        .map_err(texto)?
        .ejecutar(escritura)
        .await
        .map_err(texto)?;

    perfil.capacidades = Some(caps.clone());
    perfiles::upsert(perfil).map_err(texto)?;
    Ok(caps)
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

/// Monta o desmonta segun el estado actual. Lo usa el menu de la bandeja.
pub async fn alternar_montaje(app: &AppHandle, id: &str) -> Result<(), String> {
    let estado = app.state::<Estado>();
    let montado = estado.montados.lock().await.contains_key(id);
    if montado {
        desmontar_perfil(&estado, id).await
    } else {
        let escritura = perfiles::buscar(id)
            .map_err(texto)?
            .is_some_and(|p| p.escritura);
        montar_perfil(app, &estado, id.to_string(), escritura)
            .await
            .map(|_| ())
    }
}

/// Desmonta todo y apaga el sidecar. Se llama al salir.
pub async fn apagar_todo(app: &AppHandle) {
    let estado = app.state::<Estado>();
    let rclone = estado.rclone.lock().await.take();
    if let Some(rc) = rclone {
        let _ = rc.apagar().await;
    }
    estado.montados.lock().await.clear();
}

#[tauri::command]
async fn montar(
    app: AppHandle,
    estado: State<'_, Estado>,
    id: String,
    escritura: bool,
) -> Resp<String> {
    let punto = montar_perfil(&app, &estado, id, escritura).await?;
    bandeja::refrescar(&app).await;
    Ok(punto)
}

async fn montar_perfil(
    app: &AppHandle,
    estado: &State<'_, Estado>,
    id: String,
    escritura: bool,
) -> Resp<String> {
    let mut perfil = perfiles::buscar(&id)
        .map_err(texto)?
        .ok_or_else(|| format!("no existe la conexión '{id}'"))?;

    let password = secretos::leer(&id, &perfil.usuario)
        .map_err(texto)?
        .ok_or("no hay contraseña guardada para esta conexión")?;

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

    asegurar_sidecar(app, estado).await?;
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

    estado
        .montados
        .lock()
        .await
        .insert(id.clone(), punto.clone());

    // Lo anclado se recalienta al montar: la cache caduca y puede haber sido
    // desalojada desde la ultima sesion.
    for carpeta in perfil.anclados.clone() {
        calentar_en_segundo_plano(app.clone(), id.clone(), punto.clone(), carpeta);
    }

    Ok(punto)
}

#[tauri::command]
async fn desmontar(app: AppHandle, estado: State<'_, Estado>, id: String) -> Resp<()> {
    let r = desmontar_perfil(&estado, &id).await;
    bandeja::refrescar(&app).await;
    r
}

async fn desmontar_perfil(estado: &State<'_, Estado>, id: &str) -> Resp<()> {
    let punto = estado
        .montados
        .lock()
        .await
        .remove(id)
        .ok_or("esa conexión no esta montada")?;

    let guard = estado.rclone.lock().await;
    if let Some(rc) = guard.as_ref() {
        rc.desmontar(&punto).await.map_err(texto)?;
    }
    Ok(())
}

/// Olvida la caché de directorios: es el botón «Actualizar» de la interfaz.
///
/// Hace falta porque el proveedor no notifica cambios, así que se monta con
/// `--poll-interval 0`: sin esto, un documento subido desde Iurefficient tarda
/// hasta `--dir-cache-time` en aparecer en la unidad.
#[tauri::command]
async fn refrescar(estado: State<'_, Estado>, id: String, ruta: String) -> Resp<()> {
    let guard = estado.rclone.lock().await;
    let rc = guard.as_ref().ok_or("no hay ninguna conexión activa")?;
    rc.refrescar(&id, &ruta).await.map_err(texto)
}

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
        .ok_or_else(|| format!("no existe la conexión '{id}'"))?;
    p.escritura = escritura;
    perfiles::upsert(p).map_err(texto)
}

/// Si IureDav arranca al iniciar sesion. Se expone desde Rust y no desde el
/// complemento en JavaScript para no anadir permisos ni dependencias al frontend.
#[tauri::command]
fn autoarranque(app: AppHandle) -> Resp<bool> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(texto)
}

#[tauri::command]
fn fijar_autoarranque(app: AppHandle, activo: bool) -> Resp<()> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if activo {
        al.enable().map_err(texto)
    } else {
        al.disable().map_err(texto)
    }
}

/// Progreso de un calentamiento, para la barra de la interfaz.
#[derive(Clone, Serialize)]
struct AvanceAnclaje {
    conexion: String,
    carpeta: String,
    archivos: usize,
    bytes: u64,
    fallidos: usize,
    terminado: bool,
}

/// Marca una carpeta como disponible sin conexion y empieza a descargarla.
///
/// `carpeta` llega como ruta absoluta desde el selector del sistema; se guarda
/// relativa al punto de montaje, que es lo unico estable entre sesiones.
#[tauri::command]
async fn anclar(
    app: AppHandle,
    estado: State<'_, Estado>,
    id: String,
    carpeta: String,
) -> Resp<()> {
    let punto = estado
        .montados
        .lock()
        .await
        .get(&id)
        .cloned()
        .ok_or("monta la conexión antes de elegir carpetas sin conexión")?;

    let relativa = std::path::Path::new(&carpeta)
        .strip_prefix(&punto)
        .map_err(|_| format!("esa carpeta no está dentro de {punto}"))?
        .to_string_lossy()
        .replace('\\', "/");

    if relativa.is_empty() {
        return Err("elige una carpeta concreta, no la raíz de la unidad".into());
    }

    let mut perfil = perfiles::buscar(&id)
        .map_err(texto)?
        .ok_or("no existe esa conexión")?;
    if !perfil.anclados.contains(&relativa) {
        perfil.anclados.push(relativa.clone());
        perfiles::upsert(perfil).map_err(texto)?;
    }

    calentar_en_segundo_plano(app, id, punto, relativa);
    Ok(())
}

#[tauri::command]
async fn desanclar(id: String, ruta: String) -> Resp<()> {
    let mut perfil = perfiles::buscar(&id)
        .map_err(texto)?
        .ok_or("no existe esa conexión")?;
    perfil.anclados.retain(|r| r != &ruta);
    perfiles::upsert(perfil).map_err(texto)
}

/// Descarga una carpeta entera en segundo plano.
///
/// Va en un hilo aparte porque recorre y lee el arbol con llamadas bloqueantes, y
/// en una carpeta grande eso son minutos: bloquear el runtime dejaria la ventana
/// congelada.
fn calentar_en_segundo_plano(app: AppHandle, conexion: String, punto: String, carpeta: String) {
    tauri::async_runtime::spawn_blocking(move || {
        let ruta = std::path::PathBuf::from(&punto);
        if !anclajes::esperar_montaje(&ruta, std::time::Duration::from_secs(20)) {
            tracing::warn!(punto, "el montaje no respondió; no se puede precargar");
            return;
        }

        let emitir = |r: &Resumen, terminado: bool| {
            let _ = app.emit(
                "iuredav://anclaje",
                AvanceAnclaje {
                    conexion: conexion.clone(),
                    carpeta: carpeta.clone(),
                    archivos: r.archivos,
                    bytes: r.bytes,
                    fallidos: r.fallidos,
                    terminado,
                },
            );
        };

        // Se avisa cada 25 archivos: uno por archivo inundaria la ventana de eventos.
        let mut ultimo = 0usize;
        let resultado = anclajes::calentar(&ruta, &carpeta, |r| {
            if r.archivos >= ultimo + 25 {
                ultimo = r.archivos;
                emitir(r, false);
            }
        });

        match resultado {
            Ok(r) => {
                tracing::info!(
                    carpeta,
                    archivos = r.archivos,
                    "carpeta disponible sin conexión"
                );
                emitir(&r, true);
            }
            Err(e) => tracing::warn!(%e, carpeta, "no se pudo precargar la carpeta"),
        }
    });
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
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(Estado::default())
        .setup(|app| {
            // Higiene de arranque: si la sesión anterior murió sin desmontar, esa
            // carpeta no se puede abrir ni volver a montar hasta que se suelte.
            for nombre in perfiles::limpiar_huerfanos() {
                tracing::info!(conexion = %nombre, "soltado un montaje de una sesión anterior");
            }

            // Si el escritorio no ofrece bandeja, la aplicación sigue siendo
            // perfectamente usable desde su ventana: no es motivo para no arrancar.
            match bandeja::instalar(app.handle()) {
                Ok(()) => app
                    .state::<Estado>()
                    .hay_bandeja
                    .store(true, Ordering::Relaxed),
                Err(e) => tracing::warn!(
                    %e,
                    "sin icono de bandeja; cerrar la ventana terminara el programa"
                ),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            listar_conexiones,
            probar,
            resondear,
            guardar_conexion,
            olvidar_conexion,
            montar,
            desmontar,
            refrescar,
            punto_sugerido,
            comprobar_sistema,
            nombre_destino,
            cambiar_modo,
            listar_presets,
            autoarranque,
            fijar_autoarranque,
            anclar,
            desanclar,
        ])
        .on_window_event(|ventana, evento| {
            // Cerrar la ventana esconde, no termina. Lo normal en esta aplicación es
            // montar al arrancar el equipo y no volver a abrir la ventana en semanas;
            // que cerrarla desmontara seria una sorpresa desagradable. Para terminar
            // de verdad esta "Salir" en la bandeja, que si desmonta.
            if let tauri::WindowEvent::CloseRequested { api, .. } = evento {
                let hay_bandeja = ventana
                    .state::<Estado>()
                    .hay_bandeja
                    .load(Ordering::Relaxed);

                if hay_bandeja {
                    api.prevent_close();
                    let _ = ventana.hide();
                } else {
                    // Sin bandeja no hay otra forma de volver, ni de salir: se cierra
                    // de verdad, desmontando antes.
                    api.prevent_close();
                    let app = ventana.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        apagar_todo(&app).await;
                        app.exit(0);
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
            limites: vec!["DELETE".into()],
        };
        let j = serde_json::to_value(&v).unwrap();
        assert_eq!(
            j["preset"], "iurefficient",
            "el perfil se aplana en la vista"
        );
        assert_eq!(j["montado"], false);
        assert_eq!(j["limites"][0], "DELETE");
        // Y no puede llevar la contrasena, que vive en el llavero.
        assert!(j.get("password").is_none());
    }
}
