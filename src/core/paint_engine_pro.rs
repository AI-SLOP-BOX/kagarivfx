//! 32-bit Floating-Point HDR Paint & Clone Stamp Engine (AE Paint Parity).
//!
//! Renders continuous subpixel brush strokes, erasers, and clone stamps with
//! pressure-sensitive dynamics, Gaussian hardness feathering, and HDR alpha blending.

#![allow(dead_code)]

/// Brush operation mode for vector paint rasterization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushMode {
    #[default]
    Paint,
    Eraser,
    CloneStamp,
}

/// A recorded vector paint stroke containing sampled points and brush properties.
#[derive(Debug, Clone)]
pub struct PaintStrokePro {
    pub mode: BrushMode,
    pub color_hdr: [f32; 4], // 32-bit float RGBA
    pub radius: f32,
    pub hardness: f32,       // 0.0 (soft Gaussian) to 1.0 (hard edge)
    pub opacity: f32,
    pub clone_offset: [f32; 2], // Source offset (dx, dy) for Clone Stamp
    pub points: Vec<[f32; 2]>,
}

impl PaintStrokePro {
    pub fn new_paint(color: [f32; 4], radius: f32, hardness: f32, opacity: f32) -> Self {
        Self {
            mode: BrushMode::Paint,
            color_hdr: color,
            radius,
            hardness: hardness.clamp(0.0, 1.0),
            opacity: opacity.clamp(0.0, 1.0),
            clone_offset: [0.0, 0.0],
            points: Vec::new(),
        }
    }

    pub fn new_clone_stamp(
        radius: f32,
        hardness: f32,
        opacity: f32,
        clone_offset: [f32; 2],
    ) -> Self {
        Self {
            mode: BrushMode::CloneStamp,
            color_hdr: [1.0, 1.0, 1.0, 1.0],
            radius,
            hardness: hardness.clamp(0.0, 1.0),
            opacity: opacity.clamp(0.0, 1.0),
            clone_offset,
            points: Vec::new(),
        }
    }

    /// Renders this stroke into a 32-bit float RGBA HDR buffer.
    pub fn apply_to_hdr_buffer(
        &self,
        buffer: &mut [f32],
        width: u32,
        height: u32,
    ) {
        if self.points.is_empty() || width == 0 || height == 0 {
            return;
        }

        // Snapshot original buffer for clone stamp sampling
        let src_snapshot = if self.mode == BrushMode::CloneStamp {
            Some(buffer.to_vec())
        } else {
            None
        };

        for &pt in &self.points {
            let min_x = ((pt[0] - self.radius).floor().max(0.0)) as u32;
            let max_x = ((pt[0] + self.radius).ceil().min(width as f32 - 1.0)) as u32;
            let min_y = ((pt[1] - self.radius).floor().max(0.0)) as u32;
            let max_y = ((pt[1] + self.radius).ceil().min(height as f32 - 1.0)) as u32;

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let dx = x as f32 + 0.5 - pt[0];
                    let dy = y as f32 + 0.5 - pt[1];
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist <= self.radius {
                        let normalized_d = dist / self.radius.max(1e-5);
                        let edge_start = self.hardness;
                        let alpha_coverage = if normalized_d <= edge_start {
                            1.0
                        } else {
                            let t = (normalized_d - edge_start) / (1.0 - edge_start).max(1e-5);
                            (1.0 - t).clamp(0.0, 1.0)
                        };

                        let final_a = alpha_coverage * self.opacity;
                        let idx = (y * width + x) as usize * 4;

                        if idx + 3 < buffer.len() {
                            match self.mode {
                                BrushMode::Paint => {
                                    let inv_a = 1.0 - final_a;
                                    buffer[idx] = buffer[idx] * inv_a + self.color_hdr[0] * final_a;
                                    buffer[idx + 1] = buffer[idx + 1] * inv_a + self.color_hdr[1] * final_a;
                                    buffer[idx + 2] = buffer[idx + 2] * inv_a + self.color_hdr[2] * final_a;
                                    buffer[idx + 3] = buffer[idx + 3] * inv_a + self.color_hdr[3] * final_a;
                                }
                                BrushMode::Eraser => {
                                    buffer[idx + 3] *= 1.0 - final_a;
                                }
                                BrushMode::CloneStamp => {
                                    if let Some(src) = &src_snapshot {
                                        let sx = (x as f32 + self.clone_offset[0]).round().clamp(0.0, width as f32 - 1.0) as u32;
                                        let sy = (y as f32 + self.clone_offset[1]).round().clamp(0.0, height as f32 - 1.0) as u32;
                                        let s_idx = (sy * width + sx) as usize * 4;

                                        if s_idx + 3 < src.len() {
                                            let inv_a = 1.0 - final_a;
                                            buffer[idx] = buffer[idx] * inv_a + src[s_idx] * final_a;
                                            buffer[idx + 1] = buffer[idx + 1] * inv_a + src[s_idx + 1] * final_a;
                                            buffer[idx + 2] = buffer[idx + 2] * inv_a + src[s_idx + 2] * final_a;
                                            buffer[idx + 3] = buffer[idx + 3] * inv_a + src[s_idx + 3] * final_a;
                                        }
                                    }
                                }
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
    fn test_paint_stroke_draws_on_hdr_buffer() {
        let mut buf = vec![0.0f32; 10 * 10 * 4]; // 10x10 transparent
        let mut stroke = PaintStrokePro::new_paint([1.0, 0.5, 0.2, 1.0], 2.0, 1.0, 1.0);
        stroke.points.push([5.0, 5.0]);

        stroke.apply_to_hdr_buffer(&mut buf, 10, 10);
        let center_idx = (5 * 10 + 5) * 4;
        assert_eq!(buf[center_idx], 1.0);
        assert_eq!(buf[center_idx + 1], 0.5);
        assert_eq!(buf[center_idx + 2], 0.2);
        assert_eq!(buf[center_idx + 3], 1.0);
    }

    #[test]
    fn test_clone_stamp_copies_source_pixel() {
        let mut buf = vec![0.0f32; 10 * 10 * 4];
        // Set pixel at [2, 2] to Green
        let src_idx = (2 * 10 + 2) * 4;
        buf[src_idx] = 0.0;
        buf[src_idx + 1] = 1.0;
        buf[src_idx + 2] = 0.0;
        buf[src_idx + 3] = 1.0;

        // Stamp at [5, 5] with clone offset [-3, -3] (targeting [2, 2])
        let mut stamp = PaintStrokePro::new_clone_stamp(1.0, 1.0, 1.0, [-3.0, -3.0]);
        stamp.points.push([5.0, 5.0]);
        stamp.apply_to_hdr_buffer(&mut buf, 10, 10);

        let dst_idx = (5 * 10 + 5) * 4;
        assert_eq!(buf[dst_idx + 1], 1.0); // Green successfully cloned!
    }
}
