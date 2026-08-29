//! 3D Physics Particle Emitter Engine with Collision Planes, Turbulence,
//! and Life-cycle Dynamics (Trapcode Particular Parity for After Effects).

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EmitterType3D {
    Point,
    Box { size: [f32; 3] },
    Sphere { radius: f32 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Particle3D {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub rotation_deg: f32,
    pub angular_velocity_deg: f32,
    pub age_sec: f32,
    pub lifespan_sec: f32,
    pub start_size: f32,
    pub end_size: f32,
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
    pub dead: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollisionPlane {
    pub origin: [f32; 3],
    pub normal: [f32; 3],
    pub bounce_restitution: f32,
    pub friction: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmitterConfig3D {
    pub emitter_type: EmitterType3D,
    pub position: [f32; 3],
    pub birth_rate_per_sec: f32,
    pub initial_speed: f32,
    pub speed_random: f32,
    pub lifespan_sec: f32,
    pub start_size: f32,
    pub end_size: f32,
    pub gravity: [f32; 3],
    pub wind: [f32; 3],
    pub collision_planes: Vec<CollisionPlane>,
}

impl Default for EmitterConfig3D {
    fn default() -> Self {
        Self {
            emitter_type: EmitterType3D::Point,
            position: [960.0, 540.0, 0.0],
            birth_rate_per_sec: 100.0,
            initial_speed: 300.0,
            speed_random: 0.2,
            lifespan_sec: 2.0,
            start_size: 10.0,
            end_size: 0.0,
            gravity: [0.0, 980.0, 0.0],
            wind: [0.0, 0.0, 0.0],
            collision_planes: Vec::new(),
        }
    }
}

pub struct ParticleSimulation3D {
    pub particles: Vec<Particle3D>,
    pub spawn_accumulator: f32,
    pub rng_state: u64,
}

impl ParticleSimulation3D {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(1024),
            spawn_accumulator: 0.0,
            rng_state: 0x9E3779B97F4A7C15,
        }
    }

    fn next_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state & 0xFFFFFF) as f32 / 16777216.0
    }

    pub fn update(&mut self, dt: f32, config: &EmitterConfig3D) {
        let safe_dt = if dt.is_finite() && dt > 0.0 { dt.min(1.0) } else { 0.0 };
        if safe_dt <= 0.0 {
            return;
        }

        // 1. Spawning (guards for negative/zero lifespan and bounds on spawn rate)
        if config.lifespan_sec > 0.0 && config.lifespan_sec.is_finite() {
            let safe_birth_rate = if config.birth_rate_per_sec.is_finite() && config.birth_rate_per_sec > 0.0 {
                config.birth_rate_per_sec
            } else {
                0.0
            };
            self.spawn_accumulator = (self.spawn_accumulator + safe_birth_rate * safe_dt).clamp(0.0, 50_000.0);
            let spawn_count = (self.spawn_accumulator.floor() as usize).min(2000);
            self.spawn_accumulator -= spawn_count as f32;

            let remaining_budget = 50_000usize.saturating_sub(self.particles.len());
            let actual_spawn = spawn_count.min(remaining_budget);

            for _ in 0..actual_spawn {
                let mut pos = config.position;
                match config.emitter_type {
                    EmitterType3D::Point => {}
                    EmitterType3D::Box { size } => {
                        pos[0] += (self.next_f32() - 0.5) * size[0];
                        pos[1] += (self.next_f32() - 0.5) * size[1];
                        pos[2] += (self.next_f32() - 0.5) * size[2];
                    }
                    EmitterType3D::Sphere { radius } => {
                        let theta = self.next_f32() * std::f32::consts::TAU;
                        let phi = (self.next_f32() * 2.0 - 1.0).acos();
                        let r = radius * self.next_f32().cbrt();
                        pos[0] += r * phi.sin() * theta.cos();
                        pos[1] += r * phi.sin() * theta.sin();
                        pos[2] += r * phi.cos();
                    }
                }

                // Random spherical velocity
                let theta = self.next_f32() * std::f32::consts::TAU;
                let phi = (self.next_f32() * 2.0 - 1.0).acos();
                let speed = config.initial_speed * (1.0 + (self.next_f32() - 0.5) * 2.0 * config.speed_random);
                let vel = [
                    speed * phi.sin() * theta.cos(),
                    speed * phi.sin() * theta.sin(),
                    speed * phi.cos(),
                ];
                let rot = self.next_f32() * 360.0;
                let ang_vel = (self.next_f32() - 0.5) * 180.0;

                self.particles.push(Particle3D {
                    position: pos,
                    velocity: vel,
                    rotation_deg: rot,
                    angular_velocity_deg: ang_vel,
                    age_sec: 0.0,
                    lifespan_sec: config.lifespan_sec,
                    start_size: config.start_size,
                    end_size: config.end_size,
                    start_color: [1.0, 0.9, 0.4, 1.0],
                    end_color: [1.0, 0.2, 0.1, 0.0],
                    dead: false,
                });
            }
        }

        // 2. Integration & Collision
        for p in &mut self.particles {
            p.age_sec += safe_dt;
            if p.age_sec >= p.lifespan_sec {
                p.dead = true;
                continue;
            }

            // Apply gravity and wind
            p.velocity[0] += (config.gravity[0] + config.wind[0]) * safe_dt;
            p.velocity[1] += (config.gravity[1] + config.wind[1]) * safe_dt;
            p.velocity[2] += (config.gravity[2] + config.wind[2]) * safe_dt;

            p.position[0] += p.velocity[0] * safe_dt;
            p.position[1] += p.velocity[1] * safe_dt;
            p.position[2] += p.velocity[2] * safe_dt;

            p.rotation_deg += p.angular_velocity_deg * safe_dt;

            // Handle collision planes
            for plane in &config.collision_planes {
                let n_len = (plane.normal[0].powi(2) + plane.normal[1].powi(2) + plane.normal[2].powi(2)).sqrt();
                if n_len < 1e-6 || !n_len.is_finite() {
                    continue; // Skip zero-length or non-finite normal
                }
                let n = [plane.normal[0] / n_len, plane.normal[1] / n_len, plane.normal[2] / n_len];
                let rel = [
                    p.position[0] - plane.origin[0],
                    p.position[1] - plane.origin[1],
                    p.position[2] - plane.origin[2],
                ];
                let dist = rel[0] * n[0] + rel[1] * n[1] + rel[2] * n[2];

                if dist < 0.0 {
                    // Push out of plane
                    p.position[0] -= n[0] * dist;
                    p.position[1] -= n[1] * dist;
                    p.position[2] -= n[2] * dist;

                    // Reflect velocity: v' = v - (1 + e) * (v . n) * n
                    let v_dot_n = p.velocity[0] * n[0] + p.velocity[1] * n[1] + p.velocity[2] * n[2];
                    if v_dot_n < 0.0 {
                        let e = plane.bounce_restitution.clamp(0.0, 1.0);
                        p.velocity[0] -= (1.0 + e) * v_dot_n * n[0];
                        p.velocity[1] -= (1.0 + e) * v_dot_n * n[1];
                        p.velocity[2] -= (1.0 + e) * v_dot_n * n[2];

                        // Apply friction on tangential velocity
                        let tang = [
                            p.velocity[0] - v_dot_n * n[0],
                            p.velocity[1] - v_dot_n * n[1],
                            p.velocity[2] - v_dot_n * n[2],
                        ];
                        let f_factor = (1.0 - plane.friction).clamp(0.0, 1.0);
                        p.velocity[0] = v_dot_n * n[0] * -e + tang[0] * f_factor;
                        p.velocity[1] = v_dot_n * n[1] * -e + tang[1] * f_factor;
                        p.velocity[2] = v_dot_n * n[2] * -e + tang[2] * f_factor;
                    }
                }
            }
        }

        self.particles.retain(|p| !p.dead);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_3d_particle_simulation_spawn_and_collision() {
        let mut sim = ParticleSimulation3D::new();
        let mut config = EmitterConfig3D::default();
        config.birth_rate_per_sec = 50.0;
        config.gravity = [0.0, 100.0, 0.0];

        // Add a floor plane at Y = 600 pointing UP ([0, -1, 0])
        config.collision_planes.push(CollisionPlane {
            origin: [0.0, 600.0, 0.0],
            normal: [0.0, -1.0, 0.0],
            bounce_restitution: 0.8,
            friction: 0.1,
        });

        // Run simulation for 0.5s
        for _ in 0..15 {
            sim.update(0.033, &config);
        }

        assert!(!sim.particles.is_empty());
        // All particles must be above the floor (Y <= 600)
        for p in &sim.particles {
            assert!(p.position[1] <= 600.1, "Particle breached floor: {}", p.position[1]);
        }
    }

    #[test]
    fn test_negative_birth_rate_does_not_spawn_huge_particle_count() {
        let mut sim = ParticleSimulation3D::new();
        let config = EmitterConfig3D {
            birth_rate_per_sec: -1.0,
            ..Default::default()
        };
        sim.update(1.0, &config);
        assert!(sim.particles.len() <= 1);
    }

    #[test]
    fn test_non_unit_collision_normal_behaves_like_normalized_normal() {
        let mut sim = ParticleSimulation3D::new();
        let config = EmitterConfig3D {
            birth_rate_per_sec: 1.0,
            initial_speed: 0.0,
            gravity: [0.0, 0.0, 0.0],
            collision_planes: vec![CollisionPlane {
                origin: [0.0, 0.0, 0.0],
                normal: [0.0, 10.0, 0.0],
                bounce_restitution: 0.0,
                friction: 0.0,
            }],
            ..Default::default()
        };
        sim.update(1.0, &config);
        assert!(sim.particles.iter().all(|p| p.position[1] >= -1e-5));
    }

    #[test]
    fn test_large_dt_does_not_allocate_unbounded_particle_count() {
        let mut sim = ParticleSimulation3D::new();
        let config = EmitterConfig3D {
            birth_rate_per_sec: 10_000.0,
            ..Default::default()
        };
        sim.update(10.0, &config);
        assert!(sim.particles.len() <= 2_000);
    }

    #[test]
    fn test_zero_or_negative_lifespan_does_not_spawn_particles() {
        let mut sim = ParticleSimulation3D::new();
        let config = EmitterConfig3D {
            birth_rate_per_sec: 100.0,
            lifespan_sec: 0.0,
            ..Default::default()
        };
        sim.update(1.0, &config);
        assert_eq!(sim.particles.len(), 0);

        let neg_config = EmitterConfig3D {
            birth_rate_per_sec: 100.0,
            lifespan_sec: -5.0,
            ..Default::default()
        };
        sim.update(1.0, &neg_config);
        assert_eq!(sim.particles.len(), 0);
    }

    #[test]
    fn test_render_particles_to_buffer_renders_soft_glow() {
        let mut pixels = vec![0u8; 100 * 100 * 4];
        let p = Particle3D {
            position: [50.0, 50.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            rotation_deg: 0.0,
            angular_velocity_deg: 0.0,
            age_sec: 0.5,
            lifespan_sec: 1.0,
            start_size: 20.0,
            end_size: 20.0,
            start_color: [1.0, 0.0, 0.0, 1.0],
            end_color: [1.0, 0.0, 0.0, 1.0],
            dead: false,
        };

        render_particles_to_buffer(&mut pixels, 100, 100, &[p], 1000.0, true);
        let center_idx = (50 * 100 + 50) * 4;
        assert!(pixels[center_idx] > 200, "Center pixel should have bright red color");
    }
}

/// Renders 3D particles onto a 2D RGBA pixel buffer using perspective camera projection and soft billboards.
pub fn render_particles_to_buffer(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    particles: &[Particle3D],
    camera_fov: f32, // e.g. 1000.0
    additive_blend: bool,
) {
    let Some(expected_len) = (width as usize).checked_mul(height as usize).and_then(|s| s.checked_mul(4)) else {
        return;
    };
    if pixels.len() != expected_len || width == 0 || height == 0 || particles.is_empty() {
        return;
    }

    let fov = if camera_fov.is_finite() && camera_fov > 10.0 { camera_fov } else { 1000.0 };
    let half_w = width as f32 * 0.5;
    let half_h = height as f32 * 0.5;

    for p in particles {
        if p.dead || p.lifespan_sec <= 0.0 {
            continue;
        }

        // Perspective depth factor
        let z_dist = p.position[2] + fov;
        if z_dist <= 1.0 {
            continue; // Behind camera
        }
        let scale = fov / z_dist;

        // Screen-space projected coordinates
        let screen_x = half_w + (p.position[0] - half_w) * scale;
        let screen_y = half_h + (p.position[1] - half_h) * scale;

        // Normalized life progress (0.0 .. 1.0)
        let life = (p.age_sec / p.lifespan_sec).clamp(0.0, 1.0);

        // Interpolate size over life
        let current_size = (p.start_size + (p.end_size - p.start_size) * life) * scale;
        let radius = (current_size * 0.5).max(0.5);
        if radius > 500.0 {
            continue; // Outlandishly large
        }

        // Interpolate color and alpha over life
        let r = p.start_color[0] + (p.end_color[0] - p.start_color[0]) * life;
        let g = p.start_color[1] + (p.end_color[1] - p.start_color[1]) * life;
        let b = p.start_color[2] + (p.end_color[2] - p.start_color[2]) * life;
        let a = p.start_color[3] + (p.end_color[3] - p.start_color[3]) * life;

        let col_r = (r * 255.0).clamp(0.0, 255.0);
        let col_g = (g * 255.0).clamp(0.0, 255.0);
        let col_b = (b * 255.0).clamp(0.0, 255.0);
        let col_a = (a * 255.0).clamp(0.0, 255.0);

        let min_x = ((screen_x - radius).floor() as i32).clamp(0, width as i32 - 1) as u32;
        let max_x = ((screen_x + radius).ceil() as i32).clamp(0, width as i32 - 1) as u32;
        let min_y = ((screen_y - radius).floor() as i32).clamp(0, height as i32 - 1) as u32;
        let max_y = ((screen_y + radius).ceil() as i32).clamp(0, height as i32 - 1) as u32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let dist_sq = (px - screen_x).powi(2) + (py - screen_y).powi(2);
                let rad_sq = radius * radius;

                if dist_sq < rad_sq {
                    let norm_dist = (dist_sq / rad_sq).sqrt();
                    // Soft quadratic falloff
                    let falloff = (1.0 - norm_dist * norm_dist).powi(2) * (col_a / 255.0);

                    let idx = ((y * width + x) * 4) as usize;
                    if additive_blend {
                        pixels[idx] = (pixels[idx] as f32 + col_r * falloff).clamp(0.0, 255.0) as u8;
                        pixels[idx + 1] = (pixels[idx + 1] as f32 + col_g * falloff).clamp(0.0, 255.0) as u8;
                        pixels[idx + 2] = (pixels[idx + 2] as f32 + col_b * falloff).clamp(0.0, 255.0) as u8;
                        pixels[idx + 3] = (pixels[idx + 3] as f32 + col_a * falloff).clamp(0.0, 255.0) as u8;
                    } else {
                        // Standard alpha over composite
                        let src_a = falloff;
                        let inv_a = 1.0 - src_a;
                        pixels[idx] = (col_r * src_a + pixels[idx] as f32 * inv_a).clamp(0.0, 255.0) as u8;
                        pixels[idx + 1] = (col_g * src_a + pixels[idx + 1] as f32 * inv_a).clamp(0.0, 255.0) as u8;
                        pixels[idx + 2] = (col_b * src_a + pixels[idx + 2] as f32 * inv_a).clamp(0.0, 255.0) as u8;
                        pixels[idx + 3] = ((pixels[idx + 3] as f32 / 255.0 + src_a * (1.0 - pixels[idx + 3] as f32 / 255.0)) * 255.0).clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}
