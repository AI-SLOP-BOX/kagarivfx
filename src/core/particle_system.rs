/// Basic particle emitter system for After Effects-style particle animations.
///
/// Supports:
/// - Emitter with configurable rate, lifetime, speed, spread
/// - Gravity (with lifetime curve), wind gusts, turbulence and air drag
/// - Boundary collisions with restitution & friction
/// - Per-particle size, color, and opacity over lifetime
/// - Point and box emitter shapes
use serde::{Deserialize, Serialize};

use crate::core::particle_forces::{
    apply_drag, resolve_bounds_collision, resolve_pairwise_collisions, wind_with_gust, LifeCurve,
};

/// Emitter shape type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EmitterShape {
    #[default]
    Point = 0,
    Box = 1,
    Circle = 2,
    Line = 3,
    /// Emission from a thin circle at `emitter_size[0]` diameter.
    Ring = 4,
}

/// Opacity fade curve over particle lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FadeCurve {
    #[default]
    Linear = 0,
    EaseIn = 1,
    EaseOut = 2,
}

impl FadeCurve {
    /// Map normalized lifetime progress t (0..1) to fade factor (0..1).
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            FadeCurve::Linear => 1.0 - t,
            FadeCurve::EaseIn => 1.0 - t * t,
            FadeCurve::EaseOut => (1.0 - t) * (1.0 - t),
        }
    }
}

/// Particle emitter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleEmitter {
    /// Particles per second
    pub rate: f32,
    /// Max particles alive at once
    pub max_particles: u32,
    /// Particle lifetime in seconds
    pub lifetime: f32,
    /// Random lifetime variation (0..1)
    pub lifetime_variance: f32,
    /// Initial speed (pixels/sec)
    pub speed: f32,
    /// Random speed variation (0..1)
    pub speed_variance: f32,
    /// Spread angle in degrees (0 = one direction, 360 = all directions)
    pub spread_degrees: f32,
    /// Emitter shape
    pub shape: EmitterShape,
    /// Box emitter size [width, height]
    pub emitter_size: [f32; 2],
    /// Gravity [x, y] (pixels/sec^2)
    pub gravity: [f32; 2],
    /// Wind [x, y] (pixels/sec)
    pub wind: [f32; 2],
    /// Turbulence strength
    pub turbulence: f32,
    /// Start color [r, g, b, a]
    pub color_start: [f32; 4],
    /// End color [r, g, b, a]
    pub color_end: [f32; 4],
    /// Start size (pixels)
    pub size_start: f32,
    /// End size (pixels)
    pub size_end: f32,
    /// Start opacity (0..1)
    pub opacity_start: f32,
    /// End opacity (0..1)
    pub opacity_end: f32,
    /// Rotation speed (degrees/sec)
    #[serde(default)]
    pub rotation_speed: f32,
    /// Initial rotation (degrees)
    #[serde(default)]
    pub rotation_start: f32,
    /// Random rotation speed variation (0..1)
    #[serde(default)]
    pub rotation_speed_variance: f32,
    /// Opacity fade curve over lifetime
    #[serde(default)]
    pub fade_curve: FadeCurve,
    /// Blend mode: 0=Normal, 1=Add, 2=Screen
    pub blend_mode: u32,
    /// Gravity multiplier sampled over normalized particle lifetime.
    #[serde(default)]
    pub gravity_curve: LifeCurve,
    /// Peak sinusoidal wind gust strength (px/s^2), 0 = steady wind only.
    #[serde(default)]
    pub wind_gust_strength: f32,
    /// Wind gust frequency in Hz.
    #[serde(default)]
    pub wind_gust_frequency: f32,
    /// Exponential air drag coefficient (1/s).
    #[serde(default)]
    pub drag: f32,
    /// Enable axis-aligned boundary collisions.
    #[serde(default)]
    pub collision_enabled: bool,
    /// Collision bounds [min_x, min_y, max_x, max_y].
    #[serde(default)]
    pub collision_bounds: [f32; 4],
    /// Bounce energy retention on collision (0..1).
    #[serde(default)]
    pub restitution: f32,
    /// Tangential velocity retention on collision (0..1).
    #[serde(default)]
    pub surface_friction: f32,
    /// Particle-vs-particle soft-sphere collisions (O(n²) per update step).
    #[serde(default)]
    pub particle_collisions: bool,
    /// Contact diameter for particle-vs-particle collisions (px).
    #[serde(default = "default_particle_diameter")]
    pub particle_diameter: f32,
    /// Trail length: number of previous positions to store (0 = no trail)
    #[serde(default)]
    pub trail_length: u8,
    /// Trail taper: alpha multiplier per trail step (0..1, 1 = no fade)
    #[serde(default = "default_trail_taper")]
    pub trail_taper: f32,
    /// Vortex tangential acceleration around `vortex_center` (px/s^2, signed:
    /// positive spins clockwise in screen space, 0 = off)
    #[serde(default)]
    pub vortex_strength: f32,
    /// Vortex center [x, y]
    #[serde(default)]
    pub vortex_center: [f32; 2],
    /// Attraction (+) / repulsion (-) toward `attract_center` (px/s^2)
    #[serde(default)]
    pub attract_strength: f32,
    /// Attractor point [x, y]
    #[serde(default)]
    pub attract_center: [f32; 2],
    /// Enable per-particle Z depth for camera-projected rendering
    #[serde(default)]
    pub depth_enabled: bool,
    /// Spawn Z range [min, max] offsets from the layer plane (world units)
    #[serde(default)]
    pub depth_range: [f32; 2],
    /// Child particles emitted radially when a particle dies (0 = off)
    #[serde(default)]
    pub death_spawn_count: u32,
    /// Child initial speed = parent emitter speed × this (0..1 typical)
    #[serde(default = "default_death_speed_scale")]
    pub death_spawn_speed_scale: f32,
    /// Child lifetime = emitter lifetime × this
    #[serde(default = "default_death_life_scale")]
    pub death_spawn_life_scale: f32,
}

