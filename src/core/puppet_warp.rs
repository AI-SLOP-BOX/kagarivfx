//! Puppet-tool mesh warp: inverse-distance-weighted displacement of an
//! isolated layer buffer, driven by pin rest/current position pairs
//! (both in layer-buffer pixel space).
//!
//! For every destination pixel the displacement is a Shepard interpolation
//! of the pin offsets with weights 1/(dist²+ε), then the source color is
//! sampled bilinearly from the pre-warp copy.

/// Pins as `(source_xy, current_xy)` pairs in buffer pixel coordinates.
pub fn warp_layer_buf(
    buf: &mut [u8],
    w: u32,
    h: u32,
    pins: &[([f32; 2], [f32; 2])],
) {
    if w == 0 || h == 0 || pins.is_empty() {
        return;
    }
    // Skip work entirely when no pin actually moved.
    let mut moved = false;
    for (s, d) in pins {
        if (d[0] - s[0]).abs() > 0.01 || (d[1] - s[1]).abs() > 0.01 {
            moved = true;
            break;
        }
    }
    if !moved {
        return;
    }

    let src = buf.to_vec();



    for py in 0..h {
        for px in 0..w {
            let p = [px as f32 + 0.5, py as f32 + 0.5];
            let mut off = [0.0f32; 2];
            let mut wsum = 0.0f32;
            for (s, d) in pins {
                let dxs = p[0] - s[0];
                let dys = p[1] - s[1];
                let r2 = dxs * dxs + dys * dys;
                let wgt = 1.0 / (r2 + 4.0);
                off[0] += (d[0] - s[0]) * wgt;
                off[1] += (d[1] - s[1]) * wgt;
                wsum += wgt;
            }
            if wsum <= f32::EPSILON {
                continue;
            }
            // Source coordinates may fall outside the buffer; bilinear
            // sampling treats out-of-bounds as transparent so shifted-away
            // regions empty out instead of keeping stale pixels.
            let sx = p[0] - off[0] / wsum - 0.5;
            let sy = p[1] - off[1] / wsum - 0.5;
            let x0 = sx.floor();
            let y0 = sy.floor();
            let fx = sx - x0;
            let fy = sy - y0;
            let x0i = x0 as i64;
            let y0i = y0 as i64;
            let sample = |xx: i64, yy: i64| -> [f32; 4] {
                if xx < 0 || yy < 0 || xx >= w as i64 || yy >= h as i64 {
                    return [0.0; 4];
                }
                let idx = ((yy as u32 * w + xx as u32) * 4) as usize;
                if idx + 3 >= src.len() {
                    return [0.0; 4];
                }
                [
                    src[idx] as f32,
                    src[idx + 1] as f32,
                    src[idx + 2] as f32,
                    src[idx + 3] as f32,
                ]
            };
            let c00 = sample(x0i, y0i);
            let c10 = sample(x0i + 1, y0i);
            let c01 = sample(x0i, y0i + 1);
            let c11 = sample(x0i + 1, y0i + 1);
            let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
            let idx = ((py * w + px) * 4) as usize;
            if idx + 3 >= buf.len() {
                continue;
            }
            for ch in 0..4 {
                let top = lerp(c00[ch], c10[ch], fx);
                let bot = lerp(c01[ch], c11[ch], fx);
                buf[idx + ch] = lerp(top, bot, fy).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for px in v.chunks_exact_mut(4) {
            px.copy_from_slice(&[200, 50, 25, 255]);
        }
        v
    }

    #[test]
    fn test_no_pins_or_unmoved_is_noop() {
        let mut a = solid(4, 4);
        let before = a.clone();
        warp_layer_buf(&mut a, 4, 4, &[]);
        assert_eq!(a, before);
        warp_layer_buf(&mut a, 4, 4, &[([2.0, 2.0], [2.0, 2.0])]);
        assert_eq!(a, before);
    }

    #[test]
    fn test_single_pin_shift_moves_pixels() {
        // 8x8 opaque; shift whole plane right by 2 via one central pin.
        let mut img = solid(8, 8);
        warp_layer_buf(&mut img, 8, 8, &[([4.0, 4.0], [6.0, 4.0])]);
        // Pixel at x=5 should now carry content that was around x≈3..4.
        // With IDW falloff exact values vary; assert left edge became transparent-ish
        // and right region stayed opaque.
        let alpha_at = |x: u32, y: u32| img[((y * 8 + x) * 4 + 3) as usize];
        assert!(alpha_at(7, 4) == 255);
        assert!(alpha_at(0, 4) < 255, "left column should lose opacity after shift");
    }

    #[test]
    fn test_warp_preserves_dimensions() {
        let mut img = solid(5, 3);
        warp_layer_buf(&mut img, 5, 3, &[([1.0, 1.0], [3.0, 2.0])]);
        assert_eq!(img.len(), (5 * 3 * 4) as usize);
    }
}
