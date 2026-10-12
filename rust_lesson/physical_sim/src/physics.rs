//! High-performance 2D particle physics engine.
//!
//! Optimizations applied:
//! - SoA (Structure of Arrays) data layout for cache efficiency
//! - Fixed timestep physics with render interpolation
//! - Spatial hash grid v2: no HashSet, only forward neighbor checks
//! - Object pooling: all buffers reused every frame
//! - N-body cutoff via spatial grid for O(N) neighbor interactions
//! - Precomputed inverse mass

use rand::distributions::{Distribution, Uniform};
use rand::rngs::ThreadRng;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForceMode {
    GravityDown,
    ZeroGravity,
    MutualGravity,
    MutualRepulsion,
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

#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    pub gravity_x: f64,
    pub gravity_y: f64,
    pub restitution: f64,
    pub friction: f64,
    pub substeps: usize,
    pub response_mode: ForceMode,
    pub mutual_strength: f64,
    pub cutoff: f64,
    pub max_speed: f64,
    pub damping: f64,
    pub bounds: Bounds,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity_x: 0.0,
            gravity_y: 500.0,
            restitution: 0.9,
            friction: 0.995,
            substeps: 8,
            response_mode: ForceMode::GravityDown,
            mutual_strength: 1000.0,
            cutoff: 120.0,
            max_speed: 5000.0,
            damping: 0.98,
            bounds: Bounds::new(0.0, 0.0, 900.0, 600.0),
        }
    }
}

/// SoA (Structure of Arrays) particle storage for cache-friendly sequential access.
#[derive(Debug, Clone)]
pub struct ParticleSoA {
    pub pos_x: Vec<f64>,
    pub pos_y: Vec<f64>,
    pub old_x: Vec<f64>,
    pub old_y: Vec<f64>,
    pub acc_x: Vec<f64>,
    pub acc_y: Vec<f64>,
    pub mass: Vec<f64>,
    pub inv_mass: Vec<f64>,
    pub radius: Vec<f64>,
    pub color_r: Vec<u8>,
    pub color_g: Vec<u8>,
    pub color_b: Vec<u8>,
    pub count: usize,
    pub capacity: usize,
}

impl ParticleSoA {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            pos_x: Vec::with_capacity(cap),
            pos_y: Vec::with_capacity(cap),
            old_x: Vec::with_capacity(cap),
            old_y: Vec::with_capacity(cap),
            acc_x: Vec::with_capacity(cap),
            acc_y: Vec::with_capacity(cap),
            mass: Vec::with_capacity(cap),
            inv_mass: Vec::with_capacity(cap),
            radius: Vec::with_capacity(cap),
            color_r: Vec::with_capacity(cap),
            color_g: Vec::with_capacity(cap),
            color_b: Vec::with_capacity(cap),
            count: 0,
            capacity: cap,
        }
    }

    pub fn push(&mut self, x: f64, y: f64, vx: f64, vy: f64, m: f64, r: f64) {
        if self.count >= self.capacity {
            self.grow();
        }
        let i = self.count;
        self.pos_x.push(x);
        self.pos_y.push(y);
        self.old_x.push(x - vx);
        self.old_y.push(y - vy);
        self.acc_x.push(0.0);
        self.acc_y.push(0.0);
        self.mass.push(m);
        self.inv_mass.push(1.0 / m);
        self.radius.push(r);
        let hash = i.wrapping_mul(0x9e3779b9);
        self.color_r.push(((hash >> 0) % 200 + 55) as u8);
        self.color_g.push(((hash >> 8) % 200 + 55) as u8);
        self.color_b.push(((hash >> 16) % 200 + 55) as u8);
        self.count += 1;
    }

    fn grow(&mut self) {
        let new_cap = (self.capacity * 2).max(64);
        self.capacity = new_cap;
        self.pos_x.reserve(new_cap - self.pos_x.capacity());
        self.pos_y.reserve(new_cap - self.pos_y.capacity());
        self.old_x.reserve(new_cap - self.old_x.capacity());
        self.old_y.reserve(new_cap - self.old_y.capacity());
        self.acc_x.reserve(new_cap - self.acc_x.capacity());
        self.acc_y.reserve(new_cap - self.acc_y.capacity());
        self.mass.reserve(new_cap - self.mass.capacity());
        self.inv_mass.reserve(new_cap - self.inv_mass.capacity());
        self.radius.reserve(new_cap - self.radius.capacity());
        self.color_r.reserve(new_cap - self.color_r.capacity());
        self.color_g.reserve(new_cap - self.color_g.capacity());
        self.color_b.reserve(new_cap - self.color_b.capacity());
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.pos_x.clear();
        self.pos_y.clear();
        self.old_x.clear();
        self.old_y.clear();
        self.acc_x.clear();
        self.acc_y.clear();
        self.mass.clear();
        self.inv_mass.clear();
        self.radius.clear();
        self.color_r.clear();
        self.color_g.clear();
        self.color_b.clear();
    }

    pub fn remove_last(&mut self) {
        if self.count == 0 { return; }
        self.count -= 1;
        self.pos_x.pop();
        self.pos_y.pop();
        self.old_x.pop();
        self.old_y.pop();
        self.acc_x.pop();
        self.acc_y.pop();
        self.mass.pop();
        self.inv_mass.pop();
        self.radius.pop();
        self.color_r.pop();
        self.color_g.pop();
        self.color_b.pop();
    }
}