fn default_death_speed_scale() -> f32 {
    0.5
}
fn default_death_life_scale() -> f32 {
    0.5
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            rate: 50.0,
            max_particles: 1000,
            lifetime: 2.0,
            lifetime_variance: 0.2,
            speed: 200.0,
            speed_variance: 0.3,
            spread_degrees: 360.0,
            shape: EmitterShape::Point,
            emitter_size: [100.0, 100.0],
            gravity: [0.0, 300.0],
            wind: [0.0, 0.0],
            turbulence: 0.0,
            color_start: [1.0, 0.8, 0.2, 1.0],
            color_end: [1.0, 0.1, 0.0, 0.0],
            size_start: 8.0,
            size_end: 2.0,
            opacity_start: 1.0,
            opacity_end: 0.0,
            rotation_speed: 0.0,
            rotation_start: 0.0,
            rotation_speed_variance: 0.0,
            fade_curve: FadeCurve::Linear,
            blend_mode: 1, // Add
            gravity_curve: LifeCurve::default(),
            wind_gust_strength: 0.0,
            wind_gust_frequency: 0.5,
            drag: 0.0,
            collision_enabled: false,
            collision_bounds: [0.0, 0.0, 1920.0, 1080.0],
            restitution: 0.5,
            surface_friction: 0.9,
            particle_collisions: false,
            particle_diameter: 8.0,
            trail_length: 0,
            trail_taper: 0.7,
            vortex_strength: 0.0,
            vortex_center: [0.0, 0.0],
            attract_strength: 0.0,
            attract_center: [0.0, 0.0],
            death_spawn_count: 0,
            death_spawn_speed_scale: 0.5,
            death_spawn_life_scale: 0.5,
            depth_enabled: false,
            depth_range: [0.0, 0.0],
        }
    }
}

fn default_particle_diameter() -> f32 {
    8.0
}

fn default_trail_taper() -> f32 {
    0.7
}

/// State of a single alive particle.
#[derive(Debug, Clone)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub rotation: f32,
    /// Per-particle angular velocity (degrees/sec), set at emission
    pub angular_velocity: f32,
    /// Trail history: last N positions (newest first), ring buffer
    pub trail: [(f32, f32); 8],
    pub trail_len: u8,
    /// Depth offset (world Z) for camera-projected rendering
    pub z: f32,
}

/// Particle system simulation.
#[derive(Clone)]
pub struct ParticleSystem {
    pub emitter: ParticleEmitter,
    pub particles: Vec<Particle>,
    /// Accumulated time for emission scheduling
    pub emit_accumulator: f32,
    /// Total simulated time (drives wind gusts)
    pub elapsed_time: f32,
    /// PRNG state for deterministic randomness
    rng_state: u64,
}

impl ParticleSystem {
    pub fn new(emitter: ParticleEmitter) -> Self {
        Self {
            emitter,
            particles: Vec::new(),
            emit_accumulator: 0.0,
            elapsed_time: 0.0,
            rng_state: 0xDEAD_BEEF_CAFE_BABE,
        }
    }

