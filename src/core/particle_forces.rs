#![allow(dead_code)]
//! Extended particle forces: lifetime curves, wind gusts, air drag and
//! boundary collisions. Pure deterministic functions consumed by
//! [`crate::core::particle_system::ParticleSystem`].

use serde::{Deserialize, Serialize};

/// Control-point curve sampled over normalized lifetime (t = 0 birth .. 1 death).
///
/// Stored as an ordered list of control values; value at t is the linear
/// interpolation between neighbouring control points. An empty curve
/// evaluates to 1.0 (neutral multiplier).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifeCurve(pub Vec<f32>);

impl Default for LifeCurve {
    fn default() -> Self {
        Self(vec![1.0])
    }
}

impl LifeCurve {
    /// Constant curve.
    pub fn constant(v: f32) -> Self {
        Self(vec![v])
    }

    /// Sample the curve at normalized lifetime progress `t` (clamped 0..1).
    pub fn eval(&self, t: f32) -> f32 {
        let pts = &self.0;
        if pts.is_empty() {
            return 1.0;
        }
        if pts.len() == 1 {
            return pts[0];
        }
        let t = t.clamp(0.0, 1.0);
        let scaled = t * (pts.len() - 1) as f32;
        let i = scaled.floor() as usize;
        let f = scaled - i as f32;
        let a = pts[i.min(pts.len() - 1)];
        let b = pts[(i + 1).min(pts.len() - 1)];
        a + (b - a) * f
    }
}

/// Wind including a sinusoidal gust component travelling along the base wind
/// direction. When the base wind is zero the gust blows along +X.
pub fn wind_with_gust(base: [f32; 2], gust_strength: f32, gust_frequency_hz: f32, time: f32) -> [f32; 2] {
    if gust_strength == 0.0 {
        return base;
    }
    let mag = (base[0] * base[0] + base[1] * base[1]).sqrt();
    let dir = if mag > 1e-6 { [base[0] / mag, base[1] / mag] } else { [1.0, 0.0] };
    let gust = (std::f32::consts::TAU * gust_frequency_hz * time).sin() * gust_strength;
    [base[0] + dir[0] * gust, base[1] + dir[1] * gust]
}

/// Exponential air drag: stable for any dt (v *= 1 / (1 + drag*dt)).
pub fn apply_drag(vx: &mut f32, vy: &mut f32, drag: f32, dt: f32) {
    if drag <= 0.0 || dt <= 0.0 {
        return;
    }
    let damp = 1.0 / (1.0 + drag * dt);
    *vx *= damp;
    *vy *= damp;
}

