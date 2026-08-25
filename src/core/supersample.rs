//! 2× supersampling (SSAA) helper for exports: renders at double
//! resolution then box-filters down, giving clean anti-aliased edges from
//! the CPU rasterizer at the cost of 4× pixel work.

/// Box-downsample an RGBA buffer rendered at exactly `2w × 2h` into `w × h`.
/// Averages each 2×2 block with alpha weighting to avoid dark fringes on
/// transparent edges.
pub fn downsample2x(src: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (hw, hh) = (w as usize, h as usize);
    let sw = hw * 2;
    let mut out = vec![0u8; hw * hh * 4];
    for y in 0..hh {
        for x in 0..hw {
            let mut acc = [0.0f32; 4];
            let mut wsum = 0.0f32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let sx = x * 2 + dx;
                    let sy = y * 2 + dy;
                    let idx = (sy * sw + sx) * 4;
                    if idx + 3 >= src.len() { continue; }
                    let a = src[idx + 3] as f32 / 255.0;
                    acc[0] += src[idx] as f32 * a;
                    acc[1] += src[idx + 1] as f32 * a;
                    acc[2] += src[idx + 2] as f32 * a;
                    acc[3] += src[idx + 3] as f32;
                    wsum += a.max(f32::EPSILON);
                }
            }
            // Alpha-weighted color average; straight alpha out.
            let o = (y * hw + x) * 4;
            if wsum > 0.0 {
                out[o] = (acc[0] / wsum).round().clamp(0.0, 255.0) as u8;
                out[o + 1] = (acc[1] / wsum).round().clamp(0.0, 255.0) as u8;
                out[o + 2] = (acc[2] / wsum).round().clamp(0.0, 255.0) as u8;
            }
            out[o + 3] = (acc[3] / 4.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downsample_uniform_stays_uniform() {
        // 4x4 all red opaque → 2x2 all red opaque.
        let mut src = vec![0u8; 16 * 4];
        for px in src.chunks_exact_mut(4) { px.copy_from_slice(&[255, 0, 0, 255]); }
        let out = downsample2x(&src, 2, 2);
        assert_eq!(out.len(), 2 * 2 * 4);
        for px in out.chunks_exact(4) {
            assert_eq!(px, &[255, 0, 0, 255]);
        }
    }

    #[test]
    fn test_downsample_averages_checkerboard() {
        // 2x2 checker: white/black opaque → gray.
        let src = vec![255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255];
        let out = downsample2x(&src, 1, 1);
        assert_eq!(out[0], 128); // (255+0+0+255)/4 rounded
        assert_eq!(out[3], 255);
    }

    #[test]
    fn test_alpha_weighting_no_dark_fringe() {
        // Left column opaque red, right column fully transparent black.
        let src = vec![
            255, 0, 0, 255, 255, 0, 0, 255,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let out = downsample2x(&src, 1, 1);
        // Color must stay pure red (not blended toward black), alpha ~50%.
        assert_eq!(out[0], 255);
        assert_eq!(out[1], 0);
        assert_eq!(out[3], 128); // (255+255+0+0)/4
    }
}
