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
pub fn wind_with_gust(
    base: [f32; 2],
    gust_strength: f32,
    gust_frequency_hz: f32,
    time: f32,
) -> [f32; 2] {
    if gust_strength == 0.0 {
        return base;
    }
    let mag = (base[0] * base[0] + base[1] * base[1]).sqrt();
    let dir = if mag > 1e-6 {
        [base[0] / mag, base[1] / mag]
    } else {
        [1.0, 0.0]
    };
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

/// Vortex orbital force: pulls particles inward while accelerating them tangentially.
pub fn vortex_force(
    pos: [f32; 2],
    center: [f32; 2],
    inward_pull: f32,
    tangential_speed: f32,
) -> [f32; 2] {
    let dx = pos[0] - center[0];
    let dy = pos[1] - center[1];
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1.0 {
        return [0.0, 0.0];
    }
    let ndx = dx / dist;
    let ndy = dy / dist;

    // Radial inward acceleration: -dir * inward_pull
    let fx_rad = -ndx * inward_pull;
    let fy_rad = -ndy * inward_pull;

    // Tangential perpendicular acceleration: [-ndy, ndx] * tangential_speed
    let fx_tan = -ndy * tangential_speed;
    let fy_tan = ndx * tangential_speed;

    [fx_rad + fx_tan, fy_rad + fy_tan]
}

/// Divergence-free 2D Curl Noise turbulence force for natural fluid/smoke motion.
pub fn curl_noise_turbulence(pos: [f32; 2], frequency: f32, amplitude: f32) -> [f32; 2] {
    if amplitude <= 0.0 || frequency <= 0.0 {
        return [0.0, 0.0];
    }
    let px = pos[0] * frequency;
    let py = pos[1] * frequency;

    // Numerical gradient of potential field: Psi = sin(px)*cos(py) + cos(px*1.3)*sin(py*1.3)*0.5
    let eps = 0.01f32;
    let psi_up = (px).sin() * (py + eps).cos() + (px * 1.3).cos() * ((py + eps) * 1.3).sin() * 0.5;
    let psi_down = (px).sin() * (py - eps).cos() + (px * 1.3).cos() * ((py - eps) * 1.3).sin() * 0.5;
    let psi_right = (px + eps).sin() * (py).cos() + ((px + eps) * 1.3).cos() * (py * 1.3).sin() * 0.5;
    let psi_left = (px - eps).sin() * (py).cos() + ((px - eps) * 1.3).cos() * (py * 1.3).sin() * 0.5;

    let d_psi_dy = (psi_up - psi_down) / (2.0 * eps);
    let d_psi_dx = (psi_right - psi_left) / (2.0 * eps);

    // Curl in 2D: [dPsi/dy, -dPsi/dx] (guaranteed div=0)
    [d_psi_dy * amplitude, -d_psi_dx * amplitude]
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

/// Resolves an elastic collision between two soft-sphere particles of equal
/// mass. On overlap the pair is separated symmetrically along the contact
/// normal and normal-relative velocity is reflected with `restitution`
/// (0 = perfectly inelastic, 1 = perfectly elastic).
///
/// Returns true when a collision was resolved this call.
#[allow(clippy::too_many_arguments)]
pub fn resolve_particle_collision(
    pos_a: &mut [f32; 2],
    vel_a: &mut [f32; 2],
    pos_b: &mut [f32; 2],
    vel_b: &mut [f32; 2],
    radius_sum: f32,
    restitution: f32,
) -> bool {
    if radius_sum <= 0.0 {
        return false;
    }
    let dx = pos_b[0] - pos_a[0];
    let dy = pos_b[1] - pos_a[1];
    let dist_sq = dx * dx + dy * dy;
    if dist_sq >= radius_sum * radius_sum {
        return false;
    }

    // Contact normal; degenerate coincident particles split along +X.
    let dist = dist_sq.sqrt();
    let (nx, ny) = if dist > 1e-6 {
        (dx / dist, dy / dist)
    } else {
        (1.0, 0.0)
    };

    // Symmetric positional correction.
    let overlap = radius_sum - dist;
    let half = overlap * 0.5;
    pos_a[0] -= nx * half;
    pos_a[1] -= ny * half;
    pos_b[0] += nx * half;
    pos_b[1] += ny * half;

    // Impulse along the normal for approaching pairs only.
    let rvx = vel_b[0] - vel_a[0];
    let rvy = vel_b[1] - vel_a[1];
    let vn = rvx * nx + rvy * ny;
    if vn < 0.0 {
        let e = restitution.clamp(0.0, 1.0);
        let j = -(1.0 + e) * vn * 0.5; // equal masses
        vel_a[0] -= nx * j;
        vel_a[1] -= ny * j;
        vel_b[0] += nx * j;
        vel_b[1] += ny * j;
    }
    true
}

/// Convenience O(n²) pass applying [`resolve_particle_collision`] to every
/// pair sharing a uniform `radius` (diameter per particle). Suitable for the
/// few-hundred-particle counts typical of AE-style emitters.
pub fn resolve_pairwise_collisions(
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    diameter: f32,
    restitution: f32,
) -> u32 {
    let n = positions.len().min(velocities.len());
    let mut hits = 0u32;
    for i in 0..n {
        let (p_head, p_tail) = positions.split_at_mut(i + 1);
        let (v_head, v_tail) = velocities.split_at_mut(i + 1);
        for (bp, bv) in p_tail.iter_mut().zip(v_tail.iter_mut()) {
            if resolve_particle_collision(
                &mut p_head[i],
                &mut v_head[i],
                bp,
                bv,
                diameter,
                restitution,
            ) {
                hits += 1;
            }
        }
    }
    hits
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
        assert!(resolve_bounds_collision(
            &mut pos,
            &mut vel,
            [0.0, 0.0, 100.0, 100.0],
            1.0,
            1.0
        ));
        assert_eq!(pos[0], 0.0);
        assert_eq!(vel[0], 30.0);

        let mut pos2 = [50.0, -1.0];
        let mut vel2 = [0.0, -10.0];
        assert!(resolve_bounds_collision(
            &mut pos2,
            &mut vel2,
            [0.0, 0.0, 100.0, 100.0],
            0.0,
            1.0
        ));
        assert_eq!(pos2[1], 0.0);
        assert_eq!(vel2[1], 0.0); // restitution 0 kills the bounce
    }

    #[test]
    fn test_no_contact_returns_false() {
        let mut pos = [50.0, 50.0];
        let mut vel = [1.0, 1.0];
        assert!(!resolve_bounds_collision(
            &mut pos,
            &mut vel,
            [0.0, 0.0, 100.0, 100.0],
            0.5,
            1.0
        ));
        assert_eq!(pos, [50.0, 50.0]);
        assert_eq!(vel, [1.0, 1.0]);
        // Inverted bounds are ignored defensively.
        let mut p2 = [50.0, 50.0];
        let mut v2 = [1.0, 1.0];
        assert!(!resolve_bounds_collision(
            &mut p2,
            &mut v2,
            [100.0, 100.0, 0.0, 0.0],
            0.5,
            1.0
        ));
    }

    #[test]
    fn test_life_curve_serde_roundtrip() {
        let c = LifeCurve(vec![1.0, 0.5, 2.0]);
        let json = serde_json::to_string(&c).unwrap_or_default();
        let back: LifeCurve = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(c, back);
    }

    #[test]
    fn test_head_on_elastic_collision_swaps_velocities() {
        // Equal masses, restitution 1: velocities exchange exactly.
        let mut pa = [40.0, 50.0];
        let mut va = [30.0, 0.0];
        let mut pb = [60.0, 50.0];
        let mut vb = [-30.0, 0.0];
        assert!(resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 24.0, 1.0
        ));
        assert!(
            (va[0] + 30.0).abs() < 1e-4,
            "a takes b's velocity: {}",
            va[0]
        );
        assert!(
            (vb[0] - 30.0).abs() < 1e-4,
            "b takes a's velocity: {}",
            vb[0]
        );
        // Particles separated to contact distance.
        let dist = (pb[0] - pa[0]).hypot(pb[1] - pa[1]);
        assert!((dist - 24.0).abs() < 1e-3, "post distance {dist}");
    }

    #[test]
    fn test_inelastic_collision_kills_relative_normal_velocity() {
        let mut pa = [10.0, 10.0];
        let mut va = [20.0, 0.0];
        let mut pb = [26.0, 10.0];
        let mut vb = [0.0, 0.0];
        assert!(resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 20.0, 0.0
        ));
        let vn_after = (vb[0] - va[0]) * 1.0 + (vb[1] - va[1]) * 0.0;
        assert!(
            vn_after.abs() < 1e-4,
            "normal relative velocity must vanish"
        );
        // Both move right together afterwards (momentum conserved).
        assert!(va[0] > 5.0 && vb[0] > 5.0);
    }

    #[test]
    fn test_separating_pair_is_untouched() {
        // Already separating along the normal → no impulse.
        let mut pa = [0.0, 0.0];
        let mut va = [-10.0, 0.0];
        let mut pb = [8.0, 0.0]; // overlapping
        let mut vb = [10.0, 0.0];
        assert!(resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 12.0, 1.0
        ));
        // Positions still separated, velocities unchanged.
        assert_eq!(va, [-10.0, 0.0]);
        assert_eq!(vb, [10.0, 0.0]);
    }

    #[test]
    fn test_far_apart_and_degenerate_inputs() {
        let mut pa = [0.0, 0.0];
        let mut va = [1.0, 1.0];
        let mut pb = [100.0, 100.0];
        let mut vb = [-1.0, -1.0];
        assert!(!resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 10.0, 0.8
        ));
        assert_eq!(pa, [0.0, 0.0]);
        // Zero radius is always false.
        assert!(!resolve_particle_collision(
            &mut pa, &mut va, &mut pb, &mut vb, 0.0, 1.0
        ));
        // Coincident particles split cleanly without NaN.
        let mut qa = [50.0, 50.0];
        let mut qva = [0.0, 0.0];
        let mut qb = [50.0, 50.0];
        let mut qvb = [0.0, 0.0];
        assert!(resolve_particle_collision(
            &mut qa, &mut qva, &mut qb, &mut qvb, 8.0, 1.0
        ));
        assert!(qa[0].is_finite() && qb[0].is_finite());
        assert!((qb[0] - qa[0]).abs() >= 7.9);
    }

    #[test]
    fn test_pairwise_batch_counts_and_resolves() {
        // Three particles in a row, first one slams into the middle one.
        let mut pos = vec![[0.0, 0.0], [15.0, 0.0], [90.0, 0.0]];
        let mut vel = vec![[50.0, 0.0], [0.0, 0.0], [0.0, 0.0]];
        let hits = resolve_pairwise_collisions(&mut pos, &mut vel, 16.0, 0.9);
        assert_eq!(hits, 1, "only the close pair collides");
        // Momentum conserved along x.
        let total_before = 50.0;
        let total_after = vel[0][0] + vel[1][0];
        assert!((total_after - total_before).abs() < 1e-3);
        // Third particle untouched.
        assert_eq!(vel[2], [0.0, 0.0]);
        // Empty input safe.
        let mut ep: Vec<[f32; 2]> = vec![];
        let mut ev: Vec<[f32; 2]> = vec![];
        assert_eq!(resolve_pairwise_collisions(&mut ep, &mut ev, 4.0, 1.0), 0);
    }

    #[test]
    fn test_vortex_force_pulls_inward_and_tangential() {
        let f = vortex_force([100.0, 0.0], [0.0, 0.0], 10.0, 20.0);
        // Point is on +X axis relative to center (0,0)
        // Inward pull should be along -X (-10.0)
        // Tangential speed should be along +Y (+20.0)
        assert!((f[0] + 10.0).abs() < 1e-3);
        assert!((f[1] - 20.0).abs() < 1e-3);
    }

    #[test]
    fn test_curl_noise_turbulence_produces_finite_vectors() {
        let f = curl_noise_turbulence([50.0, 50.0], 0.05, 10.0);
        assert!(f[0].is_finite());
        assert!(f[1].is_finite());
    }
}
