mod analysis;
mod app;
mod csv_io;
mod db;
mod plot;
mod two_sample;
mod u_stats;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "U-Statistics Advanced Analyzer",
        options,
        Box::new(|cc| Ok(Box::new(app::UStatAdvancedApp::new(cc)))),
    )
}