    /// Simple xorshift64 PRNG.
    fn next_random(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state & 0xFFFF) as f32 / 65535.0
    }

    /// Emit a single particle at the emitter position.
    fn emit_particle(&mut self, emitter_x: f32, emitter_y: f32) {
        if self.particles.len() >= self.emitter.max_particles as usize {
            return;
        }

        let lifetime = self.emitter.lifetime
            * (1.0 - self.emitter.lifetime_variance * (self.next_random() * 2.0 - 1.0));
        let speed = self.emitter.speed
            * (1.0 + self.emitter.speed_variance * (self.next_random() * 2.0 - 1.0));
        let rotation_speed = if self.emitter.rotation_speed_variance > 0.0 {
            self.emitter.rotation_speed
                * (1.0 + self.emitter.rotation_speed_variance * (self.next_random() * 2.0 - 1.0))
        } else {
            self.emitter.rotation_speed
        };

        let spread_rad = self.emitter.spread_degrees.to_radians();
        let angle = (self.next_random() - 0.5) * spread_rad;

        let (px, py) = match self.emitter.shape {
            EmitterShape::Point => (emitter_x, emitter_y),
            EmitterShape::Box => (
                emitter_x + (self.next_random() - 0.5) * self.emitter.emitter_size[0],
                emitter_y + (self.next_random() - 0.5) * self.emitter.emitter_size[1],
            ),
            EmitterShape::Circle => {
                let r = self.next_random().sqrt() * self.emitter.emitter_size[0] * 0.5;
                let a = self.next_random() * std::f32::consts::TAU;
                (emitter_x + r * a.cos(), emitter_y + r * a.sin())
            }
            EmitterShape::Line => (
                emitter_x + (self.next_random() - 0.5) * self.emitter.emitter_size[0],
                emitter_y,
            ),
            EmitterShape::Ring => {
                // Points sit ON the circle of the given diameter.
                let r = self.emitter.emitter_size[0] * 0.5;
                let a = self.next_random() * std::f32::consts::TAU;
                (emitter_x + r * a.cos(), emitter_y + r * a.sin())
            }
        };

        let z = if self.emitter.depth_enabled {
            let [z0, z1] = self.emitter.depth_range;
            z0 + (z1 - z0) * self.next_random()
        } else {
            0.0
        };

        self.particles.push(Particle {
            x: px,
            y: py,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            life: lifetime,
            max_life: lifetime,
            size: self.emitter.size_start,
            rotation: self.emitter.rotation_start,
            angular_velocity: rotation_speed,
            trail: [(px, py); 8],
            trail_len: 0,
            z,
        });
    }

    /// Step the particle simulation by dt seconds.
    pub fn update(&mut self, dt: f32, emitter_x: f32, emitter_y: f32) {
        // Emit new particles
        self.emit_accumulator += dt * self.emitter.rate;
        while self.emit_accumulator >= 1.0 {
            self.emit_particle(emitter_x, emitter_y);
            self.emit_accumulator -= 1.0;
        }
        self.elapsed_time += dt;

        let gravity = self.emitter.gravity;
        let gcurve = self.emitter.gravity_curve.clone();
        let gust_strength = self.emitter.wind_gust_strength;
        let gust_freq = self.emitter.wind_gust_frequency;
        let base_wind = self.emitter.wind;
        let turbulence = self.emitter.turbulence;
        let size_start = self.emitter.size_start;
        let size_end = self.emitter.size_end;
        let drag = self.emitter.drag;
        let collide = self.emitter.collision_enabled;
        let bounds = self.emitter.collision_bounds;
        let restitution = self.emitter.restitution.clamp(0.0, 1.0);
        let friction = self.emitter.surface_friction.clamp(0.0, 1.0);
        let vortex_strength = self.emitter.vortex_strength;
        let vortex_center = self.emitter.vortex_center;
        let attract_strength = self.emitter.attract_strength;
        let attract_center = self.emitter.attract_center;

        // Update existing particles
        for p in &mut self.particles {
            p.life -= dt;
            if p.life <= 0.0 {
                continue;
            }

            // Forces — gravity scaled by its lifetime curve, wind with gusts.
            let age_t = 1.0 - (p.life / p.max_life).max(0.0);
            let gmul = gcurve.eval(age_t);
            let wind = wind_with_gust(base_wind, gust_strength, gust_freq, self.elapsed_time);
            p.vx += (gravity[0] * gmul + wind[0]) * dt;
            p.vy += (gravity[1] * gmul + wind[1]) * dt;

            // Air drag
            apply_drag(&mut p.vx, &mut p.vy, drag, dt);

            // Vortex: tangential acceleration around center (falls off with distance)
            if vortex_strength != 0.0 {
                let rx = p.x - vortex_center[0];
                let ry = p.y - vortex_center[1];
                let dist = (rx * rx + ry * ry).sqrt().max(8.0);
                // Tangent perpendicular to radius; +strength = clockwise on screen
                p.vx += (-ry / dist) * vortex_strength * dt;
                p.vy += (rx / dist) * vortex_strength * dt;
            }

            // Attraction / repulsion toward attractor point
            if attract_strength != 0.0 {
                let ax = attract_center[0] - p.x;
                let ay = attract_center[1] - p.y;
                let dist = (ax * ax + ay * ay).sqrt().max(4.0);
                p.vx += (ax / dist) * attract_strength * dt;
                p.vy += (ay / dist) * attract_strength * dt;
            }

            // Turbulence
            if turbulence > 0.0 {
                let t = p.life * 10.0;
                p.vx += (t.sin() * 0.5 + (t * 2.7).sin() * 0.3) * turbulence * dt;
                p.vy += (t.cos() * 0.5 + (t * 3.1).cos() * 0.3) * turbulence * dt;
            }

            // Integrate position
            p.x += p.vx * dt;
            p.y += p.vy * dt;

            // Update trail: shift positions and add new one
            let max_trail = self.emitter.trail_length.min(8);
            if max_trail > 0 {
                // Shift existing trail positions
                let mut i = max_trail as usize;
                while i > 0 {
                    p.trail[i] = p.trail[i - 1];
                    i -= 1;
                }
                p.trail[0] = (p.x, p.y);
                if p.trail_len < max_trail {
                    p.trail_len += 1;
                }
            }

            // Boundary collisions
            if collide {
                let mut pos = [p.x, p.y];
                let mut vel = [p.vx, p.vy];
                resolve_bounds_collision(&mut pos, &mut vel, bounds, restitution, friction);
                p.x = pos[0];
                p.y = pos[1];
                p.vx = vel[0];
                p.vy = vel[1];
            }

            // Rotation
            p.rotation += p.angular_velocity * dt;

            // Interpolate size and opacity over lifetime
            let t = 1.0 - (p.life / p.max_life);
            p.size = size_start + (size_end - size_start) * t;
        }

        // Particle-vs-particle collisions (uniform contact diameter).
        if self.emitter.particle_collisions && self.particles.len() > 1 {
            let mut pos: Vec<[f32; 2]> = self.particles.iter().map(|p| [p.x, p.y]).collect();
            let mut vel: Vec<[f32; 2]> = self.particles.iter().map(|p| [p.vx, p.vy]).collect();
            resolve_pairwise_collisions(
                &mut pos,
                &mut vel,
                self.emitter.particle_diameter.max(0.1),
                restitution,
            );
            for (p, (np, nv)) in self.particles.iter_mut().zip(pos.into_iter().zip(vel)) {
                p.x = np[0];
                p.y = np[1];
                p.vx = nv[0];
                p.vy = nv[1];
            }
        }

        // Death-spawn: emit child particles where parents just died
        if self.emitter.death_spawn_count > 0 {
            let count = self.emitter.death_spawn_count;
            let speed_scale = self.emitter.death_spawn_speed_scale;
            let life_scale = self.emitter.death_spawn_life_scale;
            let spread = self.emitter.spread_degrees.to_radians();
            let base_speed = self.emitter.speed * speed_scale;
            let dead: Vec<(f32, f32, f32)> = self
                .particles
                .iter()
                .filter(|p| p.life <= 0.0)
                .map(|p| (p.x, p.y, p.z))
                .collect();
            for (dx, dy, parent_z) in dead {
                for _ in 0..count {
                    if self.particles.len() >= self.emitter.max_particles as usize {
                        break;
                    }
                    let angle = self.next_random() * std::f32::consts::TAU;
                    let _ = spread; // children burst radially in all directions
                    let speed = base_speed * (0.5 + self.next_random() * 0.5);
                    let lifetime = (self.emitter.lifetime * life_scale).max(0.05);
                    self.particles.push(Particle {
                        x: dx,
                        y: dy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed,
                        life: lifetime,
                        max_life: lifetime,
                        size: self.emitter.size_start * 0.5,
                        rotation: 0.0,
                        angular_velocity: 0.0,
                        trail: [(dx, dy); 8],
                        trail_len: 0,
                        z: parent_z,
                    });
                }
            }
        }

        // Remove dead particles
        self.particles.retain(|p| p.life > 0.0);
    }

    /// Render all particles into an RGBA pixel buffer (flat, no camera).
    pub fn render(&self, buffer: &mut [u8], buf_width: u32, buf_height: u32, _time: f32) {
        self.render_projected(buffer, buf_width, buf_height, _time, None);
    }

    /// Render with optional camera projection: particle Z depth drives screen
    /// position and size scaling (translate + Z-rotate pinhole matching the
    /// renderer's 3D layer convention).
    pub fn render_projected(
        &self,
        buffer: &mut [u8],
        buf_width: u32,
        buf_height: u32,
        _time: f32,
        proj: Option<&CameraProjection>,
    ) {
        let e = &self.emitter;
        for p in &self.particles {
            if p.life <= 0.0 {
                continue;
            }

            let t = 1.0 - (p.life / p.max_life);
            let r = e.color_start[0] + (e.color_end[0] - e.color_start[0]) * t;
            let g = e.color_start[1] + (e.color_end[1] - e.color_start[1]) * t;
            let b = e.color_start[2] + (e.color_end[2] - e.color_start[2]) * t;
            let a = (e.color_start[3] + (e.color_end[3] - e.color_start[3]) * t)
                * e.fade_curve.apply(t);

            // Project main position (flat fallback keeps legacy behavior)
            let Some((sx, sy, sscale)) =
                proj.map_or(Some((p.x, p.y, 1.0)), |cp| cp.project(p.x, p.y, p.z))
            else {
                continue;
            };

            // Trail dots: projected per-point so streaks follow depth too
            if p.trail_len > 0 && e.trail_taper > 0.01 {
                let max_trail = p.trail_len as usize;
                let trail_size = p.size * 0.4;
                for i in 0..max_trail {
                    let (tx, ty) = p.trail[i];
                    let fade = e.trail_taper.powi(i as i32 + 1);
                    let ta = a * fade;
                    if ta < 0.01 {
                        continue;
                    }
                    let Some((txs, tys, ts_scale)) =
                        proj.map_or(Some((tx, ty, 1.0)), |cp| cp.project(tx, ty, p.z))
                    else {
                        continue;
                    };
                    let half_t =
                        trail_size * ts_scale * 0.5 * (1.0 - (i as f32 / max_trail as f32) * 0.5);
                    draw_dot(
                        buffer,
                        (buf_width, buf_height),
                        (txs, tys),
                        half_t,
                        [r, g, b],
                        ta,
                        e.blend_mode,
                    );
                }
            }

            let half = p.size * sscale * 0.5;
            draw_dot(
                buffer,
                (buf_width, buf_height),
                (sx, sy),
                half,
                [r, g, b],
                a,
                e.blend_mode,
            );
        }
    }
}

