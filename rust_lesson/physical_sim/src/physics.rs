//! 2D particle physics engine with Verlet integration, collision, and spatial hashing.

use rand::distributions::{Distribution, Uniform};
use rand::rngs::ThreadRng;

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self { Self { x, y } }
    pub fn zero() -> Self { Self::new(0.0, 0.0) }
    pub fn add(self, other: Vec2) -> Vec2 { Vec2::new(self.x + other.x, self.y + other.y) }
    pub fn sub(self, other: Vec2) -> Vec2 { Vec2::new(self.x - other.x, self.y - other.y) }
    pub fn scale(self, s: f64) -> Vec2 { Vec2::new(self.x * s, self.y * s) }
    pub fn dot(self, other: Vec2) -> f64 { self.x * other.x + self.y * other.y }
    pub fn length_sq(self) -> f64 { self.x * self.x + self.y * self.y }
    pub fn length(self) -> f64 { self.length_sq().sqrt() }
    pub fn normalize(self) -> Vec2 {
        let len = self.length();
        if len > 1e-12 { self.scale(1.0 / len) } else { Vec2::zero() }
    }
    pub fn dist_sq(self, other: Vec2) -> f64 { self.sub(other).length_sq() }
    pub fn dist(self, other: Vec2) -> f64 { self.dist_sq(other).sqrt() }
}

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub pos: Vec2,
    pub old_pos: Vec2,
    pub acc: Vec2,
    pub mass: f64,
    pub radius: f64,
    pub id: usize,
    pub color: (u8, u8, u8),
}

impl Particle {
    pub fn new(pos: Vec2, velocity: Vec2, mass: f64, radius: f64, id: usize) -> Self {
        Self {
            old_pos: pos.sub(velocity),
            pos,
            acc: Vec2::zero(),
            mass,
            radius,
            id,
            color: (
                (id.wrapping_mul(137) % 200 + 55) as u8,
                (id.wrapping_mul(241) % 200 + 55) as u8,
                (id.wrapping_mul(89) % 200 + 55) as u8,
            ),
        }
    }

    pub fn velocity(&self, dt: f64) -> Vec2 {
        self.pos.sub(self.old_pos).scale(1.0 / dt)
    }

