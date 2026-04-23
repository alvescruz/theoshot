pub mod app;
pub mod components;
pub mod painter;
pub mod renderer;
pub mod types;
pub mod utils;

pub use app::SelectionApp;

pub fn run_ui(image_data: Option<Vec<u8>>, width: u32, height: u32) {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };
    eframe::run_native(
        "theoshot",
        options,
        Box::new(|_cc| Ok(Box::new(SelectionApp::new(image_data, width, height)))),
    )
    .unwrap();
}
