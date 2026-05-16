mod app;
mod core;
mod map;
mod renderer;
mod map_loader;
mod painter;

fn main() -> eframe::Result<()> {
    env_logger::init();
    log::info!("World Smith 起動中...");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 900.0])
            .with_title("World Smith - HoI4 Map Architect"),
        ..Default::default()
    };

    eframe::run_native(
        "World Smith",
        native_options,
        Box::new(|cc| Ok(Box::new(app::WorldSmithApp::new(cc)))),
    )
}