    pub fn kinetic_energy(&self, dt: f64) -> f64 {
        let v = self.velocity(dt);
        0.5 * self.mass * v.length_sq()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForceMode {
    GravityDown,
    ZeroGravity,
    MutualGravity,
    MutualRepulsion,
}

#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    pub gravity: Vec2,
    pub restitution: f64,
    pub friction: f64,
    pub substeps: usize,
    pub response_mode: ForceMode,
    pub mutual_strength: f64,
    pub min_dist: f64,
    pub max_speed: f64,
    pub damping: f64,
    pub bounds: Bounds,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: Vec2::new(0.0, 500.0),
            restitution: 0.9,
            friction: 0.995,
            substeps: 8,
            response_mode: ForceMode::GravityDown,
            mutual_strength: 1000.0,
            min_dist: 10.0,
            max_speed: 5000.0,
            damping: 0.98,
            bounds: Bounds::new(0.0, 0.0, 900.0, 600.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Bounds {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }
    pub fn width(&self) -> f64 { self.max_x - self.min_x }
    pub fn height(&self) -> f64 { self.max_y - self.min_y }
}

#[derive(Debug)]
pub struct PhysicsWorld {
    pub particles: Vec<Particle>,
    pub config: PhysicsConfig,
    pub time: f64,
    pub rng: ThreadRng,
    pub paused: bool,
    pub step_once: bool,
    pub spatial_grid: SpatialGrid,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        let config = PhysicsConfig::default();
        let bounds = config.bounds;
        Self {
            particles: Vec::new(),
            config,
            time: 0.0,
            rng: rand::thread_rng(),
            paused: false,
            step_once: false,
            spatial_grid: SpatialGrid::new(bounds, 40.0),
        }
    }

    pub fn add_particle(&mut self, pos: Vec2, vel: Vec2, mass: f64, radius: f64) {
        let id = self.particles.len();
        self.particles.push(Particle::new(pos, vel, mass, radius, id));
    }

    pub fn spawn_random(&mut self, count: usize) {
        let w = self.config.bounds.width();
        let h = self.config.bounds.height();
        let ux = Uniform::from(50.0..w - 50.0);
        let uy = Uniform::from(50.0..h - 50.0);
        let um = Uniform::from(1.0..5.0);
        let ur = Uniform::from(5.0..12.0);
        for _ in 0..count {
            let x = ux.sample(&mut self.rng);
            let y = uy.sample(&mut self.rng);
            let m = um.sample(&mut self.rng);
            let r = ur.sample(&mut self.rng);
            let vx = (self.rng.gen::<f64>() - 0.5) * 200.0;
            let vy = (self.rng.gen::<f64>() - 0.5) * 200.0;
            self.add_particle(Vec2::new(x, y), Vec2::new(vx, vy), m, r);
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.time = 0.0;
    }

    pub fn total_kinetic_energy(&self, dt: f64) -> f64 {
        self.particles.iter().map(|p| p.kinetic_energy(dt)).sum()
    }

    pub fn total_momentum(&self, dt: f64) -> Vec2 {
        let mut px = 0.0;
        let mut py = 0.0;
        for p in &self.particles {
            let v = p.velocity(dt);
            px += p.mass * v.x;
            py += p.mass * v.y;
        }
        Vec2::new(px, py)
    }

    pub fn update(&mut self, dt: f64) {
        if self.paused && !self.step_once {
            return;
        }
        self.step_once = false;

        let sub_dt = dt / self.config.substeps as f64;
        for _ in 0..self.config.substeps {
            self.apply_forces();
            self.update_positions(sub_dt);
            self.build_spatial_grid();
            self.solve_collisions();
            self.solve_boundary_constraints();
            self.apply_damping();
        }
        self.time += dt;
    }

    fn apply_forces(&mut self) {
        let cfg = &self.config;
        match cfg.response_mode {
            ForceMode::GravityDown => {
                for p in &mut self.particles {
                    p.acc = p.acc.add(cfg.gravity);
                }
            }
            ForceMode::ZeroGravity => {}
            ForceMode::MutualGravity | ForceMode::MutualRepulsion => {
                let sign = if matches!(cfg.response_mode, ForceMode::MutualGravity) { 1.0 } else { -1.0 };
                let count = self.particles.len();
                for i in 0..count {
                    for j in (i + 1)..count {
                        let pi = &self.particles[i];
                        let pj = &self.particles[j];
                        let dx = pj.pos.x - pi.pos.x;
                        let dy = pj.pos.y - pi.pos.y;
                        let dist_sq = dx * dx + dy * dy;
                        let dist = dist_sq.sqrt().max(cfg.min_dist);
                        let force_mag = sign * cfg.mutual_strength * pi.mass * pj.mass / (dist_sq.max(100.0));
                        let fx = force_mag * dx / dist;
                        let fy = force_mag * dy / dist;
                        let mi = self.particles[i].mass;
                        let mj = self.particles[j].mass;
                        self.particles[i].acc.x += fx / mi;
                        self.particles[i].acc.y += fy / mi;
                        self.particles[j].acc.x -= fx / mj;
                        self.particles[j].acc.y -= fy / mj;
                    }
                }
            }
        }
    }

    fn update_positions(&mut self, dt: f64) {
        for p in &mut self.particles {
            let vel = p.pos.sub(p.old_pos).scale(cfg_friction(&self.config));
            let new_pos = p.pos.add(vel).add(p.acc.scale(dt * dt));
            p.old_pos = p.pos;
            p.pos = new_pos;
            p.acc = Vec2::zero();
        }
    }

    fn build_spatial_grid(&mut self) {
        self.spatial_grid.clear();
        for (i, p) in self.particles.iter().enumerate() {
            self.spatial_grid.insert(i, p.pos);
        }
    }

    fn solve_collisions(&mut self) {
        let pairs = self.spatial_grid.get_pairs(&self.particles);
        for (i, j) in pairs {
            let pi = &self.particles[i];
            let pj = &self.particles[j];
            let dx = pj.pos.x - pi.pos.x;
            let dy = pj.pos.y - pi.pos.y;
            let dist_sq = dx * dx + dy * dy;
            let min_dist = pi.radius + pj.radius;
            if dist_sq < min_dist * min_dist && dist_sq > 1e-12 {
                let dist = dist_sq.sqrt();
                let overlap = min_dist - dist;
                let nx = dx / dist;
                let ny = dy / dist;
                let total_mass = pi.mass + pj.mass;
                let mi_ratio = pj.mass / total_mass;
                let mj_ratio = pi.mass / total_mass;

                let pos_i = Vec2::new(
                    pi.pos.x - nx * overlap * mi_ratio,
                    pi.pos.y - ny * overlap * mi_ratio,
                );
                let pos_j = Vec2::new(
                    pj.pos.x + nx * overlap * mj_ratio,
                    pj.pos.y + ny * overlap * mj_ratio,
                );
                self.particles[i].pos = pos_i;
                self.particles[j].pos = pos_j;

                // Velocity response
                let vi = self.particles[i].velocity(1.0);
                let vj = self.particles[j].velocity(1.0);
                let rel_v = Vec2::new(vj.x - vi.x, vj.y - vi.y);
                let vel_along_normal = rel_v.x * nx + rel_v.y * ny;
                if vel_along_normal > 0.0 { continue; }
                let e = self.config.restitution;
                let impulse = -(1.0 + e) * vel_along_normal / (1.0 / pi.mass + 1.0 / pj.mass);
                let ix = impulse * nx;
                let iy = impulse * ny;
                let dt = 1.0;
                self.particles[i].old_pos.x -= ix / pi.mass * dt;
                self.particles[i].old_pos.y -= iy / pi.mass * dt;
                self.particles[j].old_pos.x += ix / pj.mass * dt;
                self.particles[j].old_pos.y += iy / pj.mass * dt;
            }
        }
    }

    fn solve_boundary_constraints(&mut self) {
        let b = self.config.bounds;
        let e = self.config.restitution;
        for p in &mut self.particles {
            if p.pos.x < b.min_x + p.radius {
                p.pos.x = b.min_x + p.radius;
                let vx = p.pos.x - p.old_pos.x;
                p.old_pos.x = p.pos.x + vx * e;
            }
            if p.pos.x > b.max_x - p.radius {
                p.pos.x = b.max_x - p.radius;
                let vx = p.pos.x - p.old_pos.x;
                p.old_pos.x = p.pos.x + vx * e;
            }
            if p.pos.y < b.min_y + p.radius {
                p.pos.y = b.min_y + p.radius;
                let vy = p.pos.y - p.old_pos.y;
                p.old_pos.y = p.pos.y + vy * e;
            }
            if p.pos.y > b.max_y - p.radius {
                p.pos.y = b.max_y - p.radius;
                let vy = p.pos.y - p.old_pos.y;
                p.old_pos.y = p.pos.y + vy * e;
            }
        }
    }

    fn apply_damping(&mut self) {
        let d = self.config.damping;
        for p in &mut self.particles {
            let vel = p.pos.sub(p.old_pos);
            if vel.length() > self.config.max_speed {
                let clamped = vel.normalize().scale(self.config.max_speed);
                p.old_pos = p.pos.sub(clamped);
            } else {
                p.old_pos = p.pos.sub(vel.scale(d));
            }
        }
    }
}

fn cfg_friction(cfg: &PhysicsConfig) -> f64 {
    cfg.friction
}

// ------------------------------------------------------------------
// Spatial Hash Grid for O(n) nearest-neighbor queries
// ------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct SpatialGrid {
    pub cell_size: f64,
    pub bounds: Bounds,
    pub cells: Vec<Vec<usize>>,
    pub grid_w: usize,
    pub grid_h: usize,
}

impl SpatialGrid {
    pub fn new(bounds: Bounds, cell_size: f64) -> Self {
        let gw = ((bounds.width() / cell_size).ceil() as usize).max(1);
        let gh = ((bounds.height() / cell_size).ceil() as usize).max(1);
        let cells = vec![Vec::new(); gw * gh];
        Self { cell_size, bounds, cells, grid_w: gw, grid_h: gh }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
    }

