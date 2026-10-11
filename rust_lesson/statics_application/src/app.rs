use crate::analysis;
use crate::csv_io;
use crate::db::Database;
use crate::plot;
use crate::two_sample;
use crate::u_stats::{compute, format_result, KernelType};

enum Tab {
    Data,
    OneSample,
    TwoSample,
    Hoeffding,
    Bootstrap,
    Plot,
}

pub struct UStatAdvancedApp {
    tab: Tab,
    db: Database,
    group_a: Vec<f64>,
    group_b: Vec<f64>,
    input_a: String,
    input_b: String,
    selected_kernel: KernelType,
    result_text: String,
    status: String,
    bootstrap_b: usize,
    bootstrap_result: Option<analysis::BootstrapResult>,
    hoeffding_result: Option<analysis::HoeffdingDecomp>,
    two_sample_result: Option<two_sample::TwoSampleResult>,
    csv_path: String,
    hist_bins: usize,
}

impl UStatAdvancedApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let db = Database::new("u_statistics_advanced.db").expect("Failed to open DB");
        let group_a = db.get_all_a().unwrap_or_default();
        let group_b = db.get_all_b().unwrap_or_default();
        let mut app = Self {
            tab: Tab::Data,
            db,
            group_a,
            group_b,
            input_a: String::new(),
            input_b: String::new(),
            selected_kernel: KernelType::Variance,
            result_text: String::new(),
            status: "Ready".to_string(),
            bootstrap_b: 1000,
            bootstrap_result: None,
            hoeffding_result: None,
            two_sample_result: None,
            csv_path: "data.csv".to_string(),
            hist_bins: 20,
        };
        app.refresh_results();
        app
    }

    fn refresh_results(&mut self) {
        if !self.group_a.is_empty() {
            self.result_text = format_result(&self.group_a, self.selected_kernel);
        } else {
            self.result_text = "No data in Group A.".to_string();
        }
    }

    fn add_a(&mut self) {
        if let Ok(v) = self.input_a.trim().parse::<f64>() {
            let _ = self.db.insert_value_a(v);
            self.group_a.push(v);
            self.input_a.clear();
            self.status = format!("Group A: {} values", self.group_a.len());
            self.refresh_results();
        } else {
            self.status = "Invalid number (Group A)".to_string();
        }
    }

    fn add_b(&mut self) {
        if let Ok(v) = self.input_b.trim().parse::<f64>() {
            let _ = self.db.insert_value_b(v);
            self.group_b.push(v);
            self.input_b.clear();
            self.status = format!("Group B: {} values", self.group_b.len());
        } else {
            self.status = "Invalid number (Group B)".to_string();
        }
    }

    fn run_bootstrap(&mut self) {
        if self.group_a.len() < 2 { 
            self.status = "Need >= 2 values for bootstrap".to_string(); 
            return; 
        }
        self.status = "Running bootstrap...".to_string();
        self.bootstrap_result = analysis::bootstrap(&self.group_a, self.selected_kernel, self.bootstrap_b);
        if let Some(ref res) = self.bootstrap_result {
            self.status = format!("Bootstrap done: mean={:.6}, se={:.6}", res.mean, res.std_error);
        }
    }

    fn run_hoeffding(&mut self) {
        if self.group_a.len() < 3 {
            self.status = "Need >= 3 values for Hoeffding decomposition".to_string();
            return;
        }
        self.hoeffding_result = analysis::hoeffding_decompose(&self.group_a, self.selected_kernel);
        if let Some(ref res) = self.hoeffding_result {
            self.status = format!("Hoeffding: linear={:.6}, quad={:.6}", res.linear_component, res.quadratic_component);
        }
    }

    fn run_two_sample(&mut self) {
        if self.group_a.is_empty() || self.group_b.is_empty() {
            self.status = "Both groups must have data".to_string();
            return;
        }
        self.two_sample_result = two_sample::mann_whitney_u(&self.group_a, &self.group_b);
        if let Some(ref res) = self.two_sample_result {
            self.status = format!("Mann-Whitney theta={:.4}, p={:.4}", res.theta, res.p_value_two_sided);
        }
    }

    fn load_csv_to_a(&mut self) {
        match csv_io::load_csv(&self.csv_path) {
            Ok(vals) => {
                for v in &vals { let _ = self.db.insert_value_a(*v); }
                self.group_a = self.db.get_all_a().unwrap_or_default();
                self.status = format!("Loaded {} values from CSV to Group A", vals.len());
                self.refresh_results();
            }
            Err(e) => self.status = format!("CSV Error: {}", e),
        }
    }

    fn save_a_to_csv(&mut self) {
        match csv_io::save_csv(&self.csv_path, &self.group_a) {
            Ok(_) => self.status = format!("Saved Group A to {}", self.csv_path),
            Err(e) => self.status = format!("CSV Error: {}", e),
        }
    }
}

