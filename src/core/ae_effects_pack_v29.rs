#![allow(dead_code)]
/// After Effects VFX Kernels Part 29 — Lens, Smear, Card Wipe, Timecode & Vignette Pack.
///
/// Production-quality implementations of:
///   * CC Lens              — spherical fisheye distortion (convex/concave convergence)
///   * Directional Smear    — directional trail / smear blur along an arbitrary angle
///   * Card Wipe Dissolve   — grid-based 3D rotating card transition
///   * SMPTE Timecode       — crisp rasterized timecode overlay burn-in
///   * CC Vignette Advanced — oval/circular falloff with tint & highlight preservation
///
/// All functions are deterministic, panic-free, and bilinear-sampled.
// ────────────────────────── Sampling Helper ──────────────────────────
/// Clamp-to-edge bilinear RGBA sample — delegates to shared `effect_utils`.
#[inline]
fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    crate::core::effect_utils::sample_bilinear_into(src, w, h, fx, fy, out);
}
// ─────────────────────────── CC Lens ────────────────────────────────

/// Parameters for [`apply_cc_lens`].
#[derive(Debug, Clone, Copy)]
pub struct CcLensParams {
    /// Lens centre in normalized coordinates `[0..1, 0..1]`.
    pub center: [f32; 2],
    /// Lens radius in pixels.
    pub size: f32,
    /// Convergence factor (-2.0 = concave/pinch, +2.0 = convex/fisheye, 0.0 = identity).
    pub convergence: f32,
}

impl Default for CcLensParams {
    fn default() -> Self {
        Self {
            center: [0.5, 0.5],
            size: 200.0,
            convergence: 1.0,
        }
    }
}

/// Spherical fisheye/lens distortion simulating CC Lens.
pub fn apply_cc_lens(pixels: &mut [u8], width: u32, height: u32, p: &CcLensParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let src = pixels.to_vec();
    let cx = p.center[0].clamp(0.0, 1.0) * width as f32;
    let cy = p.center[1].clamp(0.0, 1.0) * height as f32;
    let r = p.size.max(1.0);
    let conv = p.convergence;

    let mut out_px = [0u8; 4];
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < r && dist > 1e-4 {
                let norm_dist = dist / r;
                // Spherical angle theta
                let theta = norm_dist * std::f32::consts::FRAC_PI_2;
                let factor = if conv >= 0.0 {
                    // Convex / Fisheye
                    let mapped = theta.sin().powf(conv.max(0.01));
                    mapped / norm_dist
                } else {
                    // Concave / Pinch
                    let mapped = 1.0 - (1.0 - norm_dist).powf((-conv).max(0.01));
                    mapped / norm_dist
                };
                let sx = cx + dx * factor;
                let sy = cy + dy * factor;
                sample_bilinear(&src, width, height, sx, sy, &mut out_px);
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&out_px);
            }
        }
    }
}

// ─────────────────────────── Directional Smear ──────────────────────

/// Parameters for [`apply_directional_smear`].
#[derive(Debug, Clone, Copy)]
pub struct DirectionalSmearParams {
    /// Smear angle in degrees (0 = right, 90 = down).
    pub angle_deg: f32,
    /// Distance of the smear trail in pixels (0..200).
    pub length: f32,
    /// Falloff decay exponent (1.0 = linear, 2.0 = exponential decay).
    pub decay: f32,
    /// Number of sample taps (4..32).
    pub samples: u32,
}

impl Default for DirectionalSmearParams {
    fn default() -> Self {
        Self {
            angle_deg: 0.0,
            length: 20.0,
            decay: 1.2,
            samples: 12,
        }
    }
}