    fn pos_to_cell(&self, pos: Vec2) -> (usize, usize) {
        let cx = ((pos.x - self.bounds.min_x) / self.cell_size).floor() as usize;
        let cy = ((pos.y - self.bounds.min_y) / self.cell_size).floor() as usize;
        (cx.min(self.grid_w.saturating_sub(1)), cy.min(self.grid_h.saturating_sub(1)))
    }

    pub fn insert(&mut self, id: usize, pos: Vec2) {
        let (cx, cy) = self.pos_to_cell(pos);
        let idx = cy * self.grid_w + cx;
        if idx < self.cells.len() {
            self.cells[idx].push(id);
        }
    }

    pub fn get_pairs(&self, particles: &[Particle]) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        let mut checked = std::collections::HashSet::new();
        for cy in 0..self.grid_h {
            for cx in 0..self.grid_w {
                let idx = cy * self.grid_w + cx;
                let cell = &self.cells[idx];
                // Check within cell and neighboring cells
                for &i in cell {
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let ncx = cx as i32 + dx;
                            let ncy = cy as i32 + dy;
                            if ncx < 0 || ncy < 0 || ncx >= self.grid_w as i32 || ncy >= self.grid_h as i32 {
                                continue;
                            }
                            let nidx = ncy as usize * self.grid_w + ncx as usize;
                            for &j in &self.cells[nidx] {
                                if i >= j { continue; }
                                let key = (i, j);
                                if checked.insert(key) {
                                    let dist = particles[i].pos.dist(particles[j].pos);
                                    if dist < particles[i].radius + particles[j].radius + self.cell_size {
                                        pairs.push((i, j));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        pairs
    }
}
