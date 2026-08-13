/// Kinematic Physics Engine for AE Motion Graphics layers, particle emitters, and dynamics.
/// Ported & adapted from NextVFX Sovereign Engine.

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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

        // Apply drag damping
        let damp = self.drag.powf(dt * 60.0);
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
}
