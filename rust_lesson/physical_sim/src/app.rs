use crate::physics::{Bounds, ForceMode, PhysicsConfig, PhysicsWorld};
use crate::renderer::{draw_world, screen_to_world, RenderState};

pub struct PhysicsApp {
    pub world: PhysicsWorld,
    pub spawn_count: usize,
    pub selected_mode: ForceMode,
    pub show_stats: bool,
    pub click_spawn_vel_x: f64,
    pub click_spawn_vel_y: f64,
    pub click_spawn_mass: f64,
    pub click_spawn_radius: f64,
    pub last_frame_time: f64,
    pub fps_history: Vec<f64>,
    pub accumulator: f64,
    pub physics_dt: f64,
    pub render_state: RenderState,
    pub stats_timer: f64,
    pub cached_ke: f64,
    pub cached_px: f64,
    pub cached_py: f64,
    pub cached_fps: f64,
}

impl PhysicsApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut world = PhysicsWorld::new();
        world.spawn_random(100);
        let physics_dt = 1.0 / 120.0; // Fixed 120 Hz physics
        Self {
            world,
            spawn_count: 100,
            selected_mode: ForceMode::GravityDown,
            show_stats: true,
            click_spawn_vel_x: 0.0,
            click_spawn_vel_y: 0.0,
            click_spawn_mass: 2.0,
            click_spawn_radius: 8.0,
            last_frame_time: 0.0,
            fps_history: Vec::with_capacity(60),
            accumulator: 0.0,
            physics_dt,
            render_state: RenderState::new(),
            stats_timer: 0.0,
            cached_ke: 0.0,
            cached_px: 0.0,
            cached_py: 0.0,
            cached_fps: 0.0,
        }
    }
}

impl eframe::App for PhysicsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_dt = ctx.input(|i| i.stable_dt) as f64;
        self.last_frame_time = frame_dt;
        let fps = if frame_dt > 0.0 { 1.0 / frame_dt } else { 0.0 };
        self.fps_history.push(fps);
        if self.fps_history.len() > 60 { self.fps_history.remove(0); }

        // Fixed timestep accumulator
        if !self.world.paused {
            self.accumulator += frame_dt.min(0.05); // clamp to avoid spiral of death
            while self.accumulator >= self.physics_dt {
                self.world.update(self.physics_dt);
                self.accumulator -= self.physics_dt;
            }
        } else if self.world.step_once {
            self.world.update(self.physics_dt);
            self.world.step_once = false;
            self.accumulator = 0.0;
        }

        let alpha = (self.accumulator / self.physics_dt) as f32;

        // Throttled statistics update (every 0.2s)
        self.stats_timer += frame_dt;
        if self.stats_timer >= 0.2 {
            self.stats_timer = 0.0;
            self.cached_ke = self.world.total_kinetic_energy(self.physics_dt);
            let (px, py) = self.world.total_momentum(self.physics_dt);
            self.cached_px = px;
            self.cached_py = py;
            self.cached_fps = if !self.fps_history.is_empty() {
                self.fps_history.iter().sum::<f64>() / self.fps_history.len() as f64
            } else { 0.0 };
        }

        // Sync force mode
        self.world.config.response_mode = self.selected_mode;

        // Left control panel
        egui::SidePanel::left("control_panel").min_width(260.0).show(ctx, |ui| {
            ui.heading("Physics Controls");
            ui.separator();

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

            ui.group(|ui| {
                ui.label("Spawn Particles");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.spawn_count).speed(1).clamp_range(1..=2000));
                    if ui.button("Spawn Random").clicked() {
                        self.world.spawn_random(self.spawn_count);
                    }
                });
            });

            ui.separator();

            ui.group(|ui| {
                ui.label("Force Mode");
                ui.selectable_value(&mut self.selected_mode, ForceMode::GravityDown, "Gravity Down");
                ui.selectable_value(&mut self.selected_mode, ForceMode::ZeroGravity, "Zero Gravity");
                ui.selectable_value(&mut self.selected_mode, ForceMode::MutualGravity, "Mutual Gravity (N-body)");
                ui.selectable_value(&mut self.selected_mode, ForceMode::MutualRepulsion, "Mutual Repulsion");
            });

            ui.separator();

            ui.group(|ui| {
                ui.label("Parameters");
                ui.horizontal(|ui| {
                    ui.label("Gravity X:");
                    ui.add(egui::DragValue::new(&mut self.world.config.gravity_x).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Gravity Y:");
                    ui.add(egui::DragValue::new(&mut self.world.config.gravity_y).speed(10.0));
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
                    ui.label("Cutoff (N-body):");
                    ui.add(egui::DragValue::new(&mut self.world.config.cutoff).speed(5.0).clamp_range(10.0..=500.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Max Speed:");
                    ui.add(egui::DragValue::new(&mut self.world.config.max_speed).speed(100.0));
                });
            });

            ui.separator();

            ui.group(|ui| {
                ui.label("Click-Spawn Settings");
                ui.horizontal(|ui| { ui.label("Vel X:"); ui.add(egui::DragValue::new(&mut self.click_spawn_vel_x).speed(10.0)); });
                ui.horizontal(|ui| { ui.label("Vel Y:"); ui.add(egui::DragValue::new(&mut self.click_spawn_vel_y).speed(10.0)); });
                ui.horizontal(|ui| { ui.label("Mass:"); ui.add(egui::DragValue::new(&mut self.click_spawn_mass).speed(0.5).clamp_range(0.1..=50.0)); });
                ui.horizontal(|ui| { ui.label("Radius:"); ui.add(egui::DragValue::new(&mut self.click_spawn_radius).speed(0.5).clamp_range(1.0..=50.0)); });
            });

            ui.separator();

            ui.checkbox(&mut self.show_stats, "Show Statistics");
            if self.show_stats {
                ui.separator();
                ui.label(format!("Particles: {}", self.world.particles.count));
                ui.label(format!("Sim Time: {:.2}s", self.world.time));
                ui.label(format!("FPS: {:.1}", self.cached_fps));
                ui.label(format!("Frame dt: {:.3}ms", self.last_frame_time * 1000.0));
                ui.label(format!("Total KE: {:.1}", self.cached_ke));
                ui.label(format!("Momentum: ({:.1}, {:.1})", self.cached_px, self.cached_py));
                ui.label(format!("Physics dt: {:.3}ms", self.physics_dt * 1000.0));
                ui.label(format!("Accumulator: {:.4}", self.accumulator));
            }
        });

        // Central simulation view
        egui::CentralPanel::default().show(ctx, |ui| {
            let desired = egui::vec2(
                self.world.config.bounds.width() as f32,
                self.world.config.bounds.height() as f32,
            );
            let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
            if response.clicked() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let (wx, wy) = screen_to_world(rect, &self.world.config.bounds, pointer_pos);
                    self.world.add_particle(
                        wx, wy,
                        self.click_spawn_vel_x, self.click_spawn_vel_y,
                        self.click_spawn_mass, self.click_spawn_radius,
                    );
                }
            }
            draw_world(ui, &self.world, &mut self.render_state, alpha);
        });

        // Frame rate limiter (~120 FPS cap to reduce CPU/GPU waste)
        ctx.request_repaint_after(std::time::Duration::from_millis(8));
    }
}