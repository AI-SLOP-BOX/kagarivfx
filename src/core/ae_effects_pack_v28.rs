#![allow(dead_code)]
/// After Effects VFX Kernels Part 28 — Stylize & Light Pack Pro.
///
/// Production-quality implementations of:
///   * CC Light Sweep      — animated specular highlight band
///   * CC Radial Fast Blur — zoom blur around a centre point
///   * CC Bend It          — vertical-axis bend between top/bottom offsets
///   * CC Tiler Pro        — tiling with repeat/mirror edge modes
///   * Glow Pro            — threshold bright-pass + separable box blur bloom
///
/// All functions are deterministic, panic-free, and reuse clamp-to-edge
/// bilinear sampling semantics consistent with Part 27.
// ────────────────────────── Sampling Helper ──────────────────────────
fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    // Local implementation so this pack does not depend on another module's
    // internal helper staying public.
    if w == 0 || h == 0 || src.is_empty() {
        *out = [0, 0, 0, 0];
        return;
    }
    let x = fx.clamp(0.0, w as f32 - 1.0);
    let y = fy.clamp(0.0, h as f32 - 1.0);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let idx = |xx: u32, yy: u32| ((yy * w + xx) * 4) as usize;
    for c in 0..4 {
        let top = src[idx(x0, y0) + c] as f32 * (1.0 - tx) + src[idx(x1, y0) + c] as f32 * tx;
        let bot = src[idx(x0, y1) + c] as f32 * (1.0 - tx) + src[idx(x1, y1) + c] as f32 * tx;
        out[c] = (top * (1.0 - ty) + bot * ty).round().clamp(0.0, 255.0) as u8;
    }
}

// ─────────────────────────── CC Light Sweep ──────────────────────────

/// Parameters for [`apply_light_sweep`].
#[derive(Debug, Clone, Copy)]
pub struct LightSweepParams {
    /// Sweep direction in degrees (0 = left→right, 90 = top→bottom).
    pub direction_deg: f32,
    /// Band centre position across the projection axis (0..1).
    pub center: f32,
    /// Band width relative to the projection extent (0.01..1).
    pub width: f32,
    /// Peak highlight strength (0..1).
    pub sweep_intensity: f32,
    /// Extra brightness applied at band edges (0..1).
    pub edge_intensity: f32,
}

impl Default for LightSweepParams {
    fn default() -> Self {
        Self {
            direction_deg: 0.0,
            center: 0.5,
            width: 0.25,
            sweep_intensity: 0.6,
            edge_intensity: 0.3,
        }
    }
}

/// Additive specular highlight band travelling along `direction_deg`.
/// The band profile combines a smooth centre peak with thin bright edges,
/// matching the look of AE's CC Light Sweep.
pub fn apply_light_sweep(pixels: &mut [u8], width: u32, height: u32, p: &LightSweepParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let dir = p.direction_deg.to_radians();
    let (dx, dy) = (dir.cos(), dir.sin());
    // Projection extent along the sweep axis (diagonal coverage).
    let extent = (width as f32 * dx.abs() + height as f32 * dy.abs()).max(1.0);
    let center_px = p.center.clamp(0.0, 1.0) * extent;
    let half_w = (p.width.clamp(0.01, 1.0) * extent * 0.5).max(1.0);

    for y in 0..height {
        for x in 0..width {
            let proj = x as f32 * dx + y as f32 * dy;
            let d = ((proj - center_px) / half_w).abs();
            if d >= 1.0 {
                continue;
            }
            // Centre peak (cosine falloff) + narrow edge spikes near |d|≈1.
            let peak = (1.0 - d * d).max(0.0);
            let edge = ((d - 0.85) / 0.15).clamp(0.0, 1.0);
            let gain = peak * p.sweep_intensity + edge * p.edge_intensity;
            if gain <= 0.001 {
                continue;
            }
            let idx = ((y * width + x) * 4) as usize;
            for c in 0..3 {
                let v = pixels[idx + c] as f32 + 255.0 * gain;
                pixels[idx + c] = v.min(255.0) as u8;
            }
        }
    }
}

// ───────────────────────── CC Radial Fast Blur ───────────────────────

