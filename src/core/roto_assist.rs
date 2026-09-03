//! RotoAssist: tracker-driven mask animation — the practical, non-ML stand-in
//! for Rotobrush-style assistance. Bakes a tracked point's motion onto every
//! vertex of a base polygon so the matte follows the footage automatically.

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::mask::Mask;
use crate::core::property::Animatable;
use crate::core::timeline::TrackerPoint;

/// Bake per-vertex keyframes: each polygon vertex follows the tracker's
/// sampled offset curve (position(t) − position(frame0)), preserving shape.
/// Returns a NEW animated mask path; caller assigns to `mask.path`.
pub fn bake_tracked_mask(
    base_mask: &Mask,
    tracker: &TrackerPoint,
    start_frame: u32,
    end_frame: u32,
) -> Result<Animatable<Vec<[f32; 2]>>, String> {
    let base_poly = base_mask.path.to_polygon(start_frame.max(1), 16);
    if base_poly.is_empty() {
        return Err("base mask has no vertices".into());
    }
    let origin = tracker.position.evaluate(start_frame);
    let mut kfs: Vec<Keyframe<Vec<[f32; 2]>>> = Vec::new();
    let step = ((end_frame - start_frame) / 60).max(1); // ≤ ~60 samples
    let mut f = start_frame;
    while f <= end_frame {
        let cur = tracker.position.evaluate(f);
        let dx = cur[0] - origin[0];
        let dy = cur[1] - origin[1];
        let moved: Vec<[f32; 2]> = base_poly.iter().map(|p| [p[0] + dx, p[1] + dy]).collect();
        kfs.push(Keyframe::new(f, moved, InterpolationType::Linear));
        f += step;
    }
    if kfs.is_empty() {
        return Err("no frames to bake".into());
    }
    Ok(Animatable::Animated(kfs))
}

// ──────────────── Roto Brush & Refine Edge Engine ────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RotoStroke {
    pub is_foreground: bool,
    pub points: Vec<[f32; 2]>,
    pub radius: f32,
}

/// Estimates binary foreground mask from user paint strokes (Green = Foreground, Red = Background).
pub fn segment_roto_brush(
    image: &[u8],
    width: u32,
    height: u32,
    strokes: &[RotoStroke],
) -> Vec<u8> {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    let mut mask = vec![0u8; len];
    let Some(image_len) = len.checked_mul(4) else {
        return mask;
    };
    if width == 0 || height == 0 || strokes.is_empty() || image.len() != image_len {
        return mask;
    }

    // Collect sampled foreground and background colors
    let mut fg_colors: Vec<[f32; 3]> = Vec::new();
    let mut bg_colors: Vec<[f32; 3]> = Vec::new();

    let w_i = width as i32;
    let h_i = height as i32;

    for stroke in strokes {
        let r = if stroke.radius.is_finite() {
            stroke.radius.clamp(1.0, 512.0) as i32
        } else {
            1
        };
        for &pt in &stroke.points {
            let cx = pt[0].round() as i32;
            let cy = pt[1].round() as i32;

            for dy in -r..=r {
                for dx in -r..=r {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x >= 0 && x < w_i && y >= 0 && y < h_i && (dx * dx + dy * dy) <= r * r {
                        let idx = ((y * w_i + x) * 4) as usize;
                        let col = [
                            image[idx] as f32,
                            image[idx + 1] as f32,
                            image[idx + 2] as f32,
                        ];
                        if stroke.is_foreground {
                            fg_colors.push(col);
                        } else {
                            bg_colors.push(col);
                        }
                    }
                }
            }
        }
    }

    if fg_colors.is_empty() {
        return mask;
    }

    // Classify each pixel based on minimum Euclidean color distance
    for y in 0..height {
        for x in 0..width {
            let px_idx = ((y * width + x) * 4) as usize;
            let p_col = [
                image[px_idx] as f32,
                image[px_idx + 1] as f32,
                image[px_idx + 2] as f32,
            ];

            let min_fg_dist = fg_colors
                .iter()
                .map(|&c| {
                    (c[0] - p_col[0]).powi(2)
                        + (c[1] - p_col[1]).powi(2)
                        + (c[2] - p_col[2]).powi(2)
                })
                .fold(f32::INFINITY, f32::min);

            let min_bg_dist = if bg_colors.is_empty() {
                2500.0 // Default threshold
            } else {
                bg_colors
                    .iter()
                    .map(|&c| {
                        (c[0] - p_col[0]).powi(2)
                            + (c[1] - p_col[1]).powi(2)
                            + (c[2] - p_col[2]).powi(2)
                    })
                    .fold(f32::INFINITY, f32::min)
            };

            let out_idx = (y * width + x) as usize;
            if min_fg_dist < min_bg_dist {
                mask[out_idx] = 255;
            } else {
                mask[out_idx] = 0;
            }
        }
    }

    mask
}

