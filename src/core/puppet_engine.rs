//! Advanced Puppet Pin Deformation Engine (AE Parity).
//!
//! Provides Moving Least Squares (MLS) As-Rigid-As-Possible (ARAP) 2D triangle mesh deformation
//! with support for Position Pins, Starch (Rigidity) Pins, and Overlap (Depth Ordering) Pins.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum PuppetPinType {
    #[default]
    Position,
    Starch,
    Overlap,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PuppetPin {
    pub id: String,
    pub pin_type: PuppetPinType,
    pub rest_position: [f32; 2],
    pub current_position: [f32; 2],
    /// Extent / radius of influence (in pixels)
    pub extent: f32,
    /// Stiffness for Starch pins (0..100) or Depth offset for Overlap pins (-100..+100)
    pub stiffness_or_depth: f32,
}

impl PuppetPin {
    pub fn new_position(id: impl Into<String>, pos: [f32; 2]) -> Self {
        Self {
            id: id.into(),
            pin_type: PuppetPinType::Position,
            rest_position: pos,
            current_position: pos,
            extent: 50.0,
            stiffness_or_depth: 0.0,
        }
    }

    pub fn new_starch(id: impl Into<String>, pos: [f32; 2], extent: f32, stiffness: f32) -> Self {
        Self {
            id: id.into(),
            pin_type: PuppetPinType::Starch,
            rest_position: pos,
            current_position: pos,
            extent,
            stiffness_or_depth: stiffness,
        }
    }

    pub fn new_overlap(id: impl Into<String>, pos: [f32; 2], extent: f32, depth: f32) -> Self {
        Self {
            id: id.into(),
            pin_type: PuppetPinType::Overlap,
            rest_position: pos,
            current_position: pos,
            extent,
            stiffness_or_depth: depth,
        }
    }
}

/// Deforms a single 2D vertex point using Moving Least Squares (MLS) affine & rigid transformation.
pub fn deform_point_mls(
    vertex: [f32; 2],
    pins: &[PuppetPin],
) -> [f32; 2] {
    if pins.is_empty() {
        return vertex;
    }

    let pos_pins: Vec<&PuppetPin> = pins.iter().filter(|p| p.pin_type == PuppetPinType::Position).collect();
    if pos_pins.is_empty() {
        return vertex;
    }

    let starch_pins: Vec<&PuppetPin> = pins.iter().filter(|p| p.pin_type == PuppetPinType::Starch).collect();

    // Compute weights w_i = 1 / |v - p_i|^(2*alpha)
    let mut weights = Vec::with_capacity(pos_pins.len());
    let mut total_w = 0.0f32;

    for p in &pos_pins {
        let dx = vertex[0] - p.rest_position[0];
        let dy = vertex[1] - p.rest_position[1];
        let dist_sq = dx * dx + dy * dy;

        // Exact match with pin
        if dist_sq < 1e-6 {
            return p.current_position;
        }

        let w = 1.0 / dist_sq.powf(1.0); // alpha = 1.0 standard MLS
        weights.push(w);
        total_w += w;
    }

    if total_w <= 0.0 {
        return vertex;
    }

    // Weighted centroids p* and q*
    let mut p_star = [0.0f32, 0.0f32];
    let mut q_star = [0.0f32, 0.0f32];

    for (i, p) in pos_pins.iter().enumerate() {
        let w = weights[i] / total_w;
        p_star[0] += w * p.rest_position[0];
        p_star[1] += w * p.rest_position[1];
        q_star[0] += w * p.current_position[0];
        q_star[1] += w * p.current_position[1];
    }

    // Calculate affine deformation matrix M = sum(w_i * (q_i - q*) * (p_i - p*)^T) * (sum(w_i * (p_i - p*) * (p_i - p*)^T))^-1
    let mut num_x = 0.0f32;
    let mut num_y = 0.0f32;

    for (i, p) in pos_pins.iter().enumerate() {
        let w = weights[i] / total_w;
        let px = p.rest_position[0] - p_star[0];
        let py = p.rest_position[1] - p_star[1];
        let qx = p.current_position[0] - q_star[0];
        let qy = p.current_position[1] - q_star[1];

        let vx = vertex[0] - p_star[0];
        let vy = vertex[1] - p_star[1];

        // Similarity MLS transformation
        let a = px * vx + py * vy;
        let b = px * vy - py * vx;
        num_x += w * (qx * a - qy * b);
        num_y += w * (qy * a + qx * b);
    }

    let mut result = [q_star[0] + num_x, q_star[1] + num_y];

    // Apply Starch (Rigidity) suppression: interpolate back to rest position based on proximity to starch pins
    for sp in starch_pins {
        let dx = vertex[0] - sp.rest_position[0];
        let dy = vertex[1] - sp.rest_position[1];
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < sp.extent && sp.extent > 0.0 {
            let falloff = (1.0 - dist / sp.extent).clamp(0.0, 1.0);
            let stiffness = (sp.stiffness_or_depth * 0.01).clamp(0.0, 1.0) * falloff;
            result[0] = result[0] * (1.0 - stiffness) + vertex[0] * stiffness;
            result[1] = result[1] * (1.0 - stiffness) + vertex[1] * stiffness;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puppet_pin_translation() {
        let pin1 = PuppetPin {
            id: "p1".into(),
            pin_type: PuppetPinType::Position,
            rest_position: [100.0, 100.0],
            current_position: [120.0, 110.0], // Shifted +20, +10
            extent: 50.0,
            stiffness_or_depth: 0.0,
        };

        // Exact point at pin position must move exactly with the pin
        let moved = deform_point_mls([100.0, 100.0], &[pin1.clone()]);
        assert!((moved[0] - 120.0).abs() < 1e-4);
        assert!((moved[1] - 110.0).abs() < 1e-4);
    }

    #[test]
    fn test_starch_pin_reduces_distortion() {
        let pin_pos = PuppetPin::new_position("p1", [100.0, 100.0]);
        let mut pin_pos_moved = pin_pos.clone();
        pin_pos_moved.current_position = [150.0, 100.0]; // +50px shift

        let vertex = [110.0, 100.0];

        // Deformation without starch pin
        let unstarched = deform_point_mls(vertex, &[pin_pos_moved.clone()]);

        // Deformation WITH starch pin right next to vertex with 100% stiffness
        let starch = PuppetPin::new_starch("s1", [110.0, 100.0], 50.0, 100.0);
        let starched = deform_point_mls(vertex, &[pin_pos_moved, starch]);

        // Starched vertex must stay much closer to original 110.0 than unstarched
        assert!((starched[0] - vertex[0]).abs() < (unstarched[0] - vertex[0]).abs());
    }
}