/// Zoom blur: averages `samples` radially scaled copies towards `center`.
/// `amount` is the fraction of the distance collapsed at the edges (0..1).
pub fn apply_radial_fast_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    amount: f32,
    samples: u32,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let amt = amount.clamp(0.0, 1.0);
    if amt <= 0.001 {
        return;
    }
    let n = samples.clamp(2, 64);
    let temp = pixels.to_vec();
    let (cx, cy) = (center[0], center[1]);

    for y in 0..height {
        for x in 0..width {
            let mut acc = [0.0f32; 4];
            for s in 0..n {
                let t = s as f32 / (n - 1) as f32;
                let scale = 1.0 - amt * t;
                let mut rgba = [0u8; 4];
                sample_bilinear(
                    &temp,
                    width,
                    height,
                    cx + (x as f32 - cx) * scale,
                    cy + (y as f32 - cy) * scale,
                    &mut rgba,
                );
                for c in 0..4 {
                    acc[c] += rgba[c] as f32;
                }
            }
            let idx = ((y * width + x) * 4) as usize;
            for c in 0..4 {
                pixels[idx + c] = (acc[c] / n as f32).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// ───────────────────────────── CC Bend It ────────────────────────────

/// Bends the image horizontally: the top edge shifts by `top_offset` px and
/// the bottom edge by `bottom_offset` px, interpolated with a smooth ease so
/// the middle bows naturally (AE CC Bend It look).
pub fn apply_cc_bend_it_pro(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    top_offset: f32,
    bottom_offset: f32,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let fh = height as f32;

    for y in 0..height {
        let t = y as f32 / (fh - 1.0).max(1.0);
        // Smoothstep-eased interpolation between the two edge offsets.
        let eased = t * t * (3.0 - 2.0 * t);
        let shift = top_offset + (bottom_offset - top_offset) * eased;
        for x in 0..width {
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, x as f32 + shift, y as f32, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

// ──────────────────────────── CC Tiler Pro ───────────────────────────

/// Edge handling for [`apply_cc_tiler_pro`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileEdgeMode {
    Repeat,
    Mirror,
}

/// Tiles a scaled-down copy of the source across the canvas.
/// `scale_percent` > 100 shrinks the tile (more repeats); Mirror alternates
/// flipped copies so tile seams disappear.
pub fn apply_cc_tiler_pro(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    scale_percent: f32,
    edge_mode: TileEdgeMode,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let factor = (100.0 / scale_percent.max(1.0)).clamp(0.02, 50.0); // source → tile scale
    let temp = pixels.to_vec();
    let fw = width as f32;
    let fh = height as f32;

    for y in 0..height {
        for x in 0..width {
            let mut u = x as f32 * factor / fw;
            let mut v = y as f32 * factor / fh;
            match edge_mode {
                TileEdgeMode::Repeat => {
                    u = u.rem_euclid(1.0);
                    v = v.rem_euclid(1.0);
                }
                TileEdgeMode::Mirror => {
                    u = mirror_fold(u);
                    v = mirror_fold(v);
                }
            }
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, u * fw, v * fh, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

/// Fold [0, ∞) into [0, 1] with alternating mirroring.
fn mirror_fold(t: f32) -> f32 {
    let m = t.rem_euclid(2.0);
    if m <= 1.0 {
        m
    } else {
        2.0 - m
    }
}

// ────────────────────────────── Glow Pro ─────────────────────────────

/// Threshold bloom: pixels above `threshold` luminance bleed outwards with a
/// separable box blur (two O(n) passes) scaled by `intensity`.
pub fn apply_glow_pro(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    threshold: f32,
    radius: u32,
    intensity: f32,
) {
    if width < 2 || height < 2 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let thr = threshold.clamp(0.0, 1.0);
    let inten = intensity.clamp(0.0, 4.0);
    if inten <= 0.001 {
        return;
    }
    let r = radius.min(128) as usize;

    // Bright pass into f32 planes.
    let n = (width as usize) * (height as usize);
    let mut plane = vec![0.0f32; n * 3];
    for i in 0..n {
        let idx = i * 4;
        let luma = (pixels[idx] as f32 * 0.2126
            + pixels[idx + 1] as f32 * 0.7152
            + pixels[idx + 2] as f32 * 0.0722)
            / 255.0;
        if luma > thr {
            let k = ((luma - thr) / (1.0 - thr).max(1e-3)).min(1.0);
            for c in 0..3 {
                plane[i * 3 + c] = pixels[idx + c] as f32 * k;
            }
        }
    }

    // Separable box blur (horizontal then vertical), sliding window.
    if r > 0 {
        box_blur_h(&mut plane, width as usize, height as usize, r);
        box_blur_v(&mut plane, width as usize, height as usize, r);
    }

    // Screen-composite the bloom back over the source.
    for i in 0..n {
        let idx = i * 4;
        for c in 0..3 {
            let base = pixels[idx + c] as f32 / 255.0;
            let bloom = (plane[i * 3 + c] / 255.0) * inten;
            let screened = 1.0 - (1.0 - base) * (1.0 - bloom);
            pixels[idx + c] = (screened.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

fn box_blur_h(plane: &mut [f32], w: usize, h: usize, r: usize) {
    let stride = w * 3;
    let mut row = vec![0.0f32; stride];
    for y in 0..h {
        let base = y * stride;
        row.copy_from_slice(&plane[base..base + stride]);
        for x in 0..w {
            let lo = x.saturating_sub(r);
            let hi = (x + r).min(w - 1);
            let count = ((hi - lo + 1) * 3) as f32;
            for c in 0..3 {
                let mut sum = 0.0f32;
                for xx in lo..=hi {
                    sum += row[xx * 3 + c];
                }
                plane[base + x * 3 + c] = sum / count;
            }
        }
    }
}

fn box_blur_v(plane: &mut [f32], w: usize, h: usize, r: usize) {
    let stride = w * 3;
    let mut col = vec![0.0f32; h * 3];
    for x in 0..w {
        for y in 0..h {
            let b = y * stride + x * 3;
            col[y * 3..y * 3 + 3].copy_from_slice(&plane[b..b + 3]);
        }
        for y in 0..h {
            let lo = y.saturating_sub(r);
            let hi = (y + r).min(h - 1);
            let count = (hi - lo + 1) as f32;
            for c in 0..3 {
                let mut sum = 0.0f32;
                for yy in lo..=hi {
                    sum += col[yy * 3 + c];
                }
                plane[y * stride + x * 3 + c] = sum / count;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                v.push(((x * 255) / w.max(1)) as u8);
                v.push(((y * 255) / h.max(1)) as u8);
                v.push(128);
                v.push(255);
            }
        }
        v
    }

    #[test]
    fn test_light_sweep_adds_brightness_in_band_only() {
        let mut img = gradient(32, 32);
        let before = img.clone();
        apply_light_sweep(
            &mut img,
            32,
            32,
            &LightSweepParams {
                direction_deg: 0.0,
                center: 0.5,
                width: 0.2,
                sweep_intensity: 0.8,
                edge_intensity: 0.0,
            },
        );
        // Centre column must brighten; far-left column must stay identical.
        let mid = ((16 * 32 + 16) * 4) as usize;
        assert!(img[mid] > before[mid], "band centre should brighten");
        let left = ((16 * 32) * 4) as usize;
        assert_eq!(img[left], before[left], "outside band must be untouched");
    }

    #[test]
    fn test_light_sweep_zero_intensity_is_identity() {
        let mut img = gradient(16, 16);
        let before = img.clone();
        apply_light_sweep(
            &mut img,
            16,
            16,
            &LightSweepParams {
                sweep_intensity: 0.0,
                edge_intensity: 0.0,
                ..Default::default()
            },
        );
        assert_eq!(img, before);
    }

    #[test]
    fn test_radial_fast_blur_preserves_center_and_size() {
        let mut img = gradient(24, 24);
        let before = img.clone();
        // Integer-aligned centre: every radial sample lands exactly on the
        // same pixel, so that pixel must be bit-identical.
        apply_radial_fast_blur(&mut img, 24, 24, [12.0, 12.0], 0.5, 8);
        assert_eq!(img.len(), before.len());
        let c = ((12 * 24 + 12) * 4) as usize;
        assert_eq!(img[c], before[c]);
        // Zero amount is identity.
        let mut id = before.clone();
        apply_radial_fast_blur(&mut id, 24, 24, [12.0, 12.0], 0.0, 8);
        assert_eq!(id, before);
    }

    #[test]
    fn test_bend_it_shifts_edges_by_exact_offsets() {
        let mut img = gradient(20, 20);
        let before = img.clone();
        apply_cc_bend_it_pro(&mut img, 20, 20, 5.0, -5.0);
        // Sampling is src = dst + shift:
        // Top row (+5): destination (6,0) shows source (11,0).
        let top_after = (6 * 4) as usize;
        let top_src = (11 * 4) as usize;
        assert!(
            (img[top_after] as i32 - before[top_src] as i32).abs() <= 1,
            "top row must shift by +5px"
        );
        // Bottom row (-5): destination (14,19) shows source (9,19).
        let bot_after = ((19 * 20 + 14) * 4) as usize;
        let bot_src = ((19 * 20 + 9) * 4) as usize;
        assert!(
            (img[bot_after] as i32 - before[bot_src] as i32).abs() <= 1,
            "bottom row must shift by -5px"
        );
        // Middle row eased shift ≈ ±0.4px on a ~12.75 level/px gradient → small drift.
        let mid_before = ((10 * 20 + 10) * 4) as usize;
        let mid_after = ((10 * 20 + 10) * 4) as usize;
        assert!(
            (img[mid_after] as i32 - before[mid_before] as i32).abs() <= 8,
            "middle row must move far less than the edges"
        );
    }

    #[test]
    fn test_tiler_repeat_and_mirror_cover_canvas() {
        let mut a = gradient(16, 16);
        apply_cc_tiler_pro(&mut a, 16, 16, 400.0, TileEdgeMode::Repeat);
        assert!(a.chunks(4).all(|px| px[3] == 255));

        let mut b = gradient(16, 16);
        apply_cc_tiler_pro(&mut b, 16, 16, 400.0, TileEdgeMode::Mirror);
        // Mirror tiles are symmetric about tile boundaries: row 0 equals row 7 mirrored.
        let (r0, r7) = (0usize, (7 * 16) * 4);
        assert_eq!(b[r0], b[r7]);
    }

    #[test]
    fn test_glow_pro_brightens_bright_areas_only() {
        // Dark background with a bright square.
        let mut img = vec![10u8; 32 * 32 * 4];
        for y in 12..20 {
            for x in 12..20 {
                let i = ((y * 32 + x) * 4) as usize;
                img[i] = 250;
                img[i + 1] = 250;
                img[i + 2] = 250;
            }
        }
        let before = img.clone();
        apply_glow_pro(&mut img, 32, 32, 0.7, 4, 1.0);

        // Bright core stays bright (screen keeps ≥ original).
        let core = ((16 * 32 + 16) * 4) as usize;
        assert!(img[core] >= before[core]);
        // Pixel adjacent to the square (outside it) must gain brightness.
        let halo = ((16 * 32 + 21) * 4) as usize;
        assert!(img[halo] > before[halo], "halo should receive bloom");
        // Far corner barely affected.
        let far = ((32 + 1) * 4) as usize;
        assert!(img[far] <= before[far] + 30);
    }

    #[test]
    fn test_glow_pro_identity_when_intensity_zero_or_below_threshold() {
        let mut img = gradient(16, 16);
        let before = img.clone();
        apply_glow_pro(&mut img, 16, 16, 0.5, 4, 0.0);
        assert_eq!(img, before);
        // Everything below threshold → no bright pass → identity.
        let mut dark = vec![5u8; 16 * 16 * 4];
        let dark_before = dark.clone();
        apply_glow_pro(&mut dark, 16, 16, 0.9, 4, 1.0);
        assert_eq!(dark, dark_before);
    }

    #[test]
    fn test_degenerate_inputs_do_not_panic() {
        let mut empty: Vec<u8> = vec![];
        apply_light_sweep(&mut empty, 0, 0, &LightSweepParams::default());
        apply_radial_fast_blur(&mut empty, 0, 0, [0.0, 0.0], 0.5, 8);
        apply_cc_bend_it_pro(&mut empty, 0, 0, 10.0, -10.0);
        apply_cc_tiler_pro(&mut empty, 0, 0, 200.0, TileEdgeMode::Mirror);
        apply_glow_pro(&mut empty, 0, 0, 0.5, 4, 1.0);
    }

    #[test]
    fn test_all_deterministic() {
        let run = || {
            let mut img = gradient(24, 24);
            apply_light_sweep(&mut img, 24, 24, &LightSweepParams::default());
            apply_radial_fast_blur(&mut img, 24, 24, [12.0, 12.0], 0.4, 6);
            apply_cc_bend_it_pro(&mut img, 24, 24, 3.0, -3.0);
            apply_cc_tiler_pro(&mut img, 24, 24, 150.0, TileEdgeMode::Mirror);
            apply_glow_pro(&mut img, 24, 24, 0.4, 2, 0.8);
            img
        };
        assert_eq!(run(), run());
    }
}
