//! Icono de bandeja del sistema.
//!
//! Una aplicacion de este tipo vive en la bandeja, no en una ventana: lo normal es
//! montar al arrancar el equipo y no volver a abrir la ventana en semanas. Por eso
//! cerrar la ventana **no** desmonta ni termina el programa; solo la esconde. Para
//! terminar de verdad esta "Salir" en el menu, que ademas desmonta.

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use std::collections::HashMap;

use iuredav_core::perfiles;

use crate::Estado;

const ID_BANDEJA: &str = "principal";
/// Prefijo de los elementos que representan una conexion.
const PREFIJO: &str = "conexion:";

/// Construye el menu con una entrada por conexion guardada.
///
/// El conjunto de montajes se recibe hecho a proposito: tomarlo aqui obligaria a
/// bloquear un mutex asincrono, y hacerlo desde dentro del runtime provoca panico,
/// no un error. Quien llama ya lo tiene.
fn menu(app: &AppHandle, montados: &HashMap<String, String>) -> tauri::Result<Menu<Wry>> {
    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<Wry>>> = Vec::new();

    items.push(Box::new(MenuItem::with_id(
        app,
        "mostrar",
        "Abrir IureDav",
        true,
        None::<&str>,
    )?));
    items.push(Box::new(PredefinedMenuItem::separator(app)?));

    // Las conexiones guardadas, para poder montarlas sin abrir la ventana, que es
    // la razon de ser de la bandeja.
    let perfiles = perfiles::cargar().unwrap_or_default();

    if perfiles.is_empty() {
        let vacio = MenuItem::with_id(app, "vacio", "No hay conexiones", false, None::<&str>)?;
        items.push(Box::new(vacio));
    } else {
        for p in &perfiles {
            let esta = montados.contains_key(&p.id);
            let texto = if esta {
                format!("Desmontar {}", p.nombre)
            } else {
                format!("Montar {}", p.nombre)
            };
            items.push(Box::new(MenuItem::with_id(
                app,
                format!("{PREFIJO}{}", p.id),
                texto,
                true,
                None::<&str>,
            )?));
        }
    }

    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(MenuItem::with_id(
        app,
        "salir",
        "Salir",
        true,
        None::<&str>,
    )?));

    let refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = items.iter().map(|b| b.as_ref()).collect();
    Menu::with_items(app, &refs)
}

/// Rehace el menu tras montar o desmontar, para que los textos digan la verdad.
pub async fn refrescar(app: &AppHandle) {
    let montados = app.state::<Estado>().montados.lock().await.clone();
    if let Some(bandeja) = app.tray_by_id(ID_BANDEJA) {
        match menu(app, &montados) {
            Ok(m) => {
                let _ = bandeja.set_menu(Some(m));
            }
            Err(e) => tracing::warn!(%e, "no se pudo rehacer el menu de la bandeja"),
        }
    }
}

pub fn instalar(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(ID_BANDEJA)
        // Icono propio y **redondo**, no el de la ventana: en la bandeja convive
        // con los iconos del sistema, que en los tres escritorios son circulares,
        // y un cuadrado —aunque tenga las esquinas redondeadas— canta al lado.
        // Va embebido en el binario, que es lo que espera la bandeja: una ruta a
        // disco se rompe en cuanto el paquete coloca los recursos en otro sitio.
        .icon(tauri::include_image!("icons/bandeja.png"))
        .tooltip("IureDav")
        // En Windows y Linux, el clic izquierdo abre la ventana y el derecho el
        // menu, que es lo que la gente espera en cada sistema.
        .show_menu_on_left_click(false)
        // Al instalar no hay nada montado todavia.
        .menu(&menu(app, &HashMap::new())?)
        .on_menu_event(al_elegir)
        .on_tray_icon_event(|bandeja, evento| {
            if let TrayIconEvent::Click { button, .. } = evento {
                if button == tauri::tray::MouseButton::Left {
                    mostrar_ventana(bandeja.app_handle());
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn mostrar_ventana(app: &AppHandle) {
    if let Some(v) = app.get_webview_window("main") {
        let _ = v.show();
        let _ = v.unminimize();
        let _ = v.set_focus();
    }
}

fn al_elegir(app: &AppHandle, evento: MenuEvent) {
    let id = evento.id().0.clone();

    match id.as_str() {
        "mostrar" => mostrar_ventana(app),
        "salir" => {
            // Salir de verdad si desmonta: dejar puntos de montaje colgados obliga
            // al usuario a arreglarlo desde una terminal.
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::apagar_todo(&app).await;
                app.exit(0);
            });
        }
        otro if otro.starts_with(PREFIJO) => {
            let perfil = otro.trim_start_matches(PREFIJO).to_string();
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::alternar_montaje(&app, &perfil).await {
                    tracing::warn!(%e, perfil, "no se pudo cambiar el montaje desde la bandeja");
                }
                refrescar(&app).await;
            });
        }
        _ => {}
    }
}
