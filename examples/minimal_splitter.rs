// REFERENCE EXAMPLE: egui's Built-in Resizable Panel (BROKEN)
//
// This demonstrates that egui 0.29's TopBottomPanel::resizable() is BROKEN.
// The splitter shows a resize cursor but won't actually move.
//
// Run with: cargo run --example minimal_splitter --release
// Try dragging the horizontal line - IT WON'T WORK!
//
// This is why we had to implement a custom splitter (see custom_splitter_test.rs)

use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Minimal Splitter Test"),
        ..Default::default()
    };

    eframe::run_simple_native("test", native_options, move |ctx, _frame| {
        // Top panel - resizable
        egui::TopBottomPanel::top("top")
            .resizable(true)
            .default_height(200.0)
            .height_range(100.0..=400.0)
            .show(ctx, |ui| {
                ui.heading("TOP PANEL");
                ui.label("Try to drag the line below");
            });

        // Bottom panel - for status
        egui::TopBottomPanel::bottom("bottom")
            .show(ctx, |ui| {
                ui.label("Status bar");
            });

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("BOTTOM PANEL");
            ui.label("Main content area");
        });
    })
}

