//! Rendering utilities for the physics simulation using egui primitives.

use crate::physics::{Bounds, Particle, PhysicsWorld, Vec2};

pub fn draw_world(ui: &mut egui::Ui, world: &PhysicsWorld, scale: f32) {
    let available = ui.available_size();
    let desired = egui::vec2(
        world.config.bounds.width() as f32,
        world.config.bounds.height() as f32,
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Background
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 30));

    // Bounds border
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(150, 150, 180)));

    // Grid lines
    let grid_step = 50.0f32;
    let mut x = rect.min.x;
    while x < rect.max.x {
        painter.line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 50, 70)),
        );
        x += grid_step;
    }
    let mut y = rect.min.y;
    while y < rect.max.y {
        painter.line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 50, 70)),
        );
        y += grid_step;
    }

    // Particles
    for p in &world.particles {
        let center = world_to_screen(rect, &world.config.bounds, p.pos);
        let r = (p.radius as f32).max(2.0) * scale;
        let color = egui::Color32::from_rgb(p.color.0, p.color.1, p.color.2);
        painter.circle_filled(center, r, color);
        // Velocity indicator
        let vel = p.velocity(1.0 / 60.0);
        if vel.length() > 10.0 {
            let tip = world_to_screen(rect, &world.config.bounds, p.pos.add(Vec2::new(vel.x * 0.01, vel.y * 0.01)));
            painter.line_segment(
                [center, tip],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 120)),
            );
        }
    }

    // Interaction: spawn on click
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let world_pos = screen_to_world(rect, &world.config.bounds, pos);
            // Store in app via external handling (see app.rs)
        }
    }
}

fn world_to_screen(rect: egui::Rect, bounds: &Bounds, pos: Vec2) -> egui::Pos2 {
    let sx = rect.min.x + ((pos.x - bounds.min_x) / bounds.width()) as f32 * rect.width();
    let sy = rect.min.y + ((pos.y - bounds.min_y) / bounds.height()) as f32 * rect.height();
    egui::pos2(sx, sy)
}

fn screen_to_world(rect: egui::Rect, bounds: &Bounds, pos: egui::Pos2) -> Vec2 {
    let wx = bounds.min_x + ((pos.x - rect.min.x) / rect.width()) as f64 * bounds.width();
    let wy = bounds.min_y + ((pos.y - rect.min.y) / rect.height()) as f64 * bounds.height();
    Vec2::new(wx, wy)
}
