fn main() {
    // El triple del objetivo hace falta en tiempo de ejecucion para localizar el
    // rclone empaquetado durante el desarrollo.
    println!(
        "cargo:rustc-env=IUREDAV_TARGET={}",
        std::env::var("TARGET").unwrap_or_default()
    );
    println!("cargo:rerun-if-changed=build.rs");
}