impl eframe::App for UStatAdvancedApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Data, "📁 Data");
                ui.selectable_value(&mut self.tab, Tab::OneSample, "📊 One-Sample U-Stat");
                ui.selectable_value(&mut self.tab, Tab::TwoSample, "⚖ Two-Sample Test");
                ui.selectable_value(&mut self.tab, Tab::Hoeffding, "🔬 Hoeffding Decomp");
                ui.selectable_value(&mut self.tab, Tab::Bootstrap, "🔄 Bootstrap");
                ui.selectable_value(&mut self.tab, Tab::Plot, "📈 Plots");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.tab {
                Tab::Data => self.ui_data_tab(ui),
                Tab::OneSample => self.ui_one_sample_tab(ui),
                Tab::TwoSample => self.ui_two_sample_tab(ui),
                Tab::Hoeffding => self.ui_hoeffding_tab(ui),
                Tab::Bootstrap => self.ui_bootstrap_tab(ui),
                Tab::Plot => self.ui_plot_tab(ui),
            }
            ui.separator();
            ui.label(&self.status);
        });
    }
}

impl UStatAdvancedApp {
    fn ui_data_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Data Management");
        ui.separator();

        ui.group(|ui| {
            ui.label("Group A (Primary Sample)");
            ui.horizontal(|ui| {
                ui.label("Value:");
                ui.text_edit_singleline(&mut self.input_a);
                if ui.button("Add to A").clicked() { self.add_a(); }
                if ui.button("Delete Last A").clicked() {
                    let _ = self.db.delete_last_a();
                    self.group_a.pop();
                    self.refresh_results();
                }
                if ui.button("Clear A").clicked() {
                    let _ = self.db.clear_a();
                    self.group_a.clear();
                    self.refresh_results();
                }
            });
            ui.monospace(format!("A: {:?}", &self.group_a.iter().map(|v| format!("{:.3}", v)).collect::<Vec<_>>().join(", ")));
        });

        ui.group(|ui| {
            ui.label("Group B (Secondary Sample for Two-Sample Tests)");
            ui.horizontal(|ui| {
                ui.label("Value:");
                ui.text_edit_singleline(&mut self.input_b);
                if ui.button("Add to B").clicked() { self.add_b(); }
                if ui.button("Delete Last B").clicked() {
                    let _ = self.db.delete_last_b();
                    self.group_b.pop();
                }
                if ui.button("Clear B").clicked() {
                    let _ = self.db.clear_b();
                    self.group_b.clear();
                }
            });
            ui.monospace(format!("B: {:?}", &self.group_b.iter().map(|v| format!("{:.3}", v)).collect::<Vec<_>>().join(", ")));
        });

