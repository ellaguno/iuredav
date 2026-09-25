//! Nucleo de IureDav.
//!
//! IureDav monta una instancia de Iurefficient como una unidad del sistema, al
//! estilo de Mountain Duck. La particularidad del proyecto es el servidor: un
//! WsgiDAV con un proveedor propio sobre una base de documentos, que **anuncia
//! capacidades que no tiene**. Todo el diseno gira alrededor de eso:
//!
//! * [`probe`] descubre lo que el servidor hace de verdad, probandolo.
//! * [`caps`] convierte esa medicion en flags de rclone, sin listas fijas.
//! * [`errors`] traduce los choques contra esos limites a mensajes comprensibles.
//! * [`rclone`] supervisa el sidecar que realiza el montaje.
//! * [`plataforma`] comprueba que la maquina pueda montar, antes de intentarlo.
//! * [`anclajes`] deja carpetas disponibles sin conexion calentando la cache.
//! * [`ajustes`] guarda las preferencias que no son de ninguna conexion.
//! * [`actualizaciones`] avisa de versiones nuevas, sin instalarlas.

/// El idioma de los mensajes, el mismo que el del conector: la app lo fija al
/// arrancar y al cambiar el ajuste, y la CLI con el del sistema.
pub use iurefficient_connect::lang;

/// Como `iurefficient_connect::tr!`, pero con el idioma explicito. Sirve donde
/// el texto se prueba: el idioma global lo comparten todas las pruebas a la vez.
#[macro_export]
macro_rules! tr_en {
    ($idioma:expr, $en:literal, $es:literal $(, $arg:expr)* $(,)?) => {
        match $idioma {
            $crate::lang::Lang::En => format!($en $(, $arg)*),
            $crate::lang::Lang::Es => format!($es $(, $arg)*),
        }
    };
}

pub mod actualizaciones;
pub mod ajustes;
pub mod anclajes;
pub mod caps;
pub mod cuenta;
pub mod dav;
pub mod errors;
pub mod perfiles;
pub mod plataforma;
pub mod presets;
pub mod probe;
pub mod rclone;
pub mod secretos;

pub use caps::{opciones_de_montaje, MountOptions, OpcionesRclone, ServerCapabilities, Verdict};
pub use perfiles::Perfil;
pub use presets::Preset;
pub use probe::Probe;
