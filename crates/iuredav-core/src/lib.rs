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

pub mod caps;
pub mod dav;
pub mod errors;
pub mod probe;
pub mod rclone;

pub use caps::{opciones_de_montaje, MountOptions, OpcionesRclone, ServerCapabilities, Verdict};
pub use probe::Probe;
