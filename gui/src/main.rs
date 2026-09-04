//! Thin native/wasm entry point for the `freecell-gui` binary.
//!
//! Almost all logic lives in the `freecell_gui` library crate
//! (`src/lib.rs`), so it can also be built as a `cdylib` with its own
//! `android_main` entry point for the Android build -- see `lib.rs`'s
//! module docs.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    freecell_gui::run_native()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    freecell_gui::run_web()
}
