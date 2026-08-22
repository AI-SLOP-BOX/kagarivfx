#![allow(dead_code)]
/// After Effects VFX Kernels Part 27 — Distortion Pack Pro.
///
/// High-fidelity rebuilds of Wave Warp, CC Lens, Polar Coordinates and Optics
/// Compensation. Unlike the simplified kernels in earlier packs, these versions
/// feature:
///   * Sub-pixel accurate bilinear sampling (no blocky nearest-neighbour artifacts)
///   * Multiple wave shapes (Sine / Triangle / Square / Sawtooth)
///   * Edge pinning modes matching AE (All / Left-Right / Top-Bottom / None)
///   * Interpolated Rect <-> Polar conversion (AE "Interpolation" parameter)
///
/// All functions are deterministic, panic-free and allocation-light (single
/// source snapshot per invocation).

use std::f32::consts::{PI, TAU};

// ────────────────────────── Sampling Helper ──────────────────────────

/// Clamp-to-edge bilinear RGBA sample from a packed u8 buffer.
fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    if src.len() < (w as usize) * (h as usize) * 4 || w == 0 || h == 0 {
        *out = [0, 0, 0, 0];
        return;
    }
    let x = fx.clamp(0.0, w as f32 - 1.0);
    let y = fy.clamp(0.0, h as f32 - 1.0);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(w as usize - 1);
    let y1 = (y0 + 1).min(h as usize - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let idx = |xx: usize, yy: usize| (yy * w as usize + xx) * 4;

    for c in 0..4 {
        let p00 = src[idx(x0, y0) + c] as f32;
        let p10 = src[idx(x1, y0) + c] as f32;
        let p01 = src[idx(x0, y1) + c] as f32;
        let p11 = src[idx(x1, y1) + c] as f32;
        let top = p00 + (p10 - p00) * tx;
        let bot = p01 + (p11 - p01) * tx;
        out[c] = (top + (bot - top) * ty).round().clamp(0.0, 255.0) as u8;
    }
}

// ──────────────────────────── Wave Warp ──────────────────────────────

/// Wave shape family for Wave Warp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveType {
    Sine,
    Triangle,
    Square,
    Sawtooth,
}

impl WaveType {
    /// Periodic waveform in [-1, 1] for a phase in radians.
    pub fn waveform(self, phase: f32) -> f32 {
        match self {
            WaveType::Sine => phase.sin(),
            WaveType::Triangle => {
                let f = (phase / TAU).rem_euclid(1.0);
                1.0 - 4.0 * (f - 0.5).abs()
            }
            WaveType::Square => {
                if phase.rem_euclid(TAU) < PI { 1.0 } else { -1.0 }
            }
            WaveType::Sawtooth => {
                let f = (phase / TAU).rem_euclid(1.0);
                2.0 * f - 1.0
            }
        }
    }
}

/// Which edges are pinned (displacement attenuated to zero), matching AE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKind {
    All,
    LeftRight,
    TopBottom,
    None,
}

/// Parameters for [`apply_wave_warp_pro`].
#[derive(Debug, Clone, Copy)]
pub struct WaveWarpParams {
    /// Peak displacement in pixels.
    pub wave_height: f32,
    /// Spatial wavelength in pixels.
    pub wave_width: f32,
    /// Phase advance per second (wave travel speed).
    pub speed: f32,
    /// Absolute time in seconds (animation driver).
    pub time: f32,
    /// Static phase offset in radians.
    pub phase: f32,
    /// Displacement direction in degrees (90 = vertical, AE default).
    pub direction_deg: f32,
    pub wave_type: WaveType,
    pub pinning: PinKind,
}