        ui.group(|ui| {
            ui.label("CSV Import / Export");
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.text_edit_singleline(&mut self.csv_path);
                if ui.button("Load CSV -> A").clicked() { self.load_csv_to_a(); }
                if ui.button("Save A -> CSV").clicked() { self.save_a_to_csv(); }
            });
        });
    }

    fn ui_one_sample_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("One-Sample U-Statistics Estimation");
        ui.separator();

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.selected_kernel, KernelType::Mean, "Mean");
            ui.selectable_value(&mut self.selected_kernel, KernelType::Variance, "Variance");
            ui.selectable_value(&mut self.selected_kernel, KernelType::GiniMeanDifference, "Gini MD");
            ui.selectable_value(&mut self.selected_kernel, KernelType::MannWhitney, "Mann-Whitney Type");
        });

        if ui.button("Compute").clicked() {
            self.refresh_results();
        }

        ui.separator();
        egui::TextEdit::multiline(&mut self.result_text)
            .desired_rows(20)
            .desired_width(f32::INFINITY)
            .show(ui);

        if let Some((jack_mean, jack_se)) = analysis::jackknife(&self.group_a, self.selected_kernel) {
            ui.separator();
            ui.label(format!("Jackknife: mean={:.6}, SE={:.6}", jack_mean, jack_se));
        }
    }

    fn ui_two_sample_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Two-Sample U-Statistics Test");
        ui.separator();

        ui.label(format!("Group A: n={} | Group B: n={}", self.group_a.len(), self.group_b.len()));

        if ui.button("Run Mann-Whitney U Test").clicked() {
            self.run_two_sample();
        }

        if let Some(ref res) = self.two_sample_result {
            ui.separator();
            ui.group(|ui| {
                ui.label("Mann-Whitney U Test Result");
                ui.label(format!("U-statistic (theta) = {:.6}", res.theta));
                ui.label(format!("Standard error = {:.6}", res.std_error));
                ui.label(format!("Z-score = {:.6}", res.z_score));
                ui.label(format!("p-value (two-sided) = {:.6}", res.p_value_two_sided));
                ui.label(format!("p-value (P(X<=Y) > 0.5) = {:.6}", res.p_value_greater));
                ui.label(format!("p-value (P(X<=Y) < 0.5) = {:.6}", res.p_value_less));
            });
        }
    }

    fn ui_hoeffding_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Hoeffding Decomposition (Degree-2 Kernels)");
        ui.separator();

        ui.label("Decomposes U_n - theta into linear + quadratic components:");
        ui.monospace("U_n - theta = (2/n) * sum_i h_1(X_i) + residual");

        if ui.button("Compute Hoeffding Decomposition").clicked() {
            self.run_hoeffding();
        }

        if let Some(ref res) = self.hoeffding_result {
            ui.separator();
            ui.label(format!("theta (U_n) = {:.6}", res.theta));
            ui.label(format!("Linear component (2/n * sum h_1) = {:.6}", res.linear_component));
            ui.label(format!("Quadratic residual component = {:.6}", res.quadratic_component));
            ui.label(format!("Ratio linear / |quad| = {:.3}", (res.linear_component / res.quadratic_component.abs().max(1e-12)).abs()));

            ui.separator();
            ui.label("h_1(X_i) values (first 10):");
            for (i, &v) in res.h1_values.iter().take(10).enumerate() {
                ui.monospace(format!("  h_1(X_{}) = {:.6}", i + 1, v));
            }
        }
    }

    fn ui_bootstrap_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Bootstrap Analysis for U-Statistics");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Bootstrap replications (B):");
            ui.add(egui::DragValue::new(&mut self.bootstrap_b).speed(100).clamp_range(100..=10000));
        });

        if ui.button("Run Bootstrap").clicked() {
            self.run_bootstrap();
        }

        if let Some(ref res) = self.bootstrap_result {
            ui.separator();
            ui.label(format!("Bootstrap mean = {:.6}", res.mean));
            ui.label(format!("Bootstrap SE = {:.6}", res.std_error));
            ui.label(format!("Bootstrap bias = {:.6}", res.bias));
            ui.label(format!("95% Bootstrap CI = [{:.6}, {:.6}]", res.ci_lower, res.ci_upper));

            ui.separator();
            ui.label("Bootstrap distribution histogram:");
            plot::draw_histogram(ui, &res.estimates, 30, egui::Color32::from_rgb(100, 200, 100));
        }
    }

    fn ui_plot_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Distribution Visualization");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Histogram bins:");
            ui.add(egui::DragValue::new(&mut self.hist_bins).speed(1).clamp_range(5..=100));
        });

        if !self.group_a.is_empty() {
            ui.label(format!("Group A histogram (n={}):", self.group_a.len()));
            plot::draw_histogram(ui, &self.group_a, self.hist_bins, egui::Color32::from_rgb(100, 150, 255));
        }

        if !self.group_b.is_empty() {
            ui.label(format!("Group B histogram (n={}):", self.group_b.len()));
            plot::draw_histogram(ui, &self.group_b, self.hist_bins, egui::Color32::from_rgb(255, 150, 100));
        }

        if !self.group_a.is_empty() {
            ui.separator();
            ui.label("Q-Q plot for Group A (vs Normal):");
            plot::draw_qq_plot(ui, &self.group_a, egui::Color32::from_rgb(200, 100, 255));
        }

        if let Some(ref res) = self.bootstrap_result {
            ui.separator();
            ui.label("Bootstrap estimates histogram:");
            plot::draw_histogram(ui, &res.estimates, self.hist_bins, egui::Color32::from_rgb(100, 200, 100));
        }
    }
}
