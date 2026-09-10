// En Windows, evita que se abra una consola detras de la ventana en modo release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    iuredav_app::run()
}