impl Default for WaveWarpParams {
    fn default() -> Self {
        Self {
            wave_height: 50.0,
            wave_width: 100.0,
            speed: 1.0,
            time: 0.0,
            phase: 0.0,
            direction_deg: 90.0,
            wave_type: WaveType::Sine,
            pinning: PinKind::All,
        }
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Edge attenuation factor (0 at pinned edges, 1 in the interior).
fn pin_attenuation(kind: PinKind, x: u32, y: u32, w: u32, h: u32) -> f32 {
    let fw = w as f32;
    let fh = h as f32;
    let ramp = |pos: f32, size: f32| -> f32 {
        let e = (size * 0.12).clamp(2.0, 64.0);
        smoothstep(pos / e).min(smoothstep((size - 1.0 - pos) / e))
    };
    match kind {
        PinKind::None => 1.0,
        PinKind::LeftRight => ramp(x as f32, fw),
        PinKind::TopBottom => ramp(y as f32, fh),
        PinKind::All => ramp(x as f32, fw) * ramp(y as f32, fh),
    }
}

/// Production-quality Wave Warp with selectable wave shape, direction and
/// edge pinning. Displacement happens along `direction_deg`; the wave phase
/// varies along the perpendicular axis.
pub fn apply_wave_warp_pro(pixels: &mut [u8], width: u32, height: u32, p: &WaveWarpParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let dir = p.direction_deg.to_radians();
    let (dx, dy) = (dir.cos(), dir.sin());
    // Sampling axis is perpendicular to the displacement direction.
    let (px, py) = (-dy, dx);
    let ww = p.wave_width.max(1.0);

    for y in 0..height {
        for x in 0..width {
            let u = (x as f32 * px + y as f32 * py) / ww;
            let phase = u * TAU + p.phase + p.speed * p.time * TAU;
            let disp = p.wave_height * p.wave_type.waveform(phase);
            let att = pin_attenuation(p.pinning, x, y, width, height);
            let d = disp * att;
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, x as f32 + dx * d, y as f32 + dy * d, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

// ───────────────────────────── CC Lens ───────────────────────────────

/// Parameters for [`apply_cc_lens_pro`].
#[derive(Debug, Clone, Copy)]
pub struct CcLensParams {
    /// -100 (fisheye out) .. 100 (bulge in), AE-style.
    pub convergence: f32,
    /// Output zoom factor (1.0 = neutral).
    pub zoom: f32,
}

impl Default for CcLensParams {
    fn default() -> Self {
        Self { convergence: 50.0, zoom: 1.0 }
    }
}

/// True radial fisheye: source radius follows `rn^(1 + k)` so the centre is
/// magnified for positive convergence and compressed for negative. Uses
/// bilinear sampling; the exact centre pixel is invariant.
pub fn apply_cc_lens_pro(pixels: &mut [u8], width: u32, height: u32, p: &CcLensParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let cx = (width - 1) as f32 * 0.5;
    let cy = (height - 1) as f32 * 0.5;
    let rmax = (cx * cx + cy * cy).sqrt().max(1.0);
    let k = (p.convergence / 100.0).clamp(-0.95, 0.95);
    let zoom = p.zoom.max(0.01);

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let rn = ((rx * rx + ry * ry).sqrt() / rmax * zoom).clamp(0.0, 1.0);
            let src_rn = rn.powf(1.0 + k);
            let scale = if rn > 1e-6 { src_rn / rn } else { 1.0 };
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, cx + rx * scale, cy + ry * scale, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

// ───────────────────────── Polar Coordinates ─────────────────────────

/// Conversion direction for [`apply_polar_coordinates_pro`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolarMode {
    RectToPolar,
    PolarToRect,
}

/// AE-style Polar Coordinates with an interpolation blend between the
/// original image and the fully converted result.
///
/// * `RectToPolar`: the vertical axis becomes the radius, the horizontal
///   axis sweeps a full circle around the image centre.
/// * `PolarToRect`: the exact inverse mapping.
pub fn apply_polar_coordinates_pro(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    mode: PolarMode,
    interpolation: f32,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let fw = width as f32;
    let fh = height as f32;
    let cx = fw * 0.5;
    let cy = fh * 0.5;
    let mix = interpolation.clamp(0.0, 1.0);

    for y in 0..height {
        for x in 0..width {
            let (sx, sy) = match mode {
                PolarMode::RectToPolar => {
                    let a = (x as f32 / fw) * TAU;
                    let r = cy - y as f32; // vertical distance from centre = radius
                    (cx + a.cos() * r, cy + a.sin() * r)
                }
                PolarMode::PolarToRect => {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let r = (dx * dx + dy * dy).sqrt();
                    let a = dy.atan2(dx).rem_euclid(TAU);
                    (a / TAU * fw, cy - r)
                }
            };
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, sx, sy, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            if mix < 1.0 {
                for c in 0..4 {
                    let orig = temp[idx + c] as f32;
                    rgba[c] = (orig + (rgba[c] as f32 - orig) * mix).round().clamp(0.0, 255.0) as u8;
                }
            }
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

// ───────────────────────── Optics Compensation ───────────────────────

/// Parameters for [`apply_optics_compensation`].
#[derive(Debug, Clone, Copy)]
pub struct OpticsCompensationParams {
    /// Field of view in degrees. Positive = barrel distortion removal
    /// (edge squeeze), negative = pincushion. 0 = identity.
    pub field_of_view_deg: f32,
    /// Reverse the distortion direction (AE "Reverse Lens Distortion").
    pub reverse: bool,
    /// Output zoom factor (1.0 = neutral).
    pub zoom: f32,
}

impl Default for OpticsCompensationParams {
    fn default() -> Self {
        Self { field_of_view_deg: 0.0, reverse: false, zoom: 1.0 }
    }
}

/// Radial lens distortion model `src_rn = rn * (1 + c * rn^2)` where the
/// curvature is derived from the field of view. Bilinear sampled.
pub fn apply_optics_compensation(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    p: &OpticsCompensationParams,
) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let cx = (width - 1) as f32 * 0.5;
    let cy = (height - 1) as f32 * 0.5;
    let rmax = (cx * cx + cy * cy).sqrt().max(1.0);
    let fov = p.field_of_view_deg.clamp(-179.0, 179.0);
    let mut c = (fov.abs() / 180.0).powi(2) * 0.9 * fov.signum();
    if p.reverse {
        c = -c;
    }
    let zoom = p.zoom.max(0.01);

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let rn = ((rx * rx + ry * ry).sqrt() / rmax * zoom).clamp(0.0, 1.0);
            let src_rn = (rn * (1.0 + c * rn * rn)).clamp(0.0, 1.0);
            let scale = if rn > 1e-6 { src_rn / rn } else { 1.0 };
            let mut rgba = [0u8; 4];
            sample_bilinear(&temp, width, height, cx + rx * scale, cy + ry * scale, &mut rgba);
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            v.extend_from_slice(&rgba);
        }
        v
    }

    #[test]
    fn test_waveforms_bounded_and_periodic() {
        for wt in [WaveType::Sine, WaveType::Triangle, WaveType::Square, WaveType::Sawtooth] {
            for i in 0..360 {
                let v = wt.waveform(i as f32 / 360.0 * TAU);
                assert!((-1.0..=1.0).contains(&v), "{wt:?} out of range at {i}");
                // Periodicity: phase + full turn reproduces the value.
                assert_eq!(v, wt.waveform(i as f32 / 360.0 * TAU + TAU));
            }
        }
        assert_eq!(WaveType::Square.waveform(0.0), 1.0);
        assert_eq!(WaveType::Square.waveform(PI), -1.0);
        assert!((WaveType::Triangle.waveform(PI * 0.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_wave_warp_preserves_size_and_is_deterministic() {
        let src = solid(16, 16, [128, 64, 32, 255]);
        let mut a = src.clone();
        let mut b = src.clone();
        let params = WaveWarpParams { wave_height: 6.0, wave_width: 8.0, time: 1.5, ..Default::default() };
        apply_wave_warp_pro(&mut a, 16, 16, &params);
        apply_wave_warp_pro(&mut b, 16, 16, &params);
        assert_eq!(a, b);
        assert_eq!(a.len(), src.len());
    }

    #[test]
    fn test_wave_warp_pin_all_freezes_corners() {
        // Gradient image so corners are distinguishable.
        let mut img = vec![0u8; 24 * 24 * 4];
        for y in 0..24u32 {
            for x in 0..24u32 {
                let i = ((y * 24 + x) * 4) as usize;
                img[i] = (x * 10) as u8;
                img[i + 1] = (y * 10) as u8;
                img[i + 3] = 255;
            }
        }
        let mut warped = img.clone();
        let params = WaveWarpParams { wave_height: 20.0, wave_width: 10.0, pinning: PinKind::All, ..Default::default() };
        apply_wave_warp_pro(&mut warped, 24, 24, &params);
        for (x, y) in [(0u32, 0u32), (23, 0), (0, 23), (23, 23)] {
            let i = ((y * 24 + x) * 4) as usize;
            assert_eq!(warped[i], img[i], "corner R changed");
            assert_eq!(warped[i + 1], img[i + 1], "corner G changed");
        }
    }

    #[test]
    fn test_cc_lens_center_invariant_and_identity_at_zero() {
        let mut img = vec![0u8; 17 * 17 * 4];
        for y in 0..17u32 {
            for x in 0..17u32 {
                let i = ((y * 17 + x) * 4) as usize;
                img[i] = (x * 15) as u8;
                img[i + 1] = (y * 15) as u8;
                img[i + 3] = 255;
            }
        }
        // Centre pixel must survive any convergence.
        for conv in [-80.0f32, 0.0, 80.0] {
            let mut out = img.clone();
            apply_cc_lens_pro(&mut out, 17, 17, &CcLensParams { convergence: conv, zoom: 1.0 });
            let c = ((8 * 17 + 8) * 4) as usize;
            assert_eq!(out[c], img[c]);
            assert_eq!(out[c + 1], img[c + 1]);
        }
        // Zero convergence + zoom 1 == identity everywhere.
        let mut out = img.clone();
        apply_cc_lens_pro(&mut out, 17, 17, &CcLensParams { convergence: 0.0, zoom: 1.0 });
        assert_eq!(out, img);
    }

    #[test]
    fn test_polar_solid_image_is_stable() {
        let src = solid(20, 20, [10, 200, 30, 255]);
        for mode in [PolarMode::RectToPolar, PolarMode::PolarToRect] {
            let mut out = src.clone();
            apply_polar_coordinates_pro(&mut out, 20, 20, mode, 1.0);
            assert!(out.chunks(4).all(|px| px == [10, 200, 30, 255]), "{mode:?} broke solid");
        }
        // Interpolation 0 == identity even on structured input.
        let mut grad = vec![0u8; 20 * 20 * 4];
        for (i, px) in grad.chunks_mut(4).enumerate() {
            px[0] = (i % 255) as u8;
            px[3] = 255;
        }
        let mut out = grad.clone();
        apply_polar_coordinates_pro(&mut out, 20, 20, PolarMode::RectToPolar, 0.0);
        assert_eq!(out, grad);
    }

    #[test]
    fn test_optics_compensation_identity_and_symmetry() {
        let mut img = vec![0u8; 16 * 16 * 4];
        for y in 0..16u32 {
            for x in 0..16u32 {
                let i = ((y * 16 + x) * 4) as usize;
                img[i] = (x * 16 + y) as u8;
                img[i + 3] = 255;
            }
        }
        let mut out = img.clone();
        apply_optics_compensation(&mut out, 16, 16, &OpticsCompensationParams::default());
        assert_eq!(out, img, "FOV 0 must be identity");

        // Centre pixel invariant under any FOV.
        for fov in [-120.0f32, 120.0] {
            let mut o = img.clone();
            apply_optics_compensation(&mut o, 16, 16, &OpticsCompensationParams {
                field_of_view_deg: fov, ..Default::default()
            });
            let c = ((8 * 16 + 8) * 4) as usize;
            assert_eq!(o[c], img[c]);
        }
    }

    #[test]
    fn test_sample_bilinear_clamps_and_interpolates() {
        let img = solid(2, 2, [0, 0, 0, 255]);
        let mut out = [0u8; 4];
        sample_bilinear(&img, 2, 2, -5.0, -5.0, &mut out);
        assert_eq!(out, [0, 0, 0, 255]);

        // Horizontal midpoint between black and white columns.
        let mut bw = vec![0u8; 2 * 1 * 4];
        bw[0..4].copy_from_slice(&[0, 0, 0, 255]);
        bw[4..8].copy_from_slice(&[200, 200, 200, 255]);
        sample_bilinear(&bw, 2, 1, 0.5, 0.0, &mut out);
        assert_eq!(out[0], 100);
    }

    #[test]
    fn test_degenerate_inputs_do_not_panic() {
        let mut empty: Vec<u8> = vec![];
        apply_wave_warp_pro(&mut empty, 0, 0, &WaveWarpParams::default());
        apply_cc_lens_pro(&mut empty, 0, 0, &CcLensParams::default());
        apply_polar_coordinates_pro(&mut empty, 0, 0, PolarMode::RectToPolar, 1.0);
        apply_optics_compensation(&mut empty, 0, 0, &OpticsCompensationParams::default());
    }
}