use crate::db::Database;
use crate::u_stats::{compute, format_result, KernelType};

pub struct UStatApp {
    input_text: String,
    db: Database,
    current_values: Vec<f64>,
    selected_kernel: KernelType,
    result_text: String,
    status_message: String,
}

impl UStatApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let db = Database::new("u_statistics.db").expect("Failed to open database");
        let values = db.get_all_values().unwrap_or_default();
        let mut app = Self {
            input_text: String::new(),
            db,
            current_values: values.clone(),
            selected_kernel: KernelType::Variance,
            result_text: String::new(),
            status_message: format!("Loaded {} values from database", values.len()),
        };
        app.compute_and_update();
        app
    }

    fn compute_and_update(&mut self) {
        if self.current_values.is_empty() {
            self.result_text = "No data. Enter values and click Add.".to_string();
            return;
        }
        self.result_text = format_result(&self.current_values, self.selected_kernel);
    }

    fn add_value(&mut self) {
        if let Ok(val) = self.input_text.trim().parse::<f64>() {
            if let Err(e) = self.db.insert_value(val) {
                self.status_message = format!("DB Error: {}", e);
                return;
            }
            self.current_values.push(val);
            self.input_text.clear();
            self.status_message = format!("Added value. Total: {}", self.current_values.len());
            self.compute_and_update();
        } else {
            self.status_message = "Invalid number format".to_string();
        }
    }

    fn load_from_db(&mut self) {
        match self.db.get_all_values() {
            Ok(values) => {
                self.current_values = values;
                self.status_message = format!("Loaded {} values from DB", self.current_values.len());
                self.compute_and_update();
            }
            Err(e) => {
                self.status_message = format!("DB Error: {}", e);
            }
        }
    }

    fn clear_db(&mut self) {
        if let Err(e) = self.db.clear_all() {
            self.status_message = format!("DB Error: {}", e);
            return;
        }
        self.current_values.clear();
        self.result_text = "Database cleared.".to_string();
        self.status_message = "All records deleted".to_string();
    }

    fn delete_last(&mut self) {
        if let Err(e) = self.db.delete_last() {
            self.status_message = format!("DB Error: {}", e);
            return;
        }
        self.current_values.pop();
        self.status_message = format!("Deleted last value. Total: {}", self.current_values.len());
        self.compute_and_update();
    }
}

impl eframe::App for UStatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("U-Statistics Estimation App");
            ui.separator();

            // --- Input Section ---
            ui.group(|ui| {
                ui.label("Data Input");
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    ui.text_edit_singleline(&mut self.input_text)
                        .lost_focus()
                        .then(|| {
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.add_value();
                            }
                        });
                    if ui.button("Add").clicked() {
                        self.add_value();
                    }
                    if ui.button("Delete Last").clicked() {
                        self.delete_last();
                    }
                });
            });

            ui.separator();

            // --- Kernel Selection ---
            ui.group(|ui| {
                ui.label("Select U-Statistic Kernel:");
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.selected_kernel,
                        KernelType::Mean,
                        "Mean (m=1)",
                    );
                    ui.selectable_value(
                        &mut self.selected_kernel,
                        KernelType::Variance,
                        "Variance (m=2)",
                    );
                    ui.selectable_value(
                        &mut self.selected_kernel,
                        KernelType::GiniMeanDifference,
                        "Gini MD (m=2)",
                    );
                    ui.selectable_value(
                        &mut self.selected_kernel,
                        KernelType::MannWhitney,
                        "Mann-Whitney (m=2)",
                    );
                });
                if ui.button("Compute").clicked() {
                    self.compute_and_update();
                }
            });

            ui.separator();

            // --- Data Display ---
            ui.group(|ui| {
                ui.label(format!("Stored Values (n={}):", self.current_values.len()));
                let values_str = self
                    .current_values
                    .iter()
                    .map(|v| format!("{:.4}", v))
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.monospace(&values_str);
            });

            ui.separator();

            // --- Results ---
            ui.group(|ui| {
                ui.label("U-Statistic Result:");
                ui.text_edit_multiline(&mut self.result_text)
                    .desired_rows(15)
                    .desired_width(f32::INFINITY);
            });

            ui.separator();

            // --- DB Controls ---
            ui.horizontal(|ui| {
                if ui.button("Reload from DB").clicked() {
                    self.load_from_db();
                }
                if ui.button("Clear DB").clicked() {
                    self.clear_db();
                }
            });

            ui.separator();
            ui.label(&self.status_message);
        });
    }
}
