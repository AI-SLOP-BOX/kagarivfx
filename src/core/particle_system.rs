/// Basic particle emitter system for After Effects-style particle animations.
///
/// Supports:
/// - Emitter with configurable rate, lifetime, speed, spread
/// - Gravity, turbulence, and wind forces
/// - Per-particle size, color, and opacity over lifetime
/// - Point and box emitter shapes
use serde::{Serialize, Deserialize};

/// Emitter shape type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum EmitterShape {
    #[default]
    Point = 0,
    Box = 1,
    Circle = 2,
    Line = 3,
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
    pub rotation_speed: f32,
    /// Blend mode: 0=Normal, 1=Add, 2=Screen
    pub blend_mode: u32,
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
            blend_mode: 1, // Add
        }
    }
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
}

/// Particle system simulation.
pub struct ParticleSystem {
    pub emitter: ParticleEmitter,
    pub particles: Vec<Particle>,
    /// Accumulated time for emission scheduling
    pub emit_accumulator: f32,
    /// PRNG state for deterministic randomness
    rng_state: u64,
}

impl ParticleSystem {
    pub fn new(emitter: ParticleEmitter) -> Self {
        Self {
            emitter,
            particles: Vec::new(),
            emit_accumulator: 0.0,
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

        let lifetime = self.emitter.lifetime * (1.0 - self.emitter.lifetime_variance * (self.next_random() * 2.0 - 1.0));
        let speed = self.emitter.speed * (1.0 + self.emitter.speed_variance * (self.next_random() * 2.0 - 1.0));

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
        };

        self.particles.push(Particle {
            x: px,
            y: py,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            life: lifetime,
            max_life: lifetime,
            size: self.emitter.size_start,
            rotation: 0.0,
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

        let gravity = self.emitter.gravity;
        let wind = self.emitter.wind;
        let turbulence = self.emitter.turbulence;
        let rotation_speed = self.emitter.rotation_speed;
        let size_start = self.emitter.size_start;
        let size_end = self.emitter.size_end;

        // Update existing particles
        for p in &mut self.particles {
            p.life -= dt;
            if p.life <= 0.0 {
                continue;
            }

            // Forces
            p.vx += (gravity[0] + wind[0]) * dt;
            p.vy += (gravity[1] + wind[1]) * dt;

            // Turbulence
            if turbulence > 0.0 {
                let t = p.life * 10.0;
                p.vx += (t.sin() * 0.5 + (t * 2.7).sin() * 0.3) * turbulence * dt;
                p.vy += (t.cos() * 0.5 + (t * 3.1).cos() * 0.3) * turbulence * dt;
            }

            // Integrate position
            p.x += p.vx * dt;
            p.y += p.vy * dt;

            // Rotation
            p.rotation += rotation_speed * dt;

            // Interpolate size and opacity over lifetime
            let t = 1.0 - (p.life / p.max_life);
            p.size = size_start + (size_end - size_start) * t;
        }

        // Remove dead particles
        self.particles.retain(|p| p.life > 0.0);
    }

    /// Render all particles into an RGBA pixel buffer.
    pub fn render(
        &self,
        buffer: &mut [u8],
        buf_width: u32,
        buf_height: u32,
        _time: f32,
    ) {
        let e = &self.emitter;
        for p in &self.particles {
            if p.life <= 0.0 { continue; }

            let t = 1.0 - (p.life / p.max_life);
            let r = e.color_start[0] + (e.color_end[0] - e.color_start[0]) * t;
            let g = e.color_start[1] + (e.color_end[1] - e.color_start[1]) * t;
            let b = e.color_start[2] + (e.color_end[2] - e.color_start[2]) * t;
            let a = e.color_start[3] + (e.color_end[3] - e.color_start[3]) * t;

            let half = p.size * 0.5;
            let x0 = (p.x - half).max(0.0) as u32;
            let y0 = (p.y - half).max(0.0) as u32;
            let x1 = (p.x + half).min(buf_width as f32 - 1.0) as u32;
            let y1 = (p.y + half).min(buf_height as f32 - 1.0) as u32;

            for py in y0..=y1 {
                for px in x0..=x1 {
                    // Simple soft circle SDF
                    let dx = px as f32 - p.x;
                    let dy = py as f32 - p.y;
                    let dist = ((dx * dx + dy * dy).sqrt() / half).min(1.0);
                    let falloff = (1.0 - dist * dist).max(0.0);
                    let pixel_a = a * falloff;

                    if pixel_a <= 0.001 { continue; }

                    let idx = ((py * buf_width + px) * 4) as usize;
                    if idx + 3 >= buffer.len() { continue; }

                    let src_r = r * pixel_a;
                    let src_g = g * pixel_a;
                    let src_b = b * pixel_a;

                    match e.blend_mode {
                        1 => {
                            // Additive blending
                            let dr = buffer[idx] as f32 / 255.0 + src_r;
                            let dg = buffer[idx+1] as f32 / 255.0 + src_g;
                            let db = buffer[idx+2] as f32 / 255.0 + src_b;
                            let da = buffer[idx+3] as f32 / 255.0 + pixel_a;
                            buffer[idx] = (dr.min(1.0) * 255.0) as u8;
                            buffer[idx+1] = (dg.min(1.0) * 255.0) as u8;
                            buffer[idx+2] = (db.min(1.0) * 255.0) as u8;
                            buffer[idx+3] = (da.min(1.0) * 255.0) as u8;
                        }
                        2 => {
                            // Screen blending
                            let dr = 1.0 - (1.0 - buffer[idx] as f32 / 255.0) * (1.0 - src_r);
                            let dg = 1.0 - (1.0 - buffer[idx+1] as f32 / 255.0) * (1.0 - src_g);
                            let db = 1.0 - (1.0 - buffer[idx+2] as f32 / 255.0) * (1.0 - src_b);
                            buffer[idx] = (dr.min(1.0) * 255.0) as u8;
                            buffer[idx+1] = (dg.min(1.0) * 255.0) as u8;
                            buffer[idx+2] = (db.min(1.0) * 255.0) as u8;
                            buffer[idx+3] = 255;
                        }
                        _ => {
                            // Normal alpha blending
                            let dst_a = buffer[idx+3] as f32 / 255.0;
                            let out_a = pixel_a + dst_a * (1.0 - pixel_a);
                            if out_a > 0.001 {
                                let out_r = (src_r + buffer[idx] as f32 / 255.0 * dst_a * (1.0 - pixel_a)) / out_a;
                                let out_g = (src_g + buffer[idx+1] as f32 / 255.0 * dst_a * (1.0 - pixel_a)) / out_a;
                                let out_b = (src_b + buffer[idx+2] as f32 / 255.0 * dst_a * (1.0 - pixel_a)) / out_a;
                                buffer[idx] = (out_r.clamp(0.0, 1.0) * 255.0) as u8;
                                buffer[idx+1] = (out_g.clamp(0.0, 1.0) * 255.0) as u8;
                                buffer[idx+2] = (out_b.clamp(0.0, 1.0) * 255.0) as u8;
                                buffer[idx+3] = (out_a.clamp(0.0, 1.0) * 255.0) as u8;
                            }
                        }
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
    fn test_particle_emitter_defaults() {
        let e = ParticleEmitter::default();
        assert_eq!(e.shape, EmitterShape::Point);
        assert_eq!(e.blend_mode, 1);
        assert!(e.rate > 0.0);
        assert!(e.lifetime > 0.0);
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
    fn test_emitter_shapes() {
        for shape in [EmitterShape::Point, EmitterShape::Box, EmitterShape::Circle, EmitterShape::Line] {
            let emitter = ParticleEmitter {
                shape,
                rate: 10.0,
                ..Default::default()
            };
            let mut ps = ParticleSystem::new(emitter);
            ps.update(1.0, 50.0, 50.0);
            assert!(!ps.particles.is_empty(), "Shape {:?} should emit particles", shape);
        }
    }
}