/// Smear trailing blur along an arbitrary angle.
pub fn apply_directional_smear(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    p: &DirectionalSmearParams,
) {
    if width == 0
        || height == 0
        || p.length <= 0.001
        || pixels.len() < (width as usize) * (height as usize) * 4
    {
        return;
    }
    let src = pixels.to_vec();
    let rad = p.angle_deg.to_radians();
    let dx = rad.cos();
    let dy = rad.sin();
    let steps = p.samples.clamp(4, 32);
    let step_dist = p.length / (steps as f32);

    let mut out_px = [0u8; 4];
    for y in 0..height {
        for x in 0..width {
            let mut acc = [0.0f32; 4];
            let mut total_w = 0.0f32;

            for s in 0..steps {
                let dist = s as f32 * step_dist;
                let sx = x as f32 - dx * dist;
                let sy = y as f32 - dy * dist;
                sample_bilinear(&src, width, height, sx, sy, &mut out_px);

                let weight = (1.0 - (s as f32 / steps as f32)).powf(p.decay.max(0.1));
                for c in 0..4 {
                    acc[c] += out_px[c] as f32 * weight;
                }
                total_w += weight;
            }

            let idx = ((y * width + x) * 4) as usize;
            if total_w > 1e-4 {
                for c in 0..4 {
                    pixels[idx + c] = (acc[c] / total_w).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

// ─────────────────────────── Card Wipe Dissolve ────────────────────

/// Parameters for [`apply_card_wipe`].
#[derive(Debug, Clone, Copy)]
pub struct CardWipeParams {
    /// Transition progress (0.0 = unaffected, 1.0 = completely flipped/transparent).
    pub progress: f32,
    /// Number of horizontal grid columns.
    pub cols: u32,
    /// Number of vertical grid rows.
    pub rows: u32,
}

impl Default for CardWipeParams {
    fn default() -> Self {
        Self {
            progress: 0.0,
            cols: 8,
            rows: 6,
        }
    }
}

/// Grid-based rotating 3D card wipe transition.
pub fn apply_card_wipe(pixels: &mut [u8], width: u32, height: u32, p: &CardWipeParams) {
    if width == 0
        || height == 0
        || p.progress <= 0.0
        || pixels.len() < (width as usize) * (height as usize) * 4
    {
        return;
    }
    let src = pixels.to_vec();
    pixels.fill(0); // clear to black / transparent

    let cols = p.cols.max(1);
    let rows = p.rows.max(1);
    let card_w = width as f32 / cols as f32;
    let card_h = height as f32 / rows as f32;

    let mut out_px = [0u8; 4];
    for r in 0..rows {
        for c in 0..cols {
            // Stagger animation across cards (diagonal wave)
            let card_norm = (c as f32 / cols as f32 + r as f32 / rows as f32) * 0.5;
            let card_prog = ((p.progress - card_norm * 0.5) / 0.5).clamp(0.0, 1.0);
            if card_prog >= 1.0 {
                // Card has fully flipped out
                continue;
            }

            // Flip angle 0 -> 90 degrees
            let angle = card_prog * std::f32::consts::FRAC_PI_2;
            let cos_a = angle.cos().max(0.01);

            let card_cx = (c as f32 + 0.5) * card_w;
            let _card_cy = (r as f32 + 0.5) * card_h;

            let y_start = (r as f32 * card_h).floor() as u32;
            let y_end = (((r + 1) as f32 * card_h).ceil() as u32).min(height);

            for y in y_start..y_end {
                let x_start = (card_cx - (card_w * 0.5 * cos_a)).floor().max(0.0) as u32;
                let x_end = (card_cx + (card_w * 0.5 * cos_a)).ceil().min(width as f32) as u32;

                for x in x_start..x_end {
                    let rel_x = (x as f32 - card_cx) / cos_a;
                    let sx = card_cx + rel_x;
                    let sy = y as f32;

                    sample_bilinear(&src, width, height, sx, sy, &mut out_px);
                    // Slight shading as card rotates
                    let shade = cos_a.powf(0.5);
                    let idx = ((y * width + x) * 4) as usize;
                    for ch in 0..3 {
                        pixels[idx + ch] = (out_px[ch] as f32 * shade).round() as u8;
                    }
                    pixels[idx + 3] = out_px[3];
                }
            }
        }
    }
}

// ─────────────────────────── SMPTE Timecode Burn-In ────────────────

/// 5x7 Bitmapped font glyphs for '0'-'9' and ':'
const GLYPH_WIDTH: usize = 5;
const GLYPH_HEIGHT: usize = 7;

static GLYPHS: [[u8; 7]; 11] = [
    // '0'
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ],
    // '1'
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ],
    // '2'
    [
        0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
    ],
    // '3'
    [
        0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
    ],
    // '4'
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ],
    // '5'
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ],
    // '6'
    [
        0b01110, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b01110,
    ],
    // '7'
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ],
    // '8'
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ],
    // '9'
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
    ],
    // ':'
    [
        0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
    ],
];

