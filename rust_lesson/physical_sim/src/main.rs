mod app;
mod physics;
mod renderer;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native(
        "2D Physics Simulation",
        options,
        Box::new(|cc| Ok(Box::new(app::PhysicsApp::new(cc)))),
    )
}