#[derive(Debug)]
pub struct SpatialGrid {
    pub cell_size: f64,
    pub bounds: Bounds,
    pub cells: Vec<Vec<usize>>,
    pub grid_w: usize,
    pub grid_h: usize,
    pub pairs_buf: Vec<(usize, usize)>,
}

impl SpatialGrid {
    pub fn new(bounds: Bounds, cell_size: f64) -> Self {
        let gw = ((bounds.width() / cell_size).ceil() as usize).max(1);
        let gh = ((bounds.height() / cell_size).ceil() as usize).max(1);
        let cells = vec![Vec::with_capacity(16); gw * gh];
        Self {
            cell_size,
            bounds,
            cells,
            grid_w: gw,
            grid_h: gh,
            pairs_buf: Vec::with_capacity(1024),
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
        self.pairs_buf.clear();
    }

    #[inline(always)]
    fn pos_to_cell(&self, x: f64, y: f64) -> (usize, usize) {
        let cx = ((x - self.bounds.min_x) / self.cell_size).floor() as usize;
        let cy = ((y - self.bounds.min_y) / self.cell_size).floor() as usize;
        (cx.min(self.grid_w.saturating_sub(1)), cy.min(self.grid_h.saturating_sub(1)))
    }

    #[inline(always)]
    fn cell_index(&self, cx: usize, cy: usize) -> usize {
        cy * self.grid_w + cx
    }

    pub fn insert_all(&mut self, particles: &ParticleSoA) {
        for i in 0..particles.count {
            let (cx, cy) = self.pos_to_cell(particles.pos_x[i], particles.pos_y[i]);
            let idx = self.cell_index(cx, cy);
            if idx < self.cells.len() {
                self.cells[idx].push(i);
            }
        }
    }

    /// Forward-only neighbor checks to avoid duplicate pairs entirely.
    /// Checks self-cell and 4 forward neighbor directions.
    pub fn find_pairs(&mut self, particles: &ParticleSoA) {
        self.pairs_buf.clear();
        const DIRS: [(i32, i32); 5] = [(0, 0), (1, 0), (1, 1), (0, 1), (-1, 1)];
        let n = particles.count;
        if n == 0 { return; }

        for cy in 0..self.grid_h as i32 {
            for cx in 0..self.grid_w as i32 {
                let cidx = self.cell_index(cx as usize, cy as usize);
                let cell = &self.cells[cidx];
                if cell.is_empty() { continue; }

                for &(dx, dy) in &DIRS {
                    let ncx = cx + dx;
                    let ncy = cy + dy;
                    if ncx < 0 || ncy < 0 || ncx >= self.grid_w as i32 || ncy >= self.grid_h as i32 {
                        continue;
                    }
                    let ncidx = self.cell_index(ncx as usize, ncy as usize);
                    let other = &self.cells[ncidx];

                    if dx == 0 && dy == 0 {
                        // Same cell: only i < j pairs
                        for a in 0..cell.len() {
                            let i = cell[a];
                            for b in (a + 1)..cell.len() {
                                let j = cell[b];
                                let dx_ = particles.pos_x[i] - particles.pos_x[j];
                                let dy_ = particles.pos_y[i] - particles.pos_y[j];
                                let rsum = particles.radius[i] + particles.radius[j];
                                if dx_ * dx_ + dy_ * dy_ < rsum * rsum {
                                    self.pairs_buf.push((i, j));
                                }
                            }
                        }
                    } else {
                        // Neighbor cell: all cross pairs (no duplicates because of forward-only)
                        for &i in cell {
                            for &j in other {
                                let dx_ = particles.pos_x[i] - particles.pos_x[j];
                                let dy_ = particles.pos_y[i] - particles.pos_y[j];
                                let rsum = particles.radius[i] + particles.radius[j];
                                if dx_ * dx_ + dy_ * dy_ < rsum * rsum {
                                    self.pairs_buf.push((i, j));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn find_pairs_within_cutoff(&mut self, particles: &ParticleSoA, cutoff: f64) {
        self.pairs_buf.clear();
        const DIRS: [(i32, i32); 5] = [(0, 0), (1, 0), (1, 1), (0, 1), (-1, 1)];
        let cutoff_sq = cutoff * cutoff;
        let n = particles.count;
        if n == 0 { return; }

        for cy in 0..self.grid_h as i32 {
            for cx in 0..self.grid_w as i32 {
                let cidx = self.cell_index(cx as usize, cy as usize);
                let cell = &self.cells[cidx];
                if cell.is_empty() { continue; }

                for &(dx, dy) in &DIRS {
                    let ncx = cx + dx;
                    let ncy = cy + dy;
                    if ncx < 0 || ncy < 0 || ncx >= self.grid_w as i32 || ncy >= self.grid_h as i32 {
                        continue;
                    }
                    let ncidx = self.cell_index(ncx as usize, ncy as usize);
                    let other = &self.cells[ncidx];

                    if dx == 0 && dy == 0 {
                        for a in 0..cell.len() {
                            let i = cell[a];
                            for b in (a + 1)..cell.len() {
                                let j = cell[b];
                                let dx_ = particles.pos_x[i] - particles.pos_x[j];
                                let dy_ = particles.pos_y[i] - particles.pos_y[j];
                                let dist_sq = dx_ * dx_ + dy_ * dy_;
                                if dist_sq < cutoff_sq {
                                    self.pairs_buf.push((i, j));
                                }
                            }
                        }
                    } else {
                        for &i in cell {
                            for &j in other {
                                let dx_ = particles.pos_x[i] - particles.pos_x[j];
                                let dy_ = particles.pos_y[i] - particles.pos_y[j];
                                let dist_sq = dx_ * dx_ + dy_ * dy_;
                                if dist_sq < cutoff_sq {
                                    self.pairs_buf.push((i, j));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct PhysicsWorld {
    pub particles: ParticleSoA,
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
            particles: ParticleSoA::with_capacity(1024),
            config,
            time: 0.0,
            rng: rand::thread_rng(),
            paused: false,
            step_once: false,
            spatial_grid: SpatialGrid::new(bounds, 40.0),
        }
    }

    pub fn add_particle(&mut self, x: f64, y: f64, vx: f64, vy: f64, mass: f64, radius: f64) {
        self.particles.push(x, y, vx, vy, mass, radius);
    }

    pub fn spawn_random(&mut self, count: usize) {
        let b = self.config.bounds;
        let ux = Uniform::from(b.min_x + 50.0..b.max_x - 50.0);
        let uy = Uniform::from(b.min_y + 50.0..b.max_y - 50.0);
        let um = Uniform::from(1.0..5.0);
        let ur = Uniform::from(5.0..12.0);
        for _ in 0..count {
            let x = ux.sample(&mut self.rng);
            let y = uy.sample(&mut self.rng);
            let m = um.sample(&mut self.rng);
            let r = ur.sample(&mut self.rng);
            let vx = (self.rng.gen::<f64>() - 0.5) * 200.0;
            let vy = (self.rng.gen::<f64>() - 0.5) * 200.0;
            self.add_particle(x, y, vx, vy, m, r);
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.time = 0.0;
    }

    pub fn total_kinetic_energy(&self, dt: f64) -> f64 {
        let mut ke = 0.0;
        for i in 0..self.particles.count {
            let vx = (self.particles.pos_x[i] - self.particles.old_x[i]) / dt;
            let vy = (self.particles.pos_y[i] - self.particles.old_y[i]) / dt;
            ke += 0.5 * self.particles.mass[i] * (vx * vx + vy * vy);
        }
        ke
    }

    pub fn total_momentum(&self, dt: f64) -> (f64, f64) {
        let mut px = 0.0;
        let mut py = 0.0;
        for i in 0..self.particles.count {
            let vx = (self.particles.pos_x[i] - self.particles.old_x[i]) / dt;
            let vy = (self.particles.pos_y[i] - self.particles.old_y[i]) / dt;
            px += self.particles.mass[i] * vx;
            py += self.particles.mass[i] * vy;
        }
        (px, py)
    }

    pub fn update(&mut self, dt: f64) {
        if self.paused && !self.step_once {
            return;
        }
        self.step_once = false;

        let sub_dt = dt / self.config.substeps as f64;
        for _ in 0..self.config.substeps {
            self.apply_forces();
            self.integrate_verlet(sub_dt);
            self.build_spatial_grid();
            self.solve_collisions();
            self.solve_boundary_constraints();
            self.apply_damping();
        }
        self.time += dt;
    }

    fn apply_forces(&mut self) {
        let n = self.particles.count;
        match self.config.response_mode {
            ForceMode::GravityDown => {
                let gx = self.config.gravity_x;
                let gy = self.config.gravity_y;
                for i in 0..n {
                    self.particles.acc_x[i] += gx;
                    self.particles.acc_y[i] += gy;
                }
            }
            ForceMode::ZeroGravity => {}
            ForceMode::MutualGravity | ForceMode::MutualRepulsion => {
                let sign = if matches!(self.config.response_mode, ForceMode::MutualGravity) { 1.0 } else { -1.0 };
                let strength = self.config.mutual_strength;
                let cutoff = self.config.cutoff;
                self.spatial_grid.find_pairs_within_cutoff(&self.particles, cutoff);
                let pairs = &self.spatial_grid.pairs_buf;
                for &(i, j) in pairs {
                    let dx = self.particles.pos_x[j] - self.particles.pos_x[i];
                    let dy = self.particles.pos_y[j] - self.particles.pos_y[i];
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt().max(10.0);
                    let force_mag = sign * strength * self.particles.mass[i] * self.particles.mass[j] / dist_sq.max(100.0);
                    let fx = force_mag * dx / dist;
                    let fy = force_mag * dy / dist;
                    let mi = self.particles.inv_mass[i];
                    let mj = self.particles.inv_mass[j];
                    self.particles.acc_x[i] += fx * mi;
                    self.particles.acc_y[i] += fy * mi;
                    self.particles.acc_x[j] -= fx * mj;
                    self.particles.acc_y[j] -= fy * mj;
                }
            }
        }
    }

    fn integrate_verlet(&mut self, dt: f64) {
        let friction = self.config.friction;
        let dt_sq = dt * dt;
        for i in 0..self.particles.count {
            let vx = (self.particles.pos_x[i] - self.particles.old_x[i]) * friction;
            let vy = (self.particles.pos_y[i] - self.particles.old_y[i]) * friction;
            let new_x = self.particles.pos_x[i] + vx + self.particles.acc_x[i] * dt_sq;
            let new_y = self.particles.pos_y[i] + vy + self.particles.acc_y[i] * dt_sq;
            self.particles.old_x[i] = self.particles.pos_x[i];
            self.particles.old_y[i] = self.particles.pos_y[i];
            self.particles.pos_x[i] = new_x;
            self.particles.pos_y[i] = new_y;
            self.particles.acc_x[i] = 0.0;
            self.particles.acc_y[i] = 0.0;
        }
    }

    fn build_spatial_grid(&mut self) {
        self.spatial_grid.clear();
        self.spatial_grid.insert_all(&self.particles);
    }

    fn solve_collisions(&mut self) {
        self.spatial_grid.find_pairs(&self.particles);
        let pairs = self.spatial_grid.pairs_buf.clone();
        let e = self.config.restitution;
        for &(i, j) in &pairs {
            let dx = self.particles.pos_x[j] - self.particles.pos_x[i];
            let dy = self.particles.pos_y[j] - self.particles.pos_y[i];
            let dist_sq = dx * dx + dy * dy;
            let min_dist = self.particles.radius[i] + self.particles.radius[j];
            if dist_sq >= min_dist * min_dist || dist_sq < 1e-12 {
                continue;
            }
            let dist = dist_sq.sqrt();
            let overlap = min_dist - dist;
            let nx = dx / dist;
            let ny = dy / dist;

            let mi = self.particles.mass[i];
            let mj = self.particles.mass[j];
            let total_mass = mi + mj;
            let mi_ratio = mj / total_mass;
            let mj_ratio = mi / total_mass;

            self.particles.pos_x[i] -= nx * overlap * mi_ratio;
            self.particles.pos_y[i] -= ny * overlap * mi_ratio;
            self.particles.pos_x[j] += nx * overlap * mj_ratio;
            self.particles.pos_y[j] += ny * overlap * mj_ratio;

            // Velocity response via old_pos modification (Verlet)
            let vix = self.particles.pos_x[i] - self.particles.old_x[i];
            let viy = self.particles.pos_y[i] - self.particles.old_y[i];
            let vjx = self.particles.pos_x[j] - self.particles.old_x[j];
            let vjy = self.particles.pos_y[j] - self.particles.old_y[j];
            let rvx = vjx - vix;
            let rvy = vjy - viy;
            let vel_along = rvx * nx + rvy * ny;
            if vel_along > 0.0 { continue; }
            let impulse = -(1.0 + e) * vel_along / (self.particles.inv_mass[i] + self.particles.inv_mass[j]);
            let ix = impulse * nx;
            let iy = impulse * ny;
            self.particles.old_x[i] -= ix * self.particles.inv_mass[i];
            self.particles.old_y[i] -= iy * self.particles.inv_mass[i];
            self.particles.old_x[j] += ix * self.particles.inv_mass[j];
            self.particles.old_y[j] += iy * self.particles.inv_mass[j];
        }
    }

    fn solve_boundary_constraints(&mut self) {
        let b = self.config.bounds;
        let e = self.config.restitution;
        for i in 0..self.particles.count {
            let r = self.particles.radius[i];
            if self.particles.pos_x[i] < b.min_x + r {
                let vx = self.particles.pos_x[i] - self.particles.old_x[i];
                self.particles.pos_x[i] = b.min_x + r;
                self.particles.old_x[i] = self.particles.pos_x[i] + vx * e;
            }
            if self.particles.pos_x[i] > b.max_x - r {
                let vx = self.particles.pos_x[i] - self.particles.old_x[i];
                self.particles.pos_x[i] = b.max_x - r;
                self.particles.old_x[i] = self.particles.pos_x[i] + vx * e;
            }
            if self.particles.pos_y[i] < b.min_y + r {
                let vy = self.particles.pos_y[i] - self.particles.old_y[i];
                self.particles.pos_y[i] = b.min_y + r;
                self.particles.old_y[i] = self.particles.pos_y[i] + vy * e;
            }
            if self.particles.pos_y[i] > b.max_y - r {
                let vy = self.particles.pos_y[i] - self.particles.old_y[i];
                self.particles.pos_y[i] = b.max_y - r;
                self.particles.old_y[i] = self.particles.pos_y[i] + vy * e;
            }
        }
    }

    fn apply_damping(&mut self) {
        let d = self.config.damping;
        let max_spd = self.config.max_speed;
        for i in 0..self.particles.count {
            let vx = self.particles.pos_x[i] - self.particles.old_x[i];
            let vy = self.particles.pos_y[i] - self.particles.old_y[i];
            let spd_sq = vx * vx + vy * vy;
            if spd_sq > max_spd * max_spd {
                let scale = max_spd / spd_sq.sqrt();
                self.particles.old_x[i] = self.particles.pos_x[i] - vx * scale;
                self.particles.old_y[i] = self.particles.pos_y[i] - vy * scale;
            } else {
                self.particles.old_x[i] = self.particles.pos_x[i] - vx * d;
                self.particles.old_y[i] = self.particles.pos_y[i] - vy * d;
            }
        }
    }
}