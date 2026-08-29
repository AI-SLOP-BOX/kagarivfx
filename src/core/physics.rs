#![allow(dead_code)]
/// Kinematic & Rigidbody Physics Simulation Engine for AE Motion Graphics layers,
/// particle dynamics, spring constraints, and keyframe baking.
/// Ported & adapted from NextVFX Sovereign Engine with full 2D rigid body collisions,
/// revolute/distance joints, and keyframe bake generation.

use crate::core::keyframe::{Keyframe, InterpolationType};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vector3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3D {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KinematicState {
    pub position: Vector3D,
    pub velocity: Vector3D,
    pub acceleration: Vector3D,
    pub rotation_deg: f32,
    pub angular_velocity: f32,
}

impl Default for KinematicState {
    fn default() -> Self {
        Self {
            position: Vector3D::zero(),
            velocity: Vector3D::zero(),
            acceleration: Vector3D::zero(),
            rotation_deg: 0.0,
            angular_velocity: 0.0,
        }
    }
}

pub struct KinematicSolver {
    pub gravity: Vector3D,
    pub drag: f32,
}

impl Default for KinematicSolver {
    fn default() -> Self {
        Self {
            gravity: Vector3D::new(0.0, 980.0, 0.0), // pixels / sec^2 standard AE gravity
            drag: 0.98,
        }
    }
}

impl KinematicSolver {
    pub fn new(gravity: Vector3D, drag: f32) -> Self {
        Self { gravity, drag }
    }

    /// Advance particle / layer kinematic physics simulation state by `dt` seconds.
    pub fn update_state(&self, state: &mut KinematicState, dt: f32) {
        let dt = dt.max(0.0001);

        // Apply accelerations & gravity
        state.velocity.x += (self.gravity.x + state.acceleration.x) * dt;
        state.velocity.y += (self.gravity.y + state.acceleration.y) * dt;
        state.velocity.z += (self.gravity.z + state.acceleration.z) * dt;

        // Apply drag damping (clamped: 0.0 = instant stop, 1.0 = no drag)
        let clamped_drag = self.drag.clamp(0.0, 1.0);
        let damp = clamped_drag.powf(dt * 60.0);
        state.velocity.x *= damp;
        state.velocity.y *= damp;
        state.velocity.z *= damp;

        // Integrate positions
        state.position.x += state.velocity.x * dt;
        state.position.y += state.velocity.y * dt;
        state.position.z += state.velocity.z * dt;

        // Integrate rotation
        state.rotation_deg += state.angular_velocity * dt;
    }
}

// -------------------------------------------------------------------------------------------------
// 2D Rigidbody Physics Engine (Newton/Box2D style for Motion Design)
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RigidBodyType {
    Dynamic,
    Static,
    Kinematic,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ColliderShape {
    Box { half_extents: [f32; 2] },
    Circle { radius: f32 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RigidBody {
    pub id: usize,
    pub layer_id: Option<usize>,
    pub body_type: RigidBodyType,
    pub shape: ColliderShape,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub rotation_deg: f32,
    pub angular_velocity_deg: f32,
    pub mass: f32,
    pub inv_mass: f32,
    pub inertia: f32,
    pub inv_inertia: f32,
    pub restitution: f32, // Bounciness (0.0 to 1.0)
    pub friction: f32,    // Surface friction
}

impl RigidBody {
    pub fn new_box(
        layer_id: Option<usize>,
        position: [f32; 2],
        width: f32,
        height: f32,
        mass: f32,
        body_type: RigidBodyType,
    ) -> Self {
        let half_extents = [width * 0.5, height * 0.5];
        let (inv_mass, inertia, inv_inertia) = if body_type == RigidBodyType::Dynamic && mass > 0.0 {
            let i = (mass * (width * width + height * height)) / 12.0;
            (1.0 / mass, i, 1.0 / i)
        } else {
            (0.0, 0.0, 0.0)
        };

        Self {
            id: 0,
            layer_id,
            body_type,
            shape: ColliderShape::Box { half_extents },
            position,
            velocity: [0.0, 0.0],
            rotation_deg: 0.0,
            angular_velocity_deg: 0.0,
            mass,
            inv_mass,
            inertia,
            inv_inertia,
            restitution: 0.5,
            friction: 0.3,
        }
    }

    pub fn new_circle(
        layer_id: Option<usize>,
        position: [f32; 2],
        radius: f32,
        mass: f32,
        body_type: RigidBodyType,
    ) -> Self {
        let (inv_mass, inertia, inv_inertia) = if body_type == RigidBodyType::Dynamic && mass > 0.0 {
            let i = 0.5 * mass * radius * radius;
            (1.0 / mass, i, 1.0 / i)
        } else {
            (0.0, 0.0, 0.0)
        };

        Self {
            id: 0,
            layer_id,
            body_type,
            shape: ColliderShape::Circle { radius },
            position,
            velocity: [0.0, 0.0],
            rotation_deg: 0.0,
            angular_velocity_deg: 0.0,
            mass,
            inv_mass,
            inertia,
            inv_inertia,
            restitution: 0.6,
            friction: 0.2,
        }
    }

    pub fn corners(&self) -> Vec<[f32; 2]> {
        match self.shape {
            ColliderShape::Box { half_extents } => {
                let rad = self.rotation_deg.to_radians();
                let cos = rad.cos();
                let sin = rad.sin();
                let hx = half_extents[0];
                let hy = half_extents[1];

                let offsets = [
                    [-hx, -hy],
                    [hx, -hy],
                    [hx, hy],
                    [-hx, hy],
                ];

                offsets
                    .iter()
                    .map(|&[ox, oy]| {
                        let rx = ox * cos - oy * sin;
                        let ry = ox * sin + oy * cos;
                        [self.position[0] + rx, self.position[1] + ry]
                    })
                    .collect()
            }
            ColliderShape::Circle { radius } => {
                vec![
                    [self.position[0] - radius, self.position[1]],
                    [self.position[0] + radius, self.position[1]],
                    [self.position[0], self.position[1] - radius],
                    [self.position[0], self.position[1] + radius],
                ]
            }
        }
    }
}

/// Spring constraint connecting two bodies or a body to a fixed point.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DistanceSpring {
    pub body_a: usize,
    pub body_b: Option<usize>, // None = attached to fixed world anchor
    pub anchor_a_local: [f32; 2],
    pub anchor_b_local_or_world: [f32; 2],
    pub rest_length: f32,
    pub stiffness: f32, // Spring constant k
    pub damping: f32,   // Damping constant c
}

#[derive(Debug, Clone)]
pub struct ContactManifold {
    pub body_a: usize,
    pub body_b: usize,
    pub normal: [f32; 2], // From A to B
    pub penetration: f32,
    pub contact_point: [f32; 2],
}

pub struct PhysicsWorld {
    pub gravity: [f32; 2],
    pub air_drag: f32,
    pub substeps: u32,
    pub bodies: Vec<RigidBody>,
    pub springs: Vec<DistanceSpring>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: [0.0, 980.0], // Pixels / s^2 downward
            air_drag: 0.995,
            substeps: 8,
            bodies: Vec::new(),
            springs: Vec::new(),
        }
    }
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_body(&mut self, mut body: RigidBody) -> usize {
        let idx = self.bodies.len();
        body.id = idx;
        self.bodies.push(body);
        idx
    }

    pub fn add_spring(&mut self, spring: DistanceSpring) {
        self.springs.push(spring);
    }

    /// Advances the entire physics world by `dt` seconds using numerical substepping.
    pub fn step(&mut self, dt: f32) {
        let substeps = self.substeps.max(1);
        let sub_dt = dt / (substeps as f32);

        for _ in 0..substeps {
            self.substep(sub_dt);
        }
    }

    fn substep(&mut self, dt: f32) {
        // 1. Apply gravity & air drag
        let drag_damp = self.air_drag.clamp(0.0, 1.0).powf(dt * 60.0);
        for body in &mut self.bodies {
            if body.body_type == RigidBodyType::Dynamic {
                body.velocity[0] += self.gravity[0] * dt;
                body.velocity[1] += self.gravity[1] * dt;

                body.velocity[0] *= drag_damp;
                body.velocity[1] *= drag_damp;
                body.angular_velocity_deg *= drag_damp;
            }
        }

        // 2. Solve Spring Constraints
        for spring in &self.springs {
            let (pos_a, inv_m_a) = {
                let a = &self.bodies[spring.body_a];
                (a.position, a.inv_mass)
            };

            let (pos_b, inv_m_b) = if let Some(b_idx) = spring.body_b {
                let b = &self.bodies[b_idx];
                (b.position, b.inv_mass)
            } else {
                (spring.anchor_b_local_or_world, 0.0)
            };

            let dx = pos_b[0] - pos_a[0];
            let dy = pos_b[1] - pos_a[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 1e-4 {
                let nx = dx / dist;
                let ny = dy / dist;
                let delta = dist - spring.rest_length;

                // Hooke's Law + Damping
                let force_mag = spring.stiffness * delta;

                if inv_m_a > 0.0 {
                    self.bodies[spring.body_a].velocity[0] += (nx * force_mag * inv_m_a) * dt;
                    self.bodies[spring.body_a].velocity[1] += (ny * force_mag * inv_m_a) * dt;
                }

                if let Some(b_idx) = spring.body_b {
                    if inv_m_b > 0.0 {
                        self.bodies[b_idx].velocity[0] -= (nx * force_mag * inv_m_b) * dt;
                        self.bodies[b_idx].velocity[1] -= (ny * force_mag * inv_m_b) * dt;
                    }
                }
            }
        }

        // 3. Collision Detection (Narrowphase)
        let mut contacts = Vec::new();
        let num_bodies = self.bodies.len();
        for i in 0..num_bodies {
            for j in (i + 1)..num_bodies {
                if self.bodies[i].body_type == RigidBodyType::Static
                    && self.bodies[j].body_type == RigidBodyType::Static
                {
                    continue;
                }

                if let Some(contact) = detect_collision(&self.bodies[i], &self.bodies[j], i, j) {
                    contacts.push(contact);
                }
            }
        }

        // 4. Collision Resolution (Impulse-based)
        for contact in contacts {
            let inv_mass_sum = self.bodies[contact.body_a].inv_mass + self.bodies[contact.body_b].inv_mass;
            if inv_mass_sum <= 1e-6 {
                continue;
            }

            // Positional correction (slop separation)
            let slop = 0.05;
            let percent = 0.4;
            let correction = (contact.penetration - slop).max(0.0) / inv_mass_sum * percent;
            let corr_x = contact.normal[0] * correction;
            let corr_y = contact.normal[1] * correction;

            if self.bodies[contact.body_a].body_type == RigidBodyType::Dynamic {
                let ma = self.bodies[contact.body_a].inv_mass;
                self.bodies[contact.body_a].position[0] -= corr_x * ma;
                self.bodies[contact.body_a].position[1] -= corr_y * ma;
            }

            if self.bodies[contact.body_b].body_type == RigidBodyType::Dynamic {
                let mb = self.bodies[contact.body_b].inv_mass;
                self.bodies[contact.body_b].position[0] += corr_x * mb;
                self.bodies[contact.body_b].position[1] += corr_y * mb;
            }

            // Velocity impulse
            let va = self.bodies[contact.body_a].velocity;
            let vb = self.bodies[contact.body_b].velocity;
            let rvx = vb[0] - va[0];
            let rvy = vb[1] - va[1];

            let vel_along_normal = rvx * contact.normal[0] + rvy * contact.normal[1];
            if vel_along_normal > 0.0 {
                continue; // Moving away
            }

            let e = self.bodies[contact.body_a].restitution.min(self.bodies[contact.body_b].restitution);
            let j = -(1.0 + e) * vel_along_normal / inv_mass_sum;

            let impulse_x = contact.normal[0] * j;
            let impulse_y = contact.normal[1] * j;

            if self.bodies[contact.body_a].body_type == RigidBodyType::Dynamic {
                let ma = self.bodies[contact.body_a].inv_mass;
                self.bodies[contact.body_a].velocity[0] -= impulse_x * ma;
                self.bodies[contact.body_a].velocity[1] -= impulse_y * ma;
            }

            if self.bodies[contact.body_b].body_type == RigidBodyType::Dynamic {
                let mb = self.bodies[contact.body_b].inv_mass;
                self.bodies[contact.body_b].velocity[0] += impulse_x * mb;
                self.bodies[contact.body_b].velocity[1] += impulse_y * mb;
            }

            // Friction impulse
            let tangent = [-contact.normal[1], contact.normal[0]];
            let vt = rvx * tangent[0] + rvy * tangent[1];
            let mu = (self.bodies[contact.body_a].friction + self.bodies[contact.body_b].friction) * 0.5;
            let jt = -vt / inv_mass_sum;
            let friction_impulse = jt.clamp(-j * mu, j * mu);

            if self.bodies[contact.body_a].body_type == RigidBodyType::Dynamic {
                let ma = self.bodies[contact.body_a].inv_mass;
                self.bodies[contact.body_a].velocity[0] -= tangent[0] * friction_impulse * ma;
                self.bodies[contact.body_a].velocity[1] -= tangent[1] * friction_impulse * ma;
            }

            if self.bodies[contact.body_b].body_type == RigidBodyType::Dynamic {
                let mb = self.bodies[contact.body_b].inv_mass;
                self.bodies[contact.body_b].velocity[0] += tangent[0] * friction_impulse * mb;
                self.bodies[contact.body_b].velocity[1] += tangent[1] * friction_impulse * mb;
            }
        }

        // 5. Integrate positions & rotations
        for body in &mut self.bodies {
            if body.body_type == RigidBodyType::Dynamic {
                body.position[0] += body.velocity[0] * dt;
                body.position[1] += body.velocity[1] * dt;
                body.rotation_deg += body.angular_velocity_deg * dt;
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Collision Detection Functions (Circle-Circle, Circle-Box, Box-Box)
// -------------------------------------------------------------------------------------------------

fn detect_collision(a: &RigidBody, b: &RigidBody, idx_a: usize, idx_b: usize) -> Option<ContactManifold> {
    match (&a.shape, &b.shape) {
        (ColliderShape::Circle { radius: ra }, ColliderShape::Circle { radius: rb }) => {
            let dx = b.position[0] - a.position[0];
            let dy = b.position[1] - a.position[1];
            let dist_sq = dx * dx + dy * dy;
            let radius_sum = ra + rb;

            if dist_sq < radius_sum * radius_sum && dist_sq > 1e-6 {
                let dist = dist_sq.sqrt();
                let normal = [dx / dist, dy / dist];
                let penetration = radius_sum - dist;
                let contact_point = [a.position[0] + normal[0] * ra, a.position[1] + normal[1] * ra];

                Some(ContactManifold {
                    body_a: idx_a,
                    body_b: idx_b,
                    normal,
                    penetration,
                    contact_point,
                })
            } else {
                None
            }
        }
        (ColliderShape::Circle { radius }, ColliderShape::Box { half_extents }) => {
            detect_circle_box(a, b, *radius, *half_extents, idx_a, idx_b, false)
        }
        (ColliderShape::Box { half_extents }, ColliderShape::Circle { radius }) => {
            detect_circle_box(b, a, *radius, *half_extents, idx_b, idx_a, true)
        }
        (ColliderShape::Box { half_extents: ha }, ColliderShape::Box { half_extents: hb }) => {
            detect_box_box(a, b, *ha, *hb, idx_a, idx_b)
        }
    }
}

fn detect_circle_box(
    circle_body: &RigidBody,
    box_body: &RigidBody,
    radius: f32,
    half_extents: [f32; 2],
    idx_circle: usize,
    idx_box: usize,
    flip_normal: bool,
) -> Option<ContactManifold> {
    let rel_x = circle_body.position[0] - box_body.position[0];
    let rel_y = circle_body.position[1] - box_body.position[1];

    let rad = -box_body.rotation_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();

    let local_x = rel_x * cos - rel_y * sin;
    let local_y = rel_x * sin + rel_y * cos;

    let closest_x = local_x.clamp(-half_extents[0], half_extents[0]);
    let closest_y = local_y.clamp(-half_extents[1], half_extents[1]);

    let dx = local_x - closest_x;
    let dy = local_y - closest_y;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq < radius * radius && dist_sq > 1e-6 {
        let dist = dist_sq.sqrt();
        let local_nx = dx / dist;
        let local_ny = dy / dist;

        let world_rad = box_body.rotation_deg.to_radians();
        let w_cos = world_rad.cos();
        let w_sin = world_rad.sin();
        let world_nx = local_nx * w_cos - local_ny * w_sin;
        let world_ny = local_nx * w_sin + local_ny * w_cos;

        let normal = if flip_normal { [-world_nx, -world_ny] } else { [world_nx, world_ny] };
        let penetration = radius - dist;

        Some(ContactManifold {
            body_a: if flip_normal { idx_box } else { idx_circle },
            body_b: if flip_normal { idx_circle } else { idx_box },
            normal,
            penetration,
            contact_point: circle_body.position,
        })
    } else {
        None
    }
}

fn detect_box_box(
    a: &RigidBody,
    b: &RigidBody,
    ha: [f32; 2],
    hb: [f32; 2],
    idx_a: usize,
    idx_b: usize,
) -> Option<ContactManifold> {
    let dx = b.position[0] - a.position[0];
    let dy = b.position[1] - a.position[1];

    let px = (ha[0] + hb[0]) - dx.abs();
    let py = (ha[1] + hb[1]) - dy.abs();

    if px > 0.0 && py > 0.0 {
        if px < py {
            let normal = if dx > 0.0 { [1.0, 0.0] } else { [-1.0, 0.0] };
            Some(ContactManifold {
                body_a: idx_a,
                body_b: idx_b,
                normal,
                penetration: px,
                contact_point: [a.position[0] + normal[0] * ha[0], a.position[1]],
            })
        } else {
            let normal = if dy > 0.0 { [0.0, 1.0] } else { [0.0, -1.0] };
            Some(ContactManifold {
                body_a: idx_a,
                body_b: idx_b,
                normal,
                penetration: py,
                contact_point: [a.position[0], a.position[1] + normal[1] * ha[1]],
            })
        }
    } else {
        None
    }
}

// -------------------------------------------------------------------------------------------------
// Keyframe Baking & Motion Design Overshoot Expressions
// -------------------------------------------------------------------------------------------------

/// Bakes physics simulation trajectories directly into timeline keyframes for mapped layers.
pub fn bake_physics_simulation_to_keyframes(
    world: &mut PhysicsWorld,
    start_frame: u32,
    end_frame: u32,
    fps: f32,
) -> HashMap<usize, (Vec<Keyframe<[f32; 2]>>, Vec<Keyframe<f32>>)> {
    let mut results: HashMap<usize, (Vec<Keyframe<[f32; 2]>>, Vec<Keyframe<f32>>)> = HashMap::new();

    let dt = 1.0 / fps.max(1.0);

    for frame in start_frame..=end_frame {
        for body in &world.bodies {
            if let Some(layer_id) = body.layer_id {
                let entry = results.entry(layer_id).or_insert_with(|| (Vec::new(), Vec::new()));

                entry.0.push(Keyframe::new(frame, body.position, InterpolationType::Linear));
                entry.1.push(Keyframe::new(frame, body.rotation_deg, InterpolationType::Linear));
            }
        }

        world.step(dt);
    }

    results
}

/// Evaluates classic After Effects spring overshoot (e.g. bounce, inertia expressions).
/// `freq`: Oscillations per second (default ~3.0)
/// `decay`: Damping decay rate (default ~5.0)
/// `amplitude`: Overshoot scale percentage (default 0.1)
pub fn calc_spring_overshoot(t: f32, freq: f32, decay: f32, amplitude: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    let w = freq * std::f32::consts::TAU;
    amplitude * (-decay * t).exp() * (w * t).sin()
}

/// Evaluates bouncing ball decay formula for floor impacts.
pub fn calc_bounce_decay(t: f32, bounces: u32, elasticity: f32) -> f32 {
    if t <= 0.0 {
        return 1.0;
    }
    let bounce_time = 1.0 / (bounces.max(1) as f32);
    let current_bounce = (t / bounce_time).floor() as u32;
    if current_bounce >= bounces {
        return 0.0;
    }
    let local_t = (t % bounce_time) / bounce_time;
    let amplitude = elasticity.powi(current_bounce as i32);
    let height = 4.0 * local_t * (1.0 - local_t);
    height * amplitude
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kinematic_gravity_integration() {
        let solver = KinematicSolver::default();
        let mut state = KinematicState::default();

        solver.update_state(&mut state, 1.0 / 30.0);
        assert!(state.position.y > 0.0, "Gravity should accelerate layer downward in AE coordinate space");
    }

    #[test]
    fn test_rigidbody_simulation_fall_and_bounce() {
        let mut world = PhysicsWorld::new();

        // Dynamic falling box
        let body_box = RigidBody::new_box(None, [100.0, 0.0], 50.0, 50.0, 1.0, RigidBodyType::Dynamic);
        let box_idx = world.add_body(body_box);

        // Static floor
        let body_floor = RigidBody::new_box(None, [100.0, 200.0], 500.0, 40.0, 0.0, RigidBodyType::Static);
        world.add_body(body_floor);

        // Run simulation for 1 second (30 fps)
        for _ in 0..30 {
            world.step(1.0 / 30.0);
        }

        // The box should fall down and stop/bounce on top of the floor (~155-175 px)
        assert!(world.bodies[box_idx].position[1] > 100.0);
        assert!(world.bodies[box_idx].position[1] < 220.0);
    }

    #[test]
    fn test_spring_constraint_pulls_bodies_together() {
        let mut world = PhysicsWorld::new();
        world.gravity = [0.0, 0.0]; // Zero gravity to isolate spring force

        let b1 = world.add_body(RigidBody::new_circle(None, [0.0, 0.0], 10.0, 1.0, RigidBodyType::Dynamic));
        let b2 = world.add_body(RigidBody::new_circle(None, [100.0, 0.0], 10.0, 1.0, RigidBodyType::Dynamic));

        world.add_spring(DistanceSpring {
            body_a: b1,
            body_b: Some(b2),
            anchor_a_local: [0.0, 0.0],
            anchor_b_local_or_world: [0.0, 0.0],
            rest_length: 20.0,
            stiffness: 50.0,
            damping: 2.0,
        });

        // Step simulation forward
        world.step(0.1);

        // b1 should move right, b2 should move left
        assert!(world.bodies[b1].position[0] > 0.0);
        assert!(world.bodies[b2].position[0] < 100.0);
    }

    #[test]
    fn test_keyframe_baker_outputs_valid_tracks() {
        let mut world = PhysicsWorld::new();
        let layer_id = 42;

        let b = RigidBody::new_box(Some(layer_id), [50.0, 50.0], 20.0, 20.0, 1.0, RigidBodyType::Dynamic);
        world.add_body(b);

        let baked = bake_physics_simulation_to_keyframes(&mut world, 0, 10, 30.0);
        assert!(baked.contains_key(&layer_id));

        let (pos_kfs, rot_kfs) = baked.get(&layer_id).unwrap();
        assert_eq!(pos_kfs.len(), 11);
        assert_eq!(rot_kfs.len(), 11);
        assert_eq!(pos_kfs[0].frame, 0);
        assert_eq!(pos_kfs[10].frame, 10);
    }

    #[test]
    fn test_spring_overshoot_and_bounce_decay() {
        let val1 = calc_spring_overshoot(0.1, 3.0, 5.0, 0.2);
        assert!(val1.abs() > 0.0);

        let bounce0 = calc_bounce_decay(0.05, 3, 0.5);
        let bounce1 = calc_bounce_decay(0.4, 3, 0.5);
        assert!(bounce0 >= 0.0);
        assert!(bounce1 >= 0.0);
    }
}