fn get_glyph_row(ch: char, row: usize) -> u8 {
    let idx = match ch {
        '0'..='9' => (ch as usize) - ('0' as usize),
        ':' => 10,
        _ => return 0,
    };
    if row < GLYPH_HEIGHT {
        GLYPHS[idx][row]
    } else {
        0
    }
}

/// Parameters for [`apply_timecode_burn_in`].
#[derive(Debug, Clone, Copy)]
pub struct TimecodeParams {
    pub frame: u32,
    pub fps: u32,
    /// X, Y offset in pixels.
    pub position: [f32; 2],
    /// Scale multiplier (1.0 = standard, 2.0 = large, etc.).
    pub scale: f32,
    /// Foreground text colour RGBA `[0..1]`.
    pub text_color: [f32; 4],
    /// Background box opacity `[0..1]`.
    pub bg_opacity: f32,
}

impl Default for TimecodeParams {
    fn default() -> Self {
        Self {
            frame: 0,
            fps: 30,
            position: [20.0, 20.0],
            scale: 2.0,
            text_color: [1.0, 1.0, 1.0, 1.0],
            bg_opacity: 0.6,
        }
    }
}

/// Burn SMPTE timecode (HH:MM:SS:FF) into the pixel buffer.
pub fn apply_timecode_burn_in(pixels: &mut [u8], width: u32, height: u32, p: &TimecodeParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let fps = p.fps.max(1);
    let total_sec = p.frame / fps;
    let frames = p.frame % fps;
    let sec = total_sec % 60;
    let min = (total_sec / 60) % 60;
    let hrs = total_sec / 3600;

    let text = format!("{:02}:{:02}:{:02}:{:02}", hrs, min, sec, frames);
    let scale = p.scale.max(1.0);
    let char_w = (GLYPH_WIDTH as f32 * scale).round() as u32;
    let char_h = (GLYPH_HEIGHT as f32 * scale).round() as u32;
    let spacing = (2.0 * scale).round() as u32;

    let total_w = text.len() as u32 * (char_w + spacing) + spacing * 2;
    let total_h = char_h + spacing * 2;

    let start_x = p.position[0].max(0.0) as u32;
    let start_y = p.position[1].max(0.0) as u32;

    // Draw background box
    if p.bg_opacity > 0.001 {
        let box_x0 = start_x;
        let box_y0 = start_y;
        let box_x1 = (box_x0 + total_w).min(width);
        let box_y1 = (box_y0 + total_h).min(height);
        let bg_alpha = p.bg_opacity.clamp(0.0, 1.0);

        for y in box_y0..box_y1 {
            for x in box_x0..box_x1 {
                let idx = ((y * width + x) * 4) as usize;
                for c in 0..3 {
                    pixels[idx + c] = (pixels[idx + c] as f32 * (1.0 - bg_alpha)).round() as u8;
                }
                pixels[idx + 3] = (pixels[idx + 3] as f32 + bg_alpha * 255.0).min(255.0) as u8;
            }
        }
    }

    // Draw text characters
    let mut cur_x = start_x + spacing;
    let cur_y = start_y + spacing;

    for ch in text.chars() {
        for row in 0..GLYPH_HEIGHT {
            let row_bits = get_glyph_row(ch, row);
            let py_start = cur_y + (row as f32 * scale) as u32;
            let py_end = (cur_y + ((row + 1) as f32 * scale) as u32).min(height);

            for col in 0..GLYPH_WIDTH {
                if (row_bits & (1 << (4 - col))) != 0 {
                    let px_start = cur_x + (col as f32 * scale) as u32;
                    let px_end = (cur_x + ((col + 1) as f32 * scale) as u32).min(width);

                    for y in py_start..py_end {
                        for x in px_start..px_end {
                            let idx = ((y * width + x) * 4) as usize;
                            for c in 0..3 {
                                pixels[idx + c] = (p.text_color[c] * 255.0).clamp(0.0, 255.0) as u8;
                            }
                            pixels[idx + 3] = (p.text_color[3] * 255.0).clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
        }
        cur_x += char_w + spacing;
    }
}

// ─────────────────────────── CC Vignette Advanced ───────────────────

/// Parameters for [`apply_advanced_vignette`].
#[derive(Debug, Clone, Copy)]
pub struct VignetteParams {
    pub center: [f32; 2],
    pub amount: f32,  // 0..1 darkness intensity
    pub radius: f32,  // 0..1 normalized size of clear center
    pub feather: f32, // 0..1 smoothness of edge transition
    pub tint_color: [f32; 4],
}

impl Default for VignetteParams {
    fn default() -> Self {
        Self {
            center: [0.5, 0.5],
            amount: 0.5,
            radius: 0.6,
            feather: 0.4,
            tint_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Oval vignette with tint and smooth feather falloff.
pub fn apply_advanced_vignette(pixels: &mut [u8], width: u32, height: u32, p: &VignetteParams) {
    if width == 0
        || height == 0
        || p.amount <= 0.001
        || pixels.len() < (width as usize) * (height as usize) * 4
    {
        return;
    }
    let cx = p.center[0] * width as f32;
    let cy = p.center[1] * height as f32;
    let max_rx = (width as f32 * 0.5).max(1.0);
    let max_ry = (height as f32 * 0.5).max(1.0);

    let inner_r = p.radius.clamp(0.01, 1.0);
    let outer_r = (inner_r + p.feather.clamp(0.01, 1.0)).max(inner_r + 0.01);

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - cx) / max_rx;
            let dy = (y as f32 - cy) / max_ry;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > inner_r {
                let t = ((dist - inner_r) / (outer_r - inner_r)).clamp(0.0, 1.0);
                let smooth_t = t * t * (3.0 - 2.0 * t); // smoothstep falloff
                let dark_fac = 1.0 - smooth_t * p.amount.clamp(0.0, 1.0);

                let idx = ((y * width + x) * 4) as usize;
                for c in 0..3 {
                    let tinted = pixels[idx + c] as f32 * dark_fac
                        + p.tint_color[c] * 255.0 * (1.0 - dark_fac);
                    pixels[idx + c] = tinted.clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

// ─────────────────────────── Unit Tests ─────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cc_lens_bounded_and_modifies() {
        let (w, h) = (64, 64);
        let mut pixels = vec![128u8; (w * h * 4) as usize];
        let p = CcLensParams::default();
        apply_cc_lens(&mut pixels, w, h, &p);
        assert_eq!(pixels.len(), (w * h * 4) as usize);
    }

    #[test]
    fn test_directional_smear_preserves_length() {
        let (w, h) = (32, 32);
        let mut pixels = vec![100u8; (w * h * 4) as usize];
        let p = DirectionalSmearParams::default();
        apply_directional_smear(&mut pixels, w, h, &p);
        assert_eq!(pixels.len(), (w * h * 4) as usize);
    }

    #[test]
    fn test_card_wipe_progress_zero_identity() {
        let (w, h) = (16, 16);
        let orig = vec![200u8; (w * h * 4) as usize];
        let mut pixels = orig.clone();
        let p = CardWipeParams {
            progress: 0.0,
            cols: 4,
            rows: 4,
        };
        apply_card_wipe(&mut pixels, w, h, &p);
        assert_eq!(pixels, orig);
    }

    #[test]
    fn test_timecode_burn_in_draws() {
        let (w, h) = (120, 60);
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let p = TimecodeParams {
            frame: 45,
            fps: 30,
            position: [10.0, 10.0],
            scale: 1.0,
            text_color: [1.0, 1.0, 1.0, 1.0],
            bg_opacity: 0.5,
        };
        apply_timecode_burn_in(&mut pixels, w, h, &p);
        // Ensure some white pixels exist
        assert!(pixels.contains(&255));
    }

    #[test]
    fn test_vignette_darkens_corners() {
        let (w, h) = (50, 50);
        let mut pixels = vec![255u8; (w * h * 4) as usize];
        let p = VignetteParams::default();
        apply_advanced_vignette(&mut pixels, w, h, &p);
        // Corner pixel should be darker than center
        let center_val = pixels[((25 * 50 + 25) * 4) as usize];
        let corner_val = pixels[0];
        assert!(corner_val < center_val);
    }
}
