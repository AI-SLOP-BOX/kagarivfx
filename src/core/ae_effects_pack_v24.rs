//! Cinematic letterbox bars (2.39:1 scope look) — draws black bars
//! top/bottom covering a fraction of the frame height.

/// Draw letterbox bars over an RGBA buffer. `frac` = total bar coverage
/// relative to height (0.12 ≈ 2.39:1 on a 16:9 frame).
pub fn apply_letterbox(pixels: &mut [u8], width: u32, height: u32, frac: f32) {
    let frac = frac.clamp(0.0, 0.45);
    if frac <= 0.001 || width == 0 || height == 0 {
        return;
    }
    let bar_h = ((height as f32 * frac) / 2.0).floor() as u32;
    if bar_h == 0 {
        return;
    }
    let row_bytes = (width * 4) as usize;
    for row in 0..bar_h as usize {
        let top = row * row_bytes;
        let bot = (height as usize - 1 - row) * row_bytes;
        for off in (0..row_bytes).step_by(4) {
            if top + off + 3 < pixels.len() {
                pixels[top + off] = 0;
                pixels[top + off + 1] = 0;
                pixels[top + off + 2] = 0;
                pixels[top + off + 3] = 255;
            }
            if bot + off + 3 < pixels.len() {
                pixels[bot + off] = 0;
                pixels[bot + off + 1] = 0;
                pixels[bot + off + 2] = 0;
                pixels[bot + off + 3] = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bars_cover_fraction() {
        let w = 4u32;
        let h = 10u32; // frac 0.2 → bar_h = 1 per side
        let mut px = vec![255u8; (w * h * 4) as usize];
        apply_letterbox(&mut px, w, h, 0.2);
        // Row 0 and row 9 black.
        assert_eq!(&px[0..4], &[0, 0, 0, 255]);
        assert_eq!(&px[((9 * w as usize) * 4)..][..4], &[0, 0, 0, 255]);
        // Middle untouched.
        assert_eq!(px[(5 * 4) as usize], 255);
    }

    #[test]
    fn test_zero_frac_noop() {
        let mut px = vec![200u8; 16];
        apply_letterbox(&mut px, 2, 2, 0.0);
        assert!(px.iter().all(|&b| b == 200));
    }
}
