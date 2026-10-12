//! High-performance rendering using egui primitives with render interpolation.

use crate::physics::{Bounds, ParticleSoA, PhysicsWorld};

pub struct RenderState {
    pub mesh: egui::Mesh,
}

impl RenderState {
    pub fn new() -> Self {
        Self {
            mesh: egui::Mesh::default(),
        }
    }
}

pub fn draw_world(ui: &mut egui::Ui, world: &PhysicsWorld, render_state: &mut RenderState, alpha: f32) {
    let desired = egui::vec2(
        world.config.bounds.width() as f32,
        world.config.bounds.height() as f32,
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Background
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(15, 15, 22));
    // Border
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 120, 150)));

    // Grid (sparse for performance)
    let grid_step = 100.0f32;
    let mut x = rect.min.x;
    while x < rect.max.x {
        painter.line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(40, 40, 55)),
        );
        x += grid_step;
    }
    let mut y = rect.min.y;
    while y < rect.max.y {
        painter.line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(40, 40, 55)),
        );
        y += grid_step;
    }

    // Particles with render interpolation
    let particles = &world.particles;
    let b = world.config.bounds;
    let scale_x = rect.width() / b.width() as f32;
    let scale_y = rect.height() / b.height() as f32;

    for i in 0..particles.count {
        // Interpolate between old and current position using alpha
        let vx = (particles.pos_x[i] - particles.old_x[i]) as f32;
        let vy = (particles.pos_y[i] - particles.old_y[i]) as f32;
        let render_x = particles.pos_x[i] as f32 + vx * alpha;
        let render_y = particles.pos_y[i] as f32 + vy * alpha;

        let sx = rect.min.x + (render_x - b.min_x as f32) * scale_x;
        let sy = rect.min.y + (render_y - b.min_y as f32) * scale_y;
        let r = (particles.radius[i] as f32).max(2.0);
        let color = egui::Color32::from_rgb(particles.color_r[i], particles.color_g[i], particles.color_b[i]);

        painter.circle_filled(egui::pos2(sx, sy), r, color);
    }

    // Velocity indicators
    for i in 0..particles.count {
        let vx = (particles.pos_x[i] - particles.old_x[i]) as f32 * 10.0;
        let vy = (particles.pos_y[i] - particles.old_y[i]) as f32 * 10.0;
        if vx * vx + vy * vy > 2.0 {
            let sx = rect.min.x + (particles.pos_x[i] as f32 - b.min_x as f32) * scale_x;
            let sy = rect.min.y + (particles.pos_y[i] as f32 - b.min_y as f32) * scale_y;
            painter.line_segment(
                [egui::pos2(sx, sy), egui::pos2(sx + vx, sy + vy)],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 80)),
            );
        }
    }
}

pub fn screen_to_world(rect: egui::Rect, bounds: &Bounds, pos: egui::Pos2) -> (f64, f64) {
    let wx = bounds.min_x + ((pos.x - rect.min.x) / rect.width()) as f64 * bounds.width();
    let wy = bounds.min_y + ((pos.y - rect.min.y) / rect.height()) as f64 * bounds.height();
    (wx, wy)
}