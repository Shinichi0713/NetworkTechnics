mod app;
mod db;
mod u_stats;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "U-Statistics App",
        options,
        Box::new(|cc| Ok(Box::new(app::UStatApp::new(cc)))),
    )
}
