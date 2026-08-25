//! Paint stroke rasterization: stamps alpha-over round caps along a
//! polyline into an RGBA layer buffer (straight alpha, premultiplied-safe
//! blending done manually).

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
}