/// Trace the boundary of a binary mask (0/255) and return simplified polygon vertices.
/// Uses Moore-neighborhood contour tracing with Douglas-Peucker simplification.
pub fn trace_contour_to_polygon(
    mask: &[u8],
    width: u32,
    height: u32,
    simplify_tolerance: f32,
) -> Vec<[f32; 2]> {
    let w = width as i32;
    let h = height as i32;
    if mask.is_empty() || w == 0 || h == 0 {
        return Vec::new();
    }
    let at = |x: i32, y: i32| -> u8 {
        if x >= 0 && x < w && y >= 0 && y < h {
            mask[(y * w + x) as usize]
        } else {
            0
        }
    };
    // Find a starting border pixel (leftmost foreground pixel on the topmost row)
    let mut start: Option<(i32, i32)> = None;
    'outer: for y in 0..h {
        for x in 0..w {
            if at(x, y) == 255 {
                start = Some((x, y));
                break 'outer;
            }
        }
    }
    let (sx, sy) = match start {
        Some(s) => s,
        None => return Vec::new(),
    };
    // Moore neighborhood directions: E, NE, N, NW, W, SW, S, SE
    let dirs: [(i32, i32); 8] = [
        (1, 0),
        (1, -1),
        (0, -1),
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    let mut contour: Vec<[f32; 2]> = Vec::new();
    let mut bx = sx;
    let mut by = sy;
    let mut dir = 6usize; // start searching from S (we came from the left)
    let max_iters = (w * h * 4) as usize;
    for _ in 0..max_iters {
        contour.push([bx as f32 + 0.5, by as f32 + 0.5]);
        // Search neighbors starting from (dir + 5) % 8 (turn left from entry direction)
        let mut found = false;
        for step in 0..8 {
            let nd = (dir + 5 + step) % 8;
            let nx = bx + dirs[nd].0;
            let ny = by + dirs[nd].1;
            if at(nx, ny) == 255 {
                bx = nx;
                by = ny;
                dir = nd;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
        if bx == sx && by == sy {
            break;
        }
    }
    if contour.len() < 3 {
        return contour;
    }
    // Douglas-Peucker simplification
    fn dp_simplify(pts: &[[f32; 2]], eps: f32) -> Vec<[f32; 2]> {
        if pts.len() <= 2 {
            return pts.to_vec();
        }
        let mut max_d = 0.0f32;
        let mut max_i = 0usize;
        let (p0, p1) = (pts[0], pts[pts.len() - 1]);
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len_sq = dx * dx + dy * dy;
        for (i, pt) in pts.iter().enumerate().skip(1).take(pts.len() - 2) {
            let d = if len_sq < 1e-12 {
                ((pt[0] - p0[0]).powi(2) + (pt[1] - p0[1]).powi(2)).sqrt()
            } else {
                let t = ((pt[0] - p0[0]) * dx + (pt[1] - p0[1]) * dy) / len_sq;
                let t = t.clamp(0.0, 1.0);
                let proj_x = p0[0] + t * dx;
                let proj_y = p0[1] + t * dy;
                ((pt[0] - proj_x).powi(2) + (pt[1] - proj_y).powi(2)).sqrt()
            };
            if d > max_d {
                max_d = d;
                max_i = i;
            }
        }
        if max_d > eps {
            let left = dp_simplify(&pts[..=max_i], eps);
            let right = dp_simplify(&pts[max_i..], eps);
            let mut result = left;
            result.extend_from_slice(&right[1..]);
            result
        } else {
            vec![pts[0], pts[pts.len() - 1]]
        }
    }
    dp_simplify(&contour, simplify_tolerance)
}

/// Refines hair and soft translucent edges using a Guided Filter against the RGB guide image.
pub fn refine_edge_guided_filter(
    guide_image: &[u8],
    rough_mask: &[u8],
    width: u32,
    height: u32,
    radius: i32,
    eps: f32,
) -> Vec<u8> {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    let mut refined = vec![0u8; len];
    let Some(image_len) = len.checked_mul(4) else {
        return refined;
    };
    if width == 0 || height == 0 || guide_image.len() != image_len || rough_mask.len() != len {
        return refined;
    }

    let r = radius.clamp(1, 64);
    let w_i = width as i32;
    let h_i = height as i32;

    // Convert guide to grayscale I and mask to p (0..1)
    let mut i_gray = vec![0.0f32; len];
    let mut p_val = vec![0.0f32; len];

    for i in 0..len {
        let px = i * 4;
        i_gray[i] = (guide_image[px] as f32 * 0.299
            + guide_image[px + 1] as f32 * 0.587
            + guide_image[px + 2] as f32 * 0.114)
            / 255.0;
        p_val[i] = rough_mask[i] as f32 / 255.0;
    }

    // Guided filter local linear model: q = a * I + b
    let mut a_vals = vec![0.0f32; len];
    let mut b_vals = vec![0.0f32; len];

    for y in 0..h_i {
        for x in 0..w_i {
            let mut mean_i = 0.0f32;
            let mut mean_p = 0.0f32;
            let mut mean_ii = 0.0f32;
            let mut mean_ip = 0.0f32;
            let mut count = 0.0f32;

            for dy in -r..=r {
                for dx in -r..=r {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx >= 0 && nx < w_i && ny >= 0 && ny < h_i {
                        let idx = (ny * w_i + nx) as usize;
                        let i_v = i_gray[idx];
                        let p_v = p_val[idx];
                        mean_i += i_v;
                        mean_p += p_v;
                        mean_ii += i_v * i_v;
                        mean_ip += i_v * p_v;
                        count += 1.0;
                    }
                }
            }

            mean_i /= count;
            mean_p /= count;
            mean_ii /= count;
            mean_ip /= count;

            let var_i = mean_ii - mean_i * mean_i;
            let cov_ip = mean_ip - mean_i * mean_p;

            let a = cov_ip / (var_i + eps);
            let b = mean_p - a * mean_i;

            let idx = (y * w_i + x) as usize;
            a_vals[idx] = a;
            b_vals[idx] = b;
        }
    }

    // Output q = mean(a) * I + mean(b)
    for y in 0..h_i {
        for x in 0..w_i {
            let idx = (y * w_i + x) as usize;
            let q = (a_vals[idx] * i_gray[idx] + b_vals[idx]).clamp(0.0, 1.0);
            refined[idx] = (q * 255.0).round() as u8;
        }
    }

    refined
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mask::MaskPath;

    fn square_mask() -> Mask {
        // 12x12 square at origin
        crate::core::mask::Mask::new_rect("m".into(), "M".into(), 0.0, 0.0, 12.0, 12.0)
    }

    fn moving_tracker() -> TrackerPoint {
        let mut t = TrackerPoint::new("t".into(), "T".into(), [100.0, 100.0]);
        t.position = Animatable::Animated(vec![
            Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(10, [140.0, 90.0], InterpolationType::Linear),
        ]);
        t
    }

    #[test]
    fn test_bake_translates_shape_with_tracker() {
        let m = square_mask();
        let t = moving_tracker();
        let baked = bake_tracked_mask(&m, &t, 0, 10).expect("bakes");
        match &baked {
            Animatable::Animated(kfs) => {
                assert!(kfs.len() >= 2);
                let first = kfs[0].value.clone();
                let last = kfs.last().unwrap().value.clone();
                // Shape translated by (40, -10)
                assert!((last[0][0] - first[0][0] - 40.0).abs() < 0.01);
                assert!((last[0][1] - first[0][1] + 10.0).abs() < 0.01);
            }
            _ => panic!("expected animated"),
        }
    }

    #[test]
    fn test_empty_base_mask_errors() {
        let mut m = Mask::new_rect("e".into(), "E".into(), 0.0, 0.0, 8.0, 8.0);
        m.path.vertices = Animatable::Animated(vec![]); // no vertices at all
        let t = moving_tracker();
        assert!(bake_tracked_mask(&m, &t, 0, 10).is_err());
    }

    #[test]
    fn test_static_tracker_yields_constant_motion() {
        let m = square_mask();
        let t = TrackerPoint::new("t".into(), "T".into(), [5.0, 5.0]);
        let baked = bake_tracked_mask(&m, &t, 0, 20).expect("bakes");
        if let Animatable::Animated(kfs) = &baked {
            let first = kfs[0].value.clone();
            let last = kfs.last().unwrap().value.clone();
            assert_eq!(first[0], last[0], "no motion → identical vertices");
        } else {
            panic!("expected animated");
        }
    }

    #[test]
    fn test_roto_brush_segmentation_and_refine_edge() {
        let width = 16u32;
        let height = 16u32;
        let mut img = vec![0u8; (width * height * 4) as usize];

        // Left half Red (Foreground subject), Right half Blue (Background)
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                if x < 8 {
                    img[idx] = 250;
                    img[idx + 1] = 20;
                    img[idx + 2] = 20;
                    img[idx + 3] = 255;
                } else {
                    img[idx] = 20;
                    img[idx + 1] = 20;
                    img[idx + 2] = 250;
                    img[idx + 3] = 255;
                }
            }
        }

        let strokes = vec![
            RotoStroke {
                is_foreground: true,
                points: vec![[4.0, 8.0]],
                radius: 2.0,
            },
            RotoStroke {
                is_foreground: false,
                points: vec![[12.0, 8.0]],
                radius: 2.0,
            },
        ];

        let rough_mask = segment_roto_brush(&img, width, height, &strokes);
        assert_eq!(
            rough_mask[8 * 16 + 2],
            255,
            "Left half must be segmented as foreground"
        );
        assert_eq!(
            rough_mask[8 * 16 + 14],
            0,
            "Right half must be segmented as background"
        );

        let refined = refine_edge_guided_filter(&img, &rough_mask, width, height, 2, 0.01);
        assert_eq!(refined.len(), (width * height) as usize);
        assert!(refined[8 * 16 + 2] > 200);
    }
}
