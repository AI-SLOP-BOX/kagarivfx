//! Paint stroke rasterization: stamps alpha-over round caps along a
//! polyline into an RGBA layer buffer (straight alpha, premultiplied-safe
//! blending done manually).

use serde::{Deserialize, Serialize};

/// A single paint stroke with pressure-sensitive width and hardness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaintStroke {
    pub id: u64,
    pub color: [f32; 4],
    pub size: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub points: Vec<StrokePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

/// A paint layer holding all brush strokes for a frame.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaintLayer {
    pub strokes: Vec<PaintStroke>,
}

impl PaintLayer {
    pub fn new() -> Self {
        Self { strokes: Vec::new() }
    }

    pub fn add_stroke(&mut self, stroke: PaintStroke) {
        self.strokes.push(stroke);
    }

    pub fn clear(&mut self) {
        self.strokes.clear();
    }

    /// Render all paint strokes into an RGBA buffer.
    pub fn render(&self, pixels: &mut [u8], width: u32, height: u32) {
        for stroke in &self.strokes {
            let points: Vec<[f32; 2]> = stroke.points.iter().map(|p| [p.x, p.y]).collect();
            draw_stroke(pixels, width, height, &points, stroke.color, stroke.size);
        }
    }
}

/// Draw one stroke into `buf` (w×h RGBA8). Points are in buffer pixel space.
pub fn draw_stroke(
    buf: &mut [u8],
    w: u32,
    h: u32,
    points: &[[f32; 2]],
    color: [f32; 4],
    size: f32,
) {
    if points.is_empty() || size <= 0.0 || w == 0 || h == 0 {
        return;
    }
    let radius = (size * 0.5).max(0.5);
    let r2 = radius * radius;
    let src_a = color[3].clamp(0.0, 1.0);

    let mut stamp = |cx: f32, cy: f32| {
        let lo_x = ((cx - radius).floor().max(0.0)) as u32;
        let hi_x = ((cx + radius).ceil().min(w as f32 - 1.0)) as u32;
        let lo_y = ((cy - radius).floor().max(0.0)) as u32;
        let hi_y = ((cy + radius).ceil().min(h as f32 - 1.0)) as u32;
        for py in lo_y..=hi_y {
            for px in lo_x..=hi_x {
                let dx = px as f32 + 0.5 - cx;
                let dy = py as f32 + 0.5 - cy;
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                // Hard core with a 1px anti-aliased rim.
                let dist = d2.sqrt();
                if dist > radius { continue; }
                let cov = (radius - dist).clamp(0.0, 1.0);
                let idx = ((py * w + px) * 4) as usize;
                if idx + 3 >= buf.len() {
                    continue;
                }
                let dst = [
                    buf[idx] as f32 / 255.0,
                    buf[idx + 1] as f32 / 255.0,
                    buf[idx + 2] as f32 / 255.0,
                    buf[idx + 3] as f32 / 255.0,
                ];
                let sa = src_a * cov;
                let out_a = sa + dst[3] * (1.0 - sa);
                if out_a <= 0.0001 {
                    continue;
                }
                for ch in 0..3 {
                    let sc = color[ch];
                    let v = (sc * sa + dst[ch] * dst[3] * (1.0 - sa)) / out_a;
                    buf[idx + ch] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
                buf[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    };

    if points.len() == 1 {
        stamp(points[0][0], points[0][1]);
        return;
    }
    for seg in points.windows(2) {
        let (a, b) = (&seg[0], &seg[1]);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt();
        let steps = ((len / (radius * 0.5)).ceil() as usize).clamp(1, 4096);
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            stamp(a[0] + dx * t, a[1] + dy * t);
        }
    }
}

/// Configuration for the clone stamp tool.
#[derive(Debug, Clone)]
pub struct CloneStampConfig {
    pub src_offset: [f32; 2],
    pub size: f32,
    pub opacity: f32,
    pub hardness: f32,
}

/// Clone stamp: copies pixels from a source buffer at an offset into `dst`.
pub fn draw_clone_stamp(
    dst: &mut [u8],
    src: &[u8],
    w: u32,
    h: u32,
    points: &[[f32; 2]],
    config: &CloneStampConfig,
) {
    if points.is_empty() || config.size <= 0.0 || w == 0 || h == 0 {
        return;
    }
    let radius = (config.size * 0.5).max(0.5);
    let r2 = radius * radius;
    let soft = (1.0 - config.hardness).max(0.0);

    let mut stamp = |cx: f32, cy: f32| {
        let sx_base = cx + config.src_offset[0];
        let sy_base = cy + config.src_offset[1];
        let lo_x = ((cx - radius).floor().max(0.0)) as u32;
        let hi_x = ((cx + radius).ceil().min(w as f32 - 1.0)) as u32;
        let lo_y = ((cy - radius).floor().max(0.0)) as u32;
        let hi_y = ((cy + radius).ceil().min(h as f32 - 1.0)) as u32;
        for py in lo_y..=hi_y {
            for px in lo_x..=hi_x {
                let dx = px as f32 + 0.5 - cx;
                let dy = py as f32 + 0.5 - cy;
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let dist = d2.sqrt();
                let alpha = if soft <= 0.001 {
                    if dist <= radius { 1.0 } else { 0.0 }
                } else {
                    let t = dist / radius;
                    if t < 1.0 - soft { 1.0 } else if t < 1.0 { 1.0 - (t - (1.0 - soft)) / soft } else { 0.0 }
                };
                let a = alpha * config.opacity;
                if a <= 0.001 {
                    continue;
                }
                let sx = (sx_base + dx) as i32;
                let sy = (sy_base + dy) as i32;
                if sx < 0 || sy < 0 || sx >= w as i32 || sy >= h as i32 {
                    continue;
                }
                let s_idx = ((sy as u32 * w + sx as u32) * 4) as usize;
                let d_idx = ((py * w + px) * 4) as usize;
                if s_idx + 3 >= src.len() || d_idx + 3 >= dst.len() {
                    continue;
                }
                let out_a = a + dst[d_idx + 3] as f32 / 255.0 * (1.0 - a);
                if out_a > 0.0001 {
                    let inv_a = 1.0 / out_a;
                    for ch in 0..3 {
                        let s_val = src[s_idx + ch] as f32 / 255.0;
                        let d_val = dst[d_idx + ch] as f32 / 255.0;
                        dst[d_idx + ch] = ((s_val * a + d_val * dst[d_idx + 3] as f32 / 255.0 * (1.0 - a)) * inv_a * 255.0).round().min(255.0) as u8;
                    }
                    dst[d_idx + 3] = (out_a * 255.0).round().min(255.0) as u8;
                }
            }
        }
    };

    if points.len() == 1 {
        stamp(points[0][0], points[0][1]);
        return;
    }
    for seg in points.windows(2) {
        let (a, b) = (&seg[0], &seg[1]);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt();
        let steps = ((len / (radius * 0.5)).ceil() as usize).clamp(1, 4096);
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            stamp(a[0] + dx * t, a[1] + dy * t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(w: u32, h: u32) -> Vec<u8> {
        vec![0u8; (w * h * 4) as usize]
    }

    #[test]
    fn test_single_dot_paints_center_opaque() {
        let mut buf = blank(9, 9);
        draw_stroke(&mut buf, 9, 9, &[[4.5, 4.5]], [1.0, 0.0, 0.0, 1.0], 4.0);
        let center = ((4 * 9 + 4) * 4) as usize;
        assert_eq!(buf[center], 255);
        assert_eq!(buf[center + 3], 255);
        // Corner untouched.
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn test_line_is_contiguous() {
        let w = 40u32;
        let mut buf = blank(w, 7);
        draw_stroke(&mut buf, w, 7, &[[2.0, 3.5], [37.0, 3.5]], [0.0, 1.0, 0.0, 1.0], 3.0);
        // Sample midpoints along the line — all should be green-ish opaque.
        for x in [5u32, 15, 25, 34] {
            let i = ((3 * w + x) * 4) as usize;
            assert!(buf[i + 1] > 200, "gap at x={}", x);
            assert_eq!(buf[i + 3], 255);
        }
    }

    #[test]
    fn test_alpha_over_blends_existing() {
        let w = 5u32;
        let mut buf = blank(w, 1);
        // Pre-fill with opaque blue.
        for px in buf.chunks_exact_mut(4) { px.copy_from_slice(&[0, 0, 255, 255]); }
        // Half-alpha red dot on top → purple-ish, still opaque.
        draw_stroke(&mut buf, w, 1, &[[2.5, 0.5]], [1.0, 0.0, 0.0, 0.5], 5.0);
        let i = (2 * 4) as usize;
        assert_eq!(buf[i + 3], 255, "stays opaque");
        assert!(buf[i] > 100 && buf[i + 2] > 100, "blend produces mixed color");
    }

    #[test]
    fn test_zero_size_noop() {
        let mut buf = blank(4, 4);
        draw_stroke(&mut buf, 4, 4, &[[2.0, 2.0]], [1.0, 1.0, 1.0, 1.0], 0.0);
        assert!(buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_paint_layer_add_and_clear() {
        let mut layer = PaintLayer::new();
        layer.add_stroke(PaintStroke {
            id: 1, color: [1.0; 4], size: 5.0, hardness: 0.5, opacity: 1.0,
            points: vec![StrokePoint { x: 0.0, y: 0.0, pressure: 1.0 }],
        });
        assert_eq!(layer.strokes.len(), 1);
        layer.clear();
        assert!(layer.strokes.is_empty());
    }

    #[test]
    fn test_paint_layer_render() {
        let mut layer = PaintLayer::new();
        layer.add_stroke(PaintStroke {
            id: 1, color: [1.0, 0.0, 0.0, 1.0], size: 10.0, hardness: 0.5, opacity: 1.0,
            points: vec![
                StrokePoint { x: 50.0, y: 50.0, pressure: 1.0 },
                StrokePoint { x: 60.0, y: 50.0, pressure: 1.0 },
            ],
        });
        let mut buf = blank(100, 100);
        layer.render(&mut buf, 100, 100);
        let center = ((50 * 100 + 50) * 4) as usize;
        assert!(buf[center + 3] > 0, "center pixel should be painted");
    }

    #[test]
    fn test_empty_stroke_noop() {
        let mut layer = PaintLayer::new();
        layer.add_stroke(PaintStroke {
            id: 1, color: [1.0; 4], size: 5.0, hardness: 0.5, opacity: 1.0,
            points: vec![],
        });
        let mut buf = blank(10, 10);
        layer.render(&mut buf, 10, 10);
        assert!(buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_clone_stamp_copies_source() {
        let w = 20u32;
        let h = 20u32;
        let mut dst = blank(w, h);
        let mut src = vec![0u8; (w * h * 4) as usize];
        // Draw red circle in source at (5,5)
        for py in 0..h {
            for px in 0..w {
                let dx = px as f32 - 5.0;
                let dy = py as f32 - 5.0;
                if dx * dx + dy * dy < 4.0 {
                    let i = ((py * w + px) * 4) as usize;
                    src[i] = 255;
                    src[i + 3] = 255;
                }
            }
        }
        // Clone from (0,0) offset → source (5,5) maps to dest (5,5)
        let config = CloneStampConfig {
            src_offset: [-5.0, -5.0], size: 4.0, opacity: 1.0, hardness: 1.0,
        };
        draw_clone_stamp(&mut dst, &src, w, h, &[[10.0, 10.0]], &config);
        let idx = ((10 * w + 10) * 4) as usize;
        assert!(dst[idx] > 0, "cloned pixel should have red");
        assert!(dst[idx + 3] > 0, "cloned pixel should have alpha");
    }

    #[test]
    fn test_clone_stamp_empty_points_noop() {
        let mut dst = blank(10, 10);
        let src = vec![128u8; 400];
        let config = CloneStampConfig {
            src_offset: [0.0; 2], size: 5.0, opacity: 1.0, hardness: 1.0,
        };
        draw_clone_stamp(&mut dst, &src, 10, 10, &[], &config);
        assert!(dst.iter().all(|&b| b == 0));
    }
}
