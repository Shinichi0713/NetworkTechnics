use crate::physics::{Bounds, ForceMode, PhysicsConfig, PhysicsWorld, Vec2};
use crate::renderer;

pub struct PhysicsApp {
    pub world: PhysicsWorld,
    pub spawn_count: usize,
    pub selected_mode: ForceMode,
    pub scale: f32,
    pub show_stats: bool,
    pub click_spawn_vel_x: f64,
    pub click_spawn_vel_y: f64,
    pub click_spawn_mass: f64,
    pub click_spawn_radius: f64,
    pub last_frame_time: f64,
    pub fps_history: Vec<f64>,
}

impl PhysicsApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut world = PhysicsWorld::new();
        world.spawn_random(80);
        Self {
            world,
            spawn_count: 50,
            selected_mode: ForceMode::GravityDown,
            scale: 1.0,
            show_stats: true,
            click_spawn_vel_x: 0.0,
            click_spawn_vel_y: 0.0,
            click_spawn_mass: 2.0,
            click_spawn_radius: 8.0,
            last_frame_time: 0.0,
            fps_history: Vec::with_capacity(60),
        }
    }
}

impl eframe::App for PhysicsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt) as f64;
        self.last_frame_time = dt;
        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        self.fps_history.push(fps);
        if self.fps_history.len() > 60 { self.fps_history.remove(0); }

        // Update physics
        self.world.config.response_mode = self.selected_mode;
        self.world.update(dt.min(0.05));

        egui::SidePanel::left("control_panel").min_width(250.0).show(ctx, |ui| {
            ui.heading("Physics Controls");
            ui.separator();

            // Playback
            ui.group(|ui| {
                ui.label("Playback");
                ui.horizontal(|ui| {
                    if ui.button(if self.world.paused { "▶ Play" } else { "⏸ Pause" }).clicked() {
                        self.world.paused = !self.world.paused;
                    }
                    if ui.button("⏭ Step").clicked() {
                        self.world.step_once = true;
                    }
                    if ui.button("🗑 Clear").clicked() {
                        self.world.clear();
                    }
                });
            });

            ui.separator();

            // Spawn
            ui.group(|ui| {
                ui.label("Spawn Particles");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.spawn_count).speed(1).clamp_range(1..=500));
                    if ui.button("Spawn Random").clicked() {
                        self.world.spawn_random(self.spawn_count);
                    }
                });
            });

            ui.separator();

            // Force mode
            ui.group(|ui| {
                ui.label("Force Mode");
                ui.selectable_value(&mut self.selected_mode, ForceMode::GravityDown, "Gravity Down");
                ui.selectable_value(&mut self.selected_mode, ForceMode::ZeroGravity, "Zero Gravity");
                ui.selectable_value(&mut self.selected_mode, ForceMode::MutualGravity, "Mutual Gravity (N-body)");
                ui.selectable_value(&mut self.selected_mode, ForceMode::MutualRepulsion, "Mutual Repulsion");
            });

            ui.separator();

            // Parameters
            ui.group(|ui| {
                ui.label("Parameters");
                ui.horizontal(|ui| {
                    ui.label("Gravity X:");
                    ui.add(egui::DragValue::new(&mut self.world.config.gravity.x).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Gravity Y:");
                    ui.add(egui::DragValue::new(&mut self.world.config.gravity.y).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Restitution:");
                    ui.add(egui::DragValue::new(&mut self.world.config.restitution).speed(0.05).clamp_range(0.0..=1.5));
                });
                ui.horizontal(|ui| {
                    ui.label("Friction:");
                    ui.add(egui::DragValue::new(&mut self.world.config.friction).speed(0.001).clamp_range(0.9..=1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Damping:");
                    ui.add(egui::DragValue::new(&mut self.world.config.damping).speed(0.001).clamp_range(0.9..=1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Substeps:");
                    ui.add(egui::DragValue::new(&mut self.world.config.substeps).speed(1).clamp_range(1..=32));
                });
                ui.horizontal(|ui| {
                    ui.label("Mutual Strength:");
                    ui.add(egui::DragValue::new(&mut self.world.config.mutual_strength).speed(100.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Max Speed:");
                    ui.add(egui::DragValue::new(&mut self.world.config.max_speed).speed(100.0));
                });
            });

            ui.separator();

            // Click spawn settings
            ui.group(|ui| {
                ui.label("Click-Spawn Settings");
                ui.horizontal(|ui| { ui.label("Vel X:"); ui.add(egui::DragValue::new(&mut self.click_spawn_vel_x).speed(10.0)); });
                ui.horizontal(|ui| { ui.label("Vel Y:"); ui.add(egui::DragValue::new(&mut self.click_spawn_vel_y).speed(10.0)); });
                ui.horizontal(|ui| { ui.label("Mass:"); ui.add(egui::DragValue::new(&mut self.click_spawn_mass).speed(0.5).clamp_range(0.1..=50.0)); });
                ui.horizontal(|ui| { ui.label("Radius:"); ui.add(egui::DragValue::new(&mut self.click_spawn_radius).speed(0.5).clamp_range(1.0..=50.0)); });
            });

            ui.separator();

            // Stats
            ui.checkbox(&mut self.show_stats, "Show Statistics");
            if self.show_stats {
                ui.separator();
                let n = self.world.particles.len();
                let ke = self.world.total_kinetic_energy(1.0 / 60.0);
                let mom = self.world.total_momentum(1.0 / 60.0);
                let avg_fps = if !self.fps_history.is_empty() {
                    self.fps_history.iter().sum::<f64>() / self.fps_history.len() as f64
                } else { 0.0 };
                ui.label(format!("Particles: {}", n));
                ui.label(format!("Sim Time: {:.2}s", self.world.time));
                ui.label(format!("FPS: {:.1}", avg_fps));
                ui.label(format!("Frame dt: {:.3}ms", self.last_frame_time * 1000.0));
                ui.label(format!("Total KE: {:.1}", ke));
                ui.label(format!("Momentum: ({:.1}, {:.1})", mom.x, mom.y));
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Handle click-to-spawn on the simulation area
            let desired = egui::vec2(
                self.world.config.bounds.width() as f32,
                self.world.config.bounds.height() as f32,
            );
            let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
            if response.clicked() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let wx = self.world.config.bounds.min_x + ((pointer_pos.x - rect.min.x) / rect.width()) as f64 * self.world.config.bounds.width();
                    let wy = self.world.config.bounds.min_y + ((pointer_pos.y - rect.min.y) / rect.height()) as f64 * self.world.config.bounds.height();
                    self.world.add_particle(
                        Vec2::new(wx, wy),
                        Vec2::new(self.click_spawn_vel_x, self.click_spawn_vel_y),
                        self.click_spawn_mass,
                        self.click_spawn_radius,
                    );
                }
            }

            renderer::draw_world(ui, &self.world, self.scale);
        });

        ctx.request_repaint();
    }
}
