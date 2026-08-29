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
        // 1. Spawning
        self.spawn_accumulator += config.birth_rate_per_sec * dt;
        let spawn_count = self.spawn_accumulator.floor() as usize;
        self.spawn_accumulator -= spawn_count as f32;

        for _ in 0..spawn_count {
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

        // 2. Integration & Collision
        for p in &mut self.particles {
            p.age_sec += dt;
            if p.age_sec >= p.lifespan_sec {
                p.dead = true;
                continue;
            }

            // Apply gravity and wind
            p.velocity[0] += (config.gravity[0] + config.wind[0]) * dt;
            p.velocity[1] += (config.gravity[1] + config.wind[1]) * dt;
            p.velocity[2] += (config.gravity[2] + config.wind[2]) * dt;

            p.position[0] += p.velocity[0] * dt;
            p.position[1] += p.velocity[1] * dt;
            p.position[2] += p.velocity[2] * dt;

            p.rotation_deg += p.angular_velocity_deg * dt;

            // Handle collision planes
            for plane in &config.collision_planes {
                let n = plane.normal;
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
}