/// Resolve a particle against an axis-aligned boundary box.
///
/// `bounds` = [min_x, min_y, max_x, max_y]. On contact the normal velocity
/// component is reflected scaled by `restitution` (0 = dead stop, 1 = perfect
/// bounce) and the tangential component is scaled by `friction`.
/// Returns true when any contact occurred this step.
pub fn resolve_bounds_collision(
    pos: &mut [f32; 2],
    vel: &mut [f32; 2],
    bounds: [f32; 4],
    restitution: f32,
    friction: f32,
) -> bool {
    let (min_x, min_y, max_x, max_y) = (bounds[0], bounds[1], bounds[2], bounds[3]);
    if min_x > max_x || min_y > max_y {
        return false;
    }
    let mut hit = false;

    if pos[0] < min_x {
        pos[0] = min_x;
        if vel[0] < 0.0 {
            vel[0] = -vel[0] * restitution;
            vel[1] *= friction;
            hit = true;
        }
    } else if pos[0] > max_x {
        pos[0] = max_x;
        if vel[0] > 0.0 {
            vel[0] = -vel[0] * restitution;
            vel[1] *= friction;
            hit = true;
        }
    }

    if pos[1] < min_y {
        pos[1] = min_y;
        if vel[1] < 0.0 {
            vel[1] = -vel[1] * restitution;
            vel[0] *= friction;
            hit = true;
        }
    } else if pos[1] > max_y {
        pos[1] = max_y;
        if vel[1] > 0.0 {
            vel[1] = -vel[1] * restitution;
            vel[0] *= friction;
            hit = true;
        }
    }

    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_life_curve_interpolation() {
        assert_eq!(LifeCurve::default().eval(0.5), 1.0);
        assert_eq!(LifeCurve(vec![]).eval(0.7), 1.0);
        let c = LifeCurve(vec![0.0, 1.0, 0.0]);
        assert_eq!(c.eval(0.0), 0.0);
        assert_eq!(c.eval(0.5), 1.0);
        assert_eq!(c.eval(1.0), 0.0);
        assert!((c.eval(0.25) - 0.5).abs() < 1e-6);
        // Out-of-range t clamps to endpoints.
        assert_eq!(c.eval(-1.0), 0.0);
        assert_eq!(c.eval(2.0), 0.0);
    }

    #[test]
    fn test_wind_gust_bounded_and_deterministic() {
        let base = [40.0, 0.0];
        let a = wind_with_gust(base, 20.0, 0.5, 0.25);
        let b = wind_with_gust(base, 20.0, 0.5, 0.25);
        assert_eq!(a, b);
        // Gust magnitude never exceeds base + strength.
        for t in 0..100 {
            let w = wind_with_gust(base, 20.0, 0.5, t as f32 * 0.01);
            assert!(w[0] >= 40.0 - 20.0 - 1e-4 && w[0] <= 40.0 + 20.0 + 1e-4);
        }
        // Zero strength returns base exactly.
        assert_eq!(wind_with_gust(base, 0.0, 1.0, 3.0), base);
        // Zero base wind gusts along +X.
        let w = wind_with_gust([0.0, 0.0], 5.0, 1.0, 0.25); // sin(pi/2)=1
        assert!((w[0] - 5.0).abs() < 1e-4);
        assert!(w[1].abs() < 1e-4);
    }

    #[test]
    fn test_drag_decays_velocity() {
        let mut vx = 100.0;
        let mut vy = 0.0;
        for _ in 0..100 {
            apply_drag(&mut vx, &mut vy, 2.0, 0.016);
        }
        assert!(vx < 100.0 && vx > 0.0);
        // Zero drag is a no-op.
        let mut ux = 50.0;
        let mut uy = 50.0;
        apply_drag(&mut ux, &mut uy, 0.0, 0.016);
        assert_eq!(ux, 50.0);
        assert_eq!(uy, 50.0);
    }

    #[test]
    fn test_floor_collision_reflects_with_restitution() {
        let mut pos = [50.0, 105.0];
        let mut vel = [10.0, 40.0];
        let hit = resolve_bounds_collision(&mut pos, &mut vel, [0.0, 0.0, 100.0, 100.0], 0.5, 0.9);
        assert!(hit);
        assert_eq!(pos[1], 100.0);
        assert!((vel[1] + 20.0).abs() < 1e-5); // -40 * 0.5
        assert!((vel[0] - 9.0).abs() < 1e-5); // 10 * 0.9 friction
    }

    #[test]
    fn test_wall_and_ceiling_collision() {
        let mut pos = [-5.0, 50.0];
        let mut vel = [-30.0, 0.0];
        assert!(resolve_bounds_collision(&mut pos, &mut vel, [0.0, 0.0, 100.0, 100.0], 1.0, 1.0));
        assert_eq!(pos[0], 0.0);
        assert_eq!(vel[0], 30.0);

        let mut pos2 = [50.0, -1.0];
        let mut vel2 = [0.0, -10.0];
        assert!(resolve_bounds_collision(&mut pos2, &mut vel2, [0.0, 0.0, 100.0, 100.0], 0.0, 1.0));
        assert_eq!(pos2[1], 0.0);
        assert_eq!(vel2[1], 0.0); // restitution 0 kills the bounce
    }

    #[test]
    fn test_no_contact_returns_false() {
        let mut pos = [50.0, 50.0];
        let mut vel = [1.0, 1.0];
        assert!(!resolve_bounds_collision(&mut pos, &mut vel, [0.0, 0.0, 100.0, 100.0], 0.5, 1.0));
        assert_eq!(pos, [50.0, 50.0]);
        assert_eq!(vel, [1.0, 1.0]);
        // Inverted bounds are ignored defensively.
        let mut p2 = [50.0, 50.0];
        let mut v2 = [1.0, 1.0];
        assert!(!resolve_bounds_collision(&mut p2, &mut v2, [100.0, 100.0, 0.0, 0.0], 0.5, 1.0));
    }

    #[test]
    fn test_life_curve_serde_roundtrip() {
        let c = LifeCurve(vec![1.0, 0.5, 2.0]);
        let json = serde_json::to_string(&c).unwrap_or_default();
        let back: LifeCurve = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(c, back);
    }
}