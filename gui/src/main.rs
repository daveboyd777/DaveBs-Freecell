//! Scaffolding for issue #8 (egui/eframe desktop app sharing the same
//! Store) and issue #9 (the same app cross-compiled to WASM).
//!
//! This is a minimal placeholder proving the workspace wiring -- a real
//! `Store`, a real eframe app -- not the actual card rendering or move
//! input, which land in #8.

use eframe::egui;
use freecell::Store;

struct FreecellApp {
    store: Store,
}

impl Default for FreecellApp {
    fn default() -> Self {
        Self {
            store: Store::new(617),
        }
    }
}

impl eframe::App for FreecellApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("DaveB's Freecell — GUI scaffold (issue #8)");
        ui.label(format!("Deal #{}", self.store.game().seed().unwrap_or(0)));
        ui.label(format!(
            "Moves played: {}",
            self.store.game().moves_played()
        ));
        ui.label("Card rendering and move input land in issue #8.");
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "DaveB's Freecell",
        options,
        Box::new(|_cc| Ok(Box::new(FreecellApp::default()))),
    )
}