/// Camera model for projected particle rendering.
#[derive(Debug, Clone, Copy)]
pub struct CameraProjection {
    pub cam_x: f32,
    pub cam_y: f32,
    pub cam_z: f32,
    /// Focal length in pixels derived from vertical FOV and output height
    pub focal: f32,
    pub cos_rz: f32,
    pub sin_rz: f32,
}

impl CameraProjection {
    /// World (x,y,z) -> screen (sx, sy, size-scale). None when behind camera.
    pub fn project(&self, x: f32, y: f32, z: f32) -> Option<(f32, f32, f32)> {
        let dz = z - self.cam_z;
        if dz <= 0.1 {
            return None;
        }
        let s = self.focal / dz;
        let rx = x - self.cam_x;
        let ry = y - self.cam_y;
        let sx = self.cam_x + (rx * self.cos_rz - ry * self.sin_rz) * s;
        let sy = self.cam_y - (rx * self.sin_rz + ry * self.cos_rz) * s;
        // Size scales relative to the z=0 plane; guard degenerate on-plane cams
        let ref_dz = (-self.cam_z).abs().max(100.0);
        Some((sx, sy, (ref_dz / dz).max(0.01)))
    }
}

/// Rasterize one soft-circle dot with the emitter's blend mode.
fn draw_dot(
    buffer: &mut [u8],
    dims: (u32, u32),
    center: (f32, f32),
    half: f32,
    color: [f32; 3],
    a: f32,
    blend_mode: u32,
) {
    let [r, g, b] = color;
    let buf_width = dims.0;
    let buf_height = dims.1;
    let cx = center.0;
    let cy = center.1;
    if buf_width == 0
        || buf_height == 0
        || half < 0.35
        || !half.is_finite()
        || !cx.is_finite()
        || !cy.is_finite()
        || !a.is_finite()
        || !color.iter().all(|value| value.is_finite())
        || a <= 0.001
    {
        return;
    }
    let x0 = ((cx - half).max(0.0)) as u32;
    let y0 = ((cy - half).max(0.0)) as u32;
    let x1 = ((cx + half).min(buf_width as f32 - 1.0)) as u32;
    let y1 = ((cy + half).min(buf_height as f32 - 1.0)) as u32;

    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let dist = ((dx * dx + dy * dy).sqrt() / half).min(1.0);
            let falloff = (1.0 - dist * dist).max(0.0);
            let pixel_a = a * falloff;
            if pixel_a <= 0.001 {
                continue;
            }

            let idx = ((py * buf_width + px) * 4) as usize;
            if idx + 3 >= buffer.len() {
                continue;
            }

            let src_r = r * pixel_a;
            let src_g = g * pixel_a;
            let src_b = b * pixel_a;

            match blend_mode {
                1 => {
                    let dst_a = buffer[idx + 3] as f32 / 255.0;
                    let dr = buffer[idx] as f32 / 255.0 + src_r;
                    let dg = buffer[idx + 1] as f32 / 255.0 + src_g;
                    let db = buffer[idx + 2] as f32 / 255.0 + src_b;
                    let da = (dst_a + pixel_a).min(1.0);
                    buffer[idx] = (dr.min(1.0) * 255.0) as u8;
                    buffer[idx + 1] = (dg.min(1.0) * 255.0) as u8;
                    buffer[idx + 2] = (db.min(1.0) * 255.0) as u8;
                    buffer[idx + 3] = (da * 255.0) as u8;
                }
                2 => {
                    let dst_r = buffer[idx] as f32 / 255.0;
                    let dst_g = buffer[idx + 1] as f32 / 255.0;
                    let dst_b = buffer[idx + 2] as f32 / 255.0;
                    let dst_a = buffer[idx + 3] as f32 / 255.0;
                    let sa = pixel_a;
                    let sr = if sa > 0.001 { r } else { 0.0 };
                    let sg = if sa > 0.001 { g } else { 0.0 };
                    let sb = if sa > 0.001 { b } else { 0.0 };
                    let out_a = sa + dst_a * (1.0 - sa);
                    if out_a > 0.001 {
                        let out_r =
                            (sr * sa + (1.0 - sa) * dst_r * dst_a + sa * (1.0 - dst_a) * sr)
                                / out_a;
                        let out_g =
                            (sg * sa + (1.0 - sa) * dst_g * dst_a + sa * (1.0 - dst_a) * sg)
                                / out_a;
                        let out_b =
                            (sb * sa + (1.0 - sa) * dst_b * dst_a + sa * (1.0 - dst_a) * sb)
                                / out_a;
                        buffer[idx] = (out_r.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 1] = (out_g.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 2] = (out_b.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
                _ => {
                    let dst_a = buffer[idx + 3] as f32 / 255.0;
                    let out_a = pixel_a + dst_a * (1.0 - pixel_a);
                    if out_a > 0.001 {
                        let out_r =
                            (src_r + buffer[idx] as f32 / 255.0 * dst_a * (1.0 - pixel_a)) / out_a;
                        let out_g = (src_g
                            + buffer[idx + 1] as f32 / 255.0 * dst_a * (1.0 - pixel_a))
                            / out_a;
                        let out_b = (src_b
                            + buffer[idx + 2] as f32 / 255.0 * dst_a * (1.0 - pixel_a))
                            / out_a;
                        buffer[idx] = (out_r.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 1] = (out_g.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 2] = (out_b.clamp(0.0, 1.0) * 255.0) as u8;
                        buffer[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_dot_ignores_invalid_raster_inputs() {
        let original = vec![17u8; 16];
        for (center, half, alpha, color) in [
            ([f32::NAN, 1.0], 2.0, 1.0, [1.0, 1.0, 1.0]),
            ([1.0, f32::INFINITY], 2.0, 1.0, [1.0, 1.0, 1.0]),
            ([1.0, 1.0], f32::NAN, 1.0, [1.0, 1.0, 1.0]),
            ([1.0, 1.0], 2.0, f32::NAN, [1.0, 1.0, 1.0]),
            ([1.0, 1.0], 2.0, 1.0, [f32::INFINITY, 1.0, 1.0]),
        ] {
            let mut pixels = original.clone();
            draw_dot(&mut pixels, (2, 2), (center[0], center[1]), half, color, alpha, 1);
            assert_eq!(pixels, original);
        }
    }

    #[test]
    fn test_particle_emitter_defaults() {
        let e = ParticleEmitter::default();
        assert_eq!(e.shape, EmitterShape::Point);
        assert_eq!(e.blend_mode, 1);
        assert!(e.rate > 0.0);
        assert!(e.lifetime > 0.0);
        assert_eq!(e.vortex_strength, 0.0);
        assert_eq!(e.attract_strength, 0.0);
    }

    #[test]
    fn test_vortex_induces_tangential_velocity() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            vortex_strength: 500.0,
            vortex_center: [100.0, 100.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 200.0,
            y: 100.0,
            vx: 0.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(200.0, 100.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.update(0.1, 0.0, 0.0);
        let p = &sys.particles[0];
        // Particle right of center: CW tangent is downward (+vy)
        assert!(
            p.vy > 10.0,
            "vortex must induce tangential +vy, got {}",
            p.vy
        );
        assert!(
            p.vx.abs() < 1.0,
            "no radial velocity expected, got {}",
            p.vx
        );
    }

    #[test]
    fn test_attraction_pulls_particle_toward_point() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            attract_strength: 800.0,
            attract_center: [300.0, 300.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 100.0,
            y: 300.0,
            vx: 0.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(100.0, 300.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.update(0.1, 0.0, 0.0);
        let p = &sys.particles[0];
        // Pulled toward +x (attractor at x=300)
        assert!(
            p.vx > 20.0,
            "attraction must accelerate toward point, got {}",
            p.vx
        );
    }

    #[test]
    fn test_repulsion_pushes_particle_away() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            attract_strength: -800.0,
            attract_center: [100.0, 300.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 110.0,
            y: 300.0,
            vx: 0.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(110.0, 300.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.update(0.1, 0.0, 0.0);
        let p = &sys.particles[0];
        // Negative strength pushes away from attractor (+x direction here)
        assert!(
            p.vx > 20.0,
            "repulsion must push away from point, got {}",
            p.vx
        );
    }

    #[test]
    fn test_death_spawn_emits_children() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            lifetime: 1.0,
            death_spawn_count: 4,
            max_particles: 100,
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        // One particle at end of life
        sys.particles.push(Particle {
            x: 50.0,
            y: 60.0,
            vx: 0.0,
            vy: 0.0,
            life: 0.001,
            max_life: 1.0,
            size: 8.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(50.0, 60.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.update(0.016, 0.0, 0.0);
        // Parent dies → replaced by exactly 4 children near (50, 60)
        assert_eq!(sys.particles.len(), 4, "expected 4 children");
        for c in &sys.particles {
            assert!((c.x - 50.0).abs() < 2.0 && (c.y - 60.0).abs() < 2.0);
            assert!(c.life > 0.0 && c.max_life <= 0.55, "child lifetime scaled");
        }
    }

    #[test]
    fn test_death_spawn_respects_max_particles() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            death_spawn_count: 10,
            max_particles: 3,
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        for i in 0..5u32 {
            sys.particles.push(Particle {
                x: i as f32 * 10.0,
                y: 0.0,
                vx: 0.0,
                vy: 0.0,
                life: 0.001,
                max_life: 1.0,
                size: 4.0,
                rotation: 0.0,
                angular_velocity: 0.0,
                trail: [(i as f32 * 10.0, 0.0); 8],
                trail_len: 0,
                z: 0.0,
            });
        }
        sys.update(0.016, 0.0, 0.0);
        assert!(
            sys.particles.len() <= 3,
            "must cap at max_particles, got {}",
            sys.particles.len()
        );
    }

    #[test]
    fn test_camera_projection_near_particle_larger() {
        // Camera at z=-1000 looking toward +z; focal 100
        let proj = CameraProjection {
            cam_x: 500.0,
            cam_y: 400.0,
            cam_z: -1000.0,
            focal: 100.0,
            cos_rz: 1.0,
            sin_rz: 0.0,
        };
        let near = proj.project(500.0, 400.0, -900.0).expect("in front"); // dz=100
        let far = proj.project(500.0, 400.0, -500.0).expect("in front"); // dz=500
        assert!(
            near.2 > far.2 * 2.5,
            "nearer particle scales bigger: {} vs {}",
            near.2,
            far.2
        );
        // Centered on camera axis → stays centered
        assert!((near.0 - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_camera_projection_behind_returns_none() {
        let proj = CameraProjection {
            cam_x: 0.0,
            cam_y: 0.0,
            cam_z: -100.0,
            focal: 100.0,
            cos_rz: 1.0,
            sin_rz: 0.0,
        };
        assert!(proj.project(10.0, 10.0, -200.0).is_none(), "behind camera");
    }

    #[test]
    fn test_depth_spawn_assigns_z_within_range() {
        let emitter = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            depth_enabled: true,
            depth_range: [-200.0, 300.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        for _ in 0..32 {
            sys.emit_accumulator = 1.0;
            sys.update(0.001, 100.0, 100.0);
        }
        assert!(!sys.particles.is_empty());
        for p in &sys.particles {
            assert!(
                (-200.0..=300.0).contains(&p.z),
                "spawned z out of range: {}",
                p.z
            );
        }
    }

    #[test]
    fn test_projected_render_scales_dot_size() {
        let e = ParticleEmitter {
            rate: 0.0,
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(e.clone());
        // World (15,-15) with cam focal 100 / dz 50 -> screen ~(30,30) in a
        // 64px buffer; scale factor 20x makes the dot cover most of the frame.
        sys.particles.push(Particle {
            x: 15.0,
            y: -15.0,
            vx: 0.0,
            vy: 0.0,
            life: 1.0,
            max_life: 1.0,
            size: 8.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(15.0, -15.0); 8],
            trail_len: 0,
            z: -950.0,
        });
        let mut buf = vec![0u8; 64 * 64 * 4];
        let proj = CameraProjection {
            cam_x: 0.0,
            cam_y: 0.0,
            cam_z: -1000.0,
            focal: 100.0,
            cos_rz: 1.0,
            sin_rz: 0.0,
        };
        sys.render_projected(&mut buf, 64, 64, 0.0, Some(&proj));
        let lit = buf.chunks_exact(4).filter(|c| c[3] > 0).count();
        assert!(lit > 400, "depth-scaled dot must be large, got {lit} px");
    }

    #[test]
    fn test_particle_system_emit() {
        let mut ps = ParticleSystem::new(ParticleEmitter::default());
        ps.emit_accumulator = 10.0; // Force emit
        ps.update(0.01, 100.0, 100.0);
        assert!(!ps.particles.is_empty());
    }

    #[test]
    fn test_particle_lifetime() {
        let emitter = ParticleEmitter {
            rate: 1000.0,
            lifetime: 0.1,
            lifetime_variance: 0.0,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.update(1.0, 0.0, 0.0);
        let count_before = ps.particles.len();
        ps.update(0.5, 0.0, 0.0);
        // After 0.5s, particles with 0.1s lifetime should be dead
        assert!(ps.particles.len() <= count_before);
    }

    #[test]
    fn test_particle_render() {
        let emitter = ParticleEmitter {
            rate: 100.0,
            size_start: 10.0,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.update(0.1, 50.0, 50.0);
        let mut buf = vec![0u8; 100 * 100 * 4];
        ps.render(&mut buf, 100, 100, 0.0);
        // At least some pixels should be non-zero
        assert!(buf.iter().any(|&b| b > 0));
    }

    #[test]
    fn test_fade_curve() {
        assert_eq!(FadeCurve::Linear.apply(0.0), 1.0);
        assert_eq!(FadeCurve::Linear.apply(1.0), 0.0);
        // EaseIn fades slowly at first: higher factor than linear mid-life
        assert!(FadeCurve::EaseIn.apply(0.5) > FadeCurve::Linear.apply(0.5));
        // EaseOut fades fast at first: lower factor than linear mid-life
        assert!(FadeCurve::EaseOut.apply(0.5) < FadeCurve::Linear.apply(0.5));
    }

    #[test]
    fn test_fade_curve_affects_render() {
        let mut buf_linear = vec![0u8; 100 * 100 * 4];
        let mut buf_ease = vec![0u8; 100 * 100 * 4];

        for (buf, curve) in [
            (&mut buf_linear, FadeCurve::Linear),
            (&mut buf_ease, FadeCurve::EaseIn),
        ] {
            let emitter = ParticleEmitter {
                rate: 100.0,
                lifetime: 10.0,
                lifetime_variance: 0.0,
                size_start: 20.0,
                fade_curve: curve,
                ..Default::default()
            };
            let mut ps = ParticleSystem::new(emitter);
            ps.update(0.5, 50.0, 50.0);
            ps.render(buf, 100, 100, 0.0);
        }
        // EaseIn particles are still mostly opaque at t=0.05, so brighter than linear
        let sum = |b: &[u8]| b.iter().map(|&v| v as u64).sum::<u64>();
        assert!(sum(&buf_ease) >= sum(&buf_linear));
    }

    #[test]
    fn test_spin_initial_rotation_and_variance() {
        let emitter = ParticleEmitter {
            rate: 1000.0,
            rotation_start: 45.0,
            rotation_speed: 90.0,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.update(0.01, 0.0, 0.0);
        assert!(!ps.particles.is_empty());
        for p in &ps.particles {
            assert!((p.rotation - (45.0 + 90.0 * 0.01)).abs() < 0.001);
        }

        // Variance gives per-particle angular velocities but stays deterministic
        let make = || {
            ParticleSystem::new(ParticleEmitter {
                rate: 1000.0,
                rotation_start: 0.0,
                rotation_speed: 90.0,
                rotation_speed_variance: 0.5,
                ..Default::default()
            })
        };
        let mut a = make();
        let mut b = make();
        a.update(0.05, 0.0, 0.0);
        b.update(0.05, 0.0, 0.0);
        let rots: Vec<f32> = a.particles.iter().map(|p| p.rotation).collect();
        assert!(!rots.is_empty());
        assert!(
            rots.windows(2).any(|w| (w[0] - w[1]).abs() > f32::EPSILON),
            "variance should differ per particle"
        );
        for (pa, pb) in a.particles.iter().zip(b.particles.iter()) {
            assert_eq!(pa.rotation, pb.rotation);
        }
    }

    #[test]
    fn test_emitter_shapes() {
        for shape in [
            EmitterShape::Point,
            EmitterShape::Box,
            EmitterShape::Circle,
            EmitterShape::Line,
        ] {
            let emitter = ParticleEmitter {
                shape,
                rate: 10.0,
                ..Default::default()
            };
            let mut ps = ParticleSystem::new(emitter);
            ps.update(1.0, 50.0, 50.0);
            assert!(
                !ps.particles.is_empty(),
                "Shape {:?} should emit particles",
                shape
            );
        }
    }

    #[test]
    fn test_gravity_curve_accelerates_fall() {
        let make = |curve: LifeCurve| {
            ParticleSystem::new(ParticleEmitter {
                rate: 1000.0,
                lifetime: 10.0,
                lifetime_variance: 0.0,
                speed: 0.0,
                speed_variance: 0.0,
                spread_degrees: 0.0,
                gravity: [0.0, 100.0],
                gravity_curve: curve,
                ..Default::default()
            })
        };
        // Constant gravity x2 vs constant x1.
        let mut weak = make(LifeCurve::constant(1.0));
        let mut strong = make(LifeCurve::constant(2.0));
        weak.update(0.5, 0.0, 0.0);
        strong.update(0.5, 0.0, 0.0);
        let y_weak: f32 =
            weak.particles.iter().map(|p| p.y).sum::<f32>() / weak.particles.len().max(1) as f32;
        let y_strong: f32 = strong.particles.iter().map(|p| p.y).sum::<f32>()
            / strong.particles.len().max(1) as f32;
        assert!(
            y_strong > y_weak * 1.5,
            "curve-multiplied gravity should fall faster"
        );

        // Determinism: identical configs produce identical results.
        let mut a = make(LifeCurve(vec![0.0, 3.0]));
        let mut b = make(LifeCurve(vec![0.0, 3.0]));
        a.update(0.3, 0.0, 0.0);
        b.update(0.3, 0.0, 0.0);
        let ya: Vec<f32> = a.particles.iter().map(|p| p.y).collect();
        let yb: Vec<f32> = b.particles.iter().map(|p| p.y).collect();
        assert_eq!(ya, yb);
    }

    #[test]
    fn test_wind_gust_changes_velocity_over_time() {
        let emitter = ParticleEmitter {
            rate: 1000.0,
            lifetime: 10.0,
            lifetime_variance: 0.0,
            speed: 0.0,
            speed_variance: 0.0,
            spread_degrees: 0.0,
            gravity: [0.0, 0.0],
            wind: [100.0, 0.0],
            wind_gust_strength: 80.0,
            wind_gust_frequency: 1.0,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.emit_accumulator = 5.0;
        ps.update(0.01, 0.0, 0.0); // t=0.01 → gust ≈ sin(2π*0.01) > 0
        let vx_early = ps.particles[0].vx;
        let mut ps2 = ParticleSystem::new(ParticleEmitter {
            rate: 1000.0,
            lifetime: 10.0,
            lifetime_variance: 0.0,
            speed: 0.0,
            speed_variance: 0.0,
            spread_degrees: 0.0,
            gravity: [0.0, 0.0],
            wind: [100.0, 0.0],
            wind_gust_strength: 80.0,
            wind_gust_frequency: 1.0,
            ..Default::default()
        });
        ps2.emit_accumulator = 5.0;
        // Fast-forward to half a gust period later.
        for _ in 0..50 {
            ps2.update(0.01, 0.0, 0.0);
        }
        let vx_late =
            ps2.particles.iter().map(|p| p.vx).sum::<f32>() / ps2.particles.len().max(1) as f32;
        assert!(
            (vx_early - vx_late).abs() > 1.0,
            "gust should modulate wind over time"
        );
    }

    #[test]
    fn test_collision_bounces_on_floor() {
        let emitter = ParticleEmitter {
            rate: 1000.0,
            lifetime: 10.0,
            lifetime_variance: 0.0,
            speed: 0.0,
            speed_variance: 0.0,
            spread_degrees: 0.0,
            gravity: [0.0, 500.0],
            collision_enabled: true,
            collision_bounds: [-1000.0, -1000.0, 1000.0, 200.0],
            restitution: 0.6,
            surface_friction: 0.9,
            ..Default::default()
        };
        let mut ps = ParticleSystem::new(emitter);
        ps.emit_accumulator = 5.0;
        for _ in 0..120 {
            ps.update(0.05, 0.0, 0.0);
        }
        assert!(!ps.particles.is_empty());
        for p in &ps.particles {
            assert!(p.y <= 200.0 + 1e-3, "particle fell through floor: {}", p.y);
        }
        // At least one particle must have bounced (upward velocity after contact).
        assert!(
            ps.particles.iter().any(|p| p.vy < -1.0),
            "no bounce detected"
        );
    }

    #[test]
    fn test_drag_slows_particles() {
        let make = |drag: f32| {
            ParticleSystem::new(ParticleEmitter {
                rate: 1000.0,
                lifetime: 10.0,
                lifetime_variance: 0.0,
                speed: 300.0,
                speed_variance: 0.0,
                spread_degrees: 0.0,
                gravity: [0.0, 0.0],
                drag,
                ..Default::default()
            })
        };
        let mut free = make(0.0);
        let mut damped = make(4.0);
        free.update(1.0, 0.0, 0.0);
        damped.update(1.0, 0.0, 0.0);
        let v_free = free.particles[0].vx;
        let v_damped = damped.particles[0].vx;
        assert!(v_free > v_damped, "drag must reduce velocity");
        assert!(v_damped > 0.0);
    }

    #[test]
    fn test_emitter_serde_backward_compat_without_new_fields() {
        // Old project JSON lacking the physics-extension fields must deserialize.
        let json = r#"{
            "rate": 30.0, "max_particles": 500, "lifetime": 1.5,
            "lifetime_variance": 0.1, "speed": 150.0, "speed_variance": 0.2,
            "spread_degrees": 180.0, "shape": "Point",
            "emitter_size": [50.0, 50.0], "gravity": [0.0, 200.0],
            "wind": [0.0, 0.0], "turbulence": 0.0,
            "color_start": [1.0, 1.0, 1.0, 1.0], "color_end": [0.0, 0.0, 0.0, 0.0],
            "size_start": 6.0, "size_end": 1.0,
            "opacity_start": 1.0, "opacity_end": 0.0, "blend_mode": 1
        }"#;
        let e: ParticleEmitter = serde_json::from_str(json).unwrap_or_default();
        assert_eq!(e.rate, 30.0);
        assert_eq!(e.gravity_curve, LifeCurve::default());
        assert!(!e.collision_enabled);
        assert_eq!(e.drag, 0.0);
        assert_eq!(e.wind_gust_strength, 0.0);
    }

    #[test]
    fn test_simulation_is_deterministic_with_all_forces() {
        let make = || {
            ParticleSystem::new(ParticleEmitter {
                rate: 500.0,
                lifetime: 3.0,
                gravity_curve: LifeCurve(vec![0.2, 1.5, 0.8]),
                wind: [60.0, -20.0],
                wind_gust_strength: 40.0,
                wind_gust_frequency: 2.0,
                drag: 0.8,
                turbulence: 15.0,
                collision_enabled: true,
                collision_bounds: [0.0, 0.0, 800.0, 600.0],
                restitution: 0.55,
                surface_friction: 0.85,
                ..Default::default()
            })
        };
        let mut a = make();
        let mut b = make();
        for _ in 0..60 {
            a.update(1.0 / 30.0, 400.0, 100.0);
        }
        for _ in 0..60 {
            b.update(1.0 / 30.0, 400.0, 100.0);
        }
        assert_eq!(a.particles.len(), b.particles.len());
        for (pa, pb) in a.particles.iter().zip(b.particles.iter()) {
            assert_eq!(pa.x, pb.x);
            assert_eq!(pa.y, pb.y);
            assert_eq!(pa.vx, pb.vx);
            assert_eq!(pa.vy, pb.vy);
        }
    }
}

#[cfg(test)]
mod particle_collision_tests {
    use super::*;
    use crate::core::particle_forces::LifeCurve as LC;

    fn headon_system() -> ParticleSystem {
        let mut emitter = ParticleEmitter {
            rate: 0.0, // no emission — inject manually
            gravity: [0.0, 0.0],
            wind: [0.0, 0.0],
            drag: 0.0,
            turbulence: 0.0,
            collision_enabled: false,
            particle_collisions: true,
            particle_diameter: 10.0,
            restitution: 1.0,
            ..Default::default()
        };
        emitter.gravity_curve = LC::constant(1.0);
        let mut sys = ParticleSystem::new(emitter);
        sys.particles.push(Particle {
            x: 46.0,
            y: 50.0,
            vx: 30.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(46.0, 50.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys.particles.push(Particle {
            x: 54.0,
            y: 50.0,
            vx: -30.0,
            vy: 0.0,
            life: 5.0,
            max_life: 5.0,
            size: 4.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            trail: [(54.0, 50.0); 8],
            trail_len: 0,
            z: 0.0,
        });
        sys
    }

    #[test]
    fn test_system_resolves_particle_particle_collision() {
        let mut sys = headon_system();
        sys.update(0.016, 0.0, 0.0);
        let (a, b) = (&sys.particles[0], &sys.particles[1]);
        // Elastic exchange: each took the other's velocity.
        assert!((a.vx + 30.0).abs() < 1e-3, "a.vx {}", a.vx);
        assert!((b.vx - 30.0).abs() < 1e-3, "b.vx {}", b.vx);
        // Separated to at least the contact diameter.
        let dist = (b.x - a.x).hypot(b.y - a.y);
        assert!(dist >= 9.9, "post distance {dist}");
    }

    #[test]
    fn test_flag_off_leaves_overlapping_pair_alone() {
        let mut sys = headon_system();
        sys.emitter.particle_collisions = false;
        let before = (sys.particles[0].vx, sys.particles[1].vx);
        sys.update(0.016, 0.0, 0.0);
        assert_eq!(sys.particles[0].vx, before.0, "flag off → untouched");
    }
}
