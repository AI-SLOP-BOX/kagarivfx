use crate::core::effect_plugin::evaluate_effects;
use crate::core::timeline::{Composition, LayerType, ShapeType};

use std::sync::Arc;

// Helper matrix functions
#[allow(dead_code)]
fn mat4_identity() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[allow(dead_code)]
fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] =
                a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c] + a[r][3] * b[3][c];
        }
    }
    out
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5],
        tex_coords: [0.0, 0.0],
    }, // Top-Left
    Vertex {
        position: [-0.5, -0.5],
        tex_coords: [0.0, 1.0],
    }, // Bottom-Left
    Vertex {
        position: [0.5, -0.5],
        tex_coords: [1.0, 1.0],
    }, // Bottom-Right
    Vertex {
        position: [0.5, 0.5],
        tex_coords: [1.0, 0.0],
    }, // Top-Right
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalsUniform {
    viewport_size: [f32; 2],
    exposure_ev: f32,
    lut_mode: u32,
}

/// Compile-time proof that LayerUniform is non-zero-sized, embedded directly
/// into bind-group setup — no runtime unwrap needed anywhere.
const LAYER_UNIFORM_SIZE: std::num::NonZeroU64 =
    std::num::NonZeroU64::new(std::mem::size_of::<LayerUniform>() as u64)
        .expect("LayerUniform holds f32 fields and can never be zero-sized");

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LayerUniform {
    transform_matrix: [[f32; 4]; 4],
    color: [f32; 4],
    opacity: f32,
    layer_type: u32,
    shape_type: u32,

    effect_tint_enabled: u32,
    effect_tint_color: [f32; 4],
    effect_tint_intensity: f32,
    effect_blur_enabled: u32,
    effect_blur_radius: f32,

    effect_shadow_enabled: u32,
    effect_shadow_color: [f32; 4],
    effect_shadow_opacity: f32,
    effect_shadow_direction: f32,
    effect_shadow_distance: f32,
    effect_shadow_softness: f32,

    effect_ca_enabled: u32,
    effect_ca_shift_r: f32,
    effect_ca_shift_b: f32,
    effect_ca_edge_falloff: f32,

    effect_vignette_enabled: u32,
    effect_vignette_intensity: f32,
    effect_vignette_roundness: f32,
    effect_vignette_feather: f32,
    effect_vignette_color: [f32; 4],
    blend_mode: u32,

    // Levels Adjustment
    levels_enabled: u32,
    levels_in_black: f32,
    levels_in_white: f32,
    levels_gamma: f32,
    levels_out_black: f32,
    levels_out_white: f32,

    // Hue / Saturation
    huesat_enabled: u32,
    huesat_hue: f32,
    huesat_sat: f32,
    huesat_light: f32,

    // Glow / Bloom
    glow_enabled: u32,
    glow_threshold: f32,
    glow_radius: f32,
    glow_intensity: f32,
    glow_color: [f32; 4],

    // Physical Film Grain
    grain_enabled: u32,
    grain_intensity: f32,
    grain_size: f32,

    // Track Matte System
    track_matte_mode: u32,

    // Shape params: x = polygon/star point count, y = rectangle corner radius (px)
    shape_params: [f32; 4],

    // Mesh Warp / Corner Pin
    meshwarp_enabled: u32,
    corner_top_left: [f32; 2],
    corner_top_right: [f32; 2],
    corner_bottom_left: [f32; 2],
    corner_bottom_right: [f32; 2],

    // Per-layer GPU mask flags (coverage baked CPU-side; see rasterize_layer_masks)
    mask_enabled: u32,
    mask_mode: u32,
    mask_inverted: u32,
    mask_feather: f32,

    _padding_align: [[f32; 4]; 9], // Keep total size a multiple of 256 for WGPU dynamic uniform offsets
}

// ─── GPU Layer Mask Rasterization (CPU-baked coverage → group(3) texture) ──

/// One evaluated mask shape, already scaled into output pixel space.
#[derive(Debug, Clone)]
pub(crate) struct MaskShape {
    pub poly: Vec<[f32; 2]>,
    pub mode: crate::core::mask::MaskMode,
    pub opacity: f32,
    pub inverted: bool,
    /// Feather radius in output pixels (0 = hard edge).
    pub feather: f32,
}

/// CPU-rasterized combined mask coverage for one layer, ready for GPU upload.
pub(crate) struct MaskRaster {
    /// Hash of the mask INPUTS (scaled shapes + output size) — identical
    /// inputs across frames hit the cache and share one upload.
    pub key: u64,
    /// RGBA8 pixels; rgb = white, a = combined coverage.
    pub pixels: Vec<u8>,
}

/// Content-addressed FIFO cache so static masks skip the (expensive) EDT
/// re-raster while OTHER layers keep animating during playback.
#[derive(Default)]
struct MaskRasterCache {
    order: Vec<u64>,
    map: std::collections::HashMap<u64, std::sync::Arc<MaskRaster>>,
}

impl MaskRasterCache {
    const CAP: usize = 6;

    fn get(&self, key: u64) -> Option<std::sync::Arc<MaskRaster>> {
        self.map.get(&key).cloned()
    }

    fn insert(&mut self, raster: std::sync::Arc<MaskRaster>) {
        let key = raster.key;
        if self.map.insert(key, raster).is_none() {
            self.order.push(key);
            while self.order.len() > Self::CAP {
                let evicted = self.order.remove(0);
                self.map.remove(&evicted);
            }
        }
    }
}

/// Hashes only the mask INPUTS — cheaper than hashing the full pixel buffer
/// and enables lookup before any rasterization work.
fn mask_input_key(shapes: &[MaskShape], width: u32, height: u32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    for shape in shapes {
        for pt in &shape.poly {
            pt[0].to_bits().hash(&mut hasher);
            pt[1].to_bits().hash(&mut hasher);
        }
        (shape.mode as u8).hash(&mut hasher);
        shape.opacity.to_bits().hash(&mut hasher);
        shape.inverted.hash(&mut hasher);
        shape.feather.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Even-odd scanline polygon fill with pixel-center sampling.
/// Returns per-pixel binary coverage in row-major order.
fn rasterize_polygon_evenodd(poly: &[[f32; 2]], width: u32, height: u32) -> Vec<f32> {
    let mut cov = vec![0.0f32; (width as usize) * (height as usize)];
    let n = poly.len();
    if n < 3 || width == 0 || height == 0 {
        return cov;
    }
    for y in 0..height {
        let py = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::new();
        let mut j = n - 1;
        for i in 0..n {
            let (x1, y1) = (poly[j][0], poly[j][1]);
            let (x2, y2) = (poly[i][0], poly[i][1]);
            if (y1 <= py && y2 > py) || (y2 <= py && y1 > py) {
                let t = (py - y1) / (y2 - y1);
                xs.push(x1 + t * (x2 - x1));
            }
            j = i;
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let row = y as usize * width as usize;
        let mut k = 0;
        while k + 1 < xs.len() {
            let xa = xs[k];
            let xb = xs[k + 1];
            if xb <= xa {
                k += 2;
                continue;
            }
            let x_start = ((xa - 0.5).ceil().max(0.0)) as usize;
            let x_end_incl = ((xb - 0.5).floor().min(width as f32 - 1.0)).max(-1.0) as usize;
            for x in x_start..=(x_end_incl.min(width as usize - 1)) {
                let cx = x as f32 + 0.5;
                if cx >= xa && cx < xb {
                    cov[row + x] = 1.0;
                }
            }
            k += 2;
        }
    }
    cov
}

/// One pass of the Felzenszwalb–Huttenlocher 1D squared distance transform.
/// `f` holds per-site squared distances (INF where no target site exists).
fn edt_1d(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        let mut s = ((f[q] + (q * q) as f64) - (f[v[k]] + (v[k] * v[k]) as f64))
            / (2.0 * (q - v[k]) as f64);
        while s <= z[k] {
            k -= 1;
            s = ((f[q] + (q * q) as f64) - (f[v[k]] + (v[k] * v[k]) as f64))
                / (2.0 * (q - v[k]) as f64);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, dq) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        // Cast before subtracting: with all-INF rows the envelope can hold
        // sites past q, and usize underflow would panic in debug builds.
        let dx = q as f64 - v[k] as f64;
        *dq = dx * dx + f[v[k]];
    }
}

/// Exact Euclidean distance (px) from every pixel to the nearest pixel where
/// `binary == target`. Two-pass separable EDT — O(w·h), branch-stable, and
/// fully deterministic for identical inputs.
fn edt_2d(binary: &[bool], width: u32, height: u32, target: bool) -> Vec<f32> {
    const INF: f64 = 1.0e18;
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mut grid = vec![0.0f64; w * h];
    for (i, g) in grid.iter_mut().enumerate() {
        *g = if binary[i] == target { 0.0 } else { INF };
    }
    let maxlen = w.max(h);
    let mut f = vec![0.0f64; maxlen];
    let mut d = vec![0.0f64; maxlen];
    let mut v = vec![0usize; maxlen];
    let mut z = vec![0.0f64; maxlen + 1];

    for y in 0..h {
        f[..w].copy_from_slice(&grid[y * w..(y + 1) * w]);
        edt_1d(&f[..w], &mut d[..w], &mut v[..w], &mut z[..w + 1]);
        grid[y * w..(y + 1) * w].copy_from_slice(&d[..w]);
    }
    for x in 0..w {
        for y in 0..h {
            f[y] = grid[y * w + x];
        }
        edt_1d(&f[..h], &mut d[..h], &mut v[..h], &mut z[..h + 1]);
        for y in 0..h {
            grid[y * w + x] = d[y];
        }
    }
    grid.into_iter().map(|g| g.sqrt() as f32).collect()
}

/// Replaces a binary coverage map with an AE-style feathered matte: alpha
/// ramps linearly across `feather` pixels centered on the polygon boundary.
/// Distances are capped so uniform regions clamp cleanly (no INF−INF NaN).
fn feather_coverage(cov: &[f32], width: u32, height: u32, feather: f32) -> Vec<f32> {
    let bin: Vec<bool> = cov.iter().map(|&c| c > 0.5).collect();
    let d_to_uncovered = edt_2d(&bin, width, height, false); // inside → edge
    let d_to_covered = edt_2d(&bin, width, height, true); // outside → edge
    let cap = (width + height) as f32;
    let half = feather.max(0.5);
    cov.iter()
        .zip(d_to_uncovered)
        .zip(d_to_covered)
        .map(|((_, di), do_)| {
            let di = di.min(cap);
            let dout = do_.min(cap);
            let signed = di - dout; // >0 inside the shape
            ((signed / half) * 0.5 + 0.5).clamp(0.0, 1.0)
        })
        .collect()
}

/// Combines evaluated mask shapes into a single coverage buffer using AE's
/// mask-mode semantics. A Subtract FIRST mask starts from a full frame so it
/// carves material away (subtraction against an empty base is a no-op);
/// an inverted Add already carries its complement in its own coverage.
pub(crate) fn combine_mask_shapes(
    shapes: &[MaskShape],
    width: u32,
    height: u32,
) -> Option<Vec<f32>> {
    use crate::core::mask::MaskMode;
    let n = (width as usize) * (height as usize);
    if n == 0 {
        return None;
    }
    let mut acc = vec![0.0f32; n];
    let mut any = false;
    let mut first = true;
    for shape in shapes {
        if shape.mode == MaskMode::None || shape.poly.len() < 3 {
            continue;
        }
        let mut cov = rasterize_polygon_evenodd(&shape.poly, width, height);
        if shape.feather >= 0.5 {
            cov = feather_coverage(&cov, width, height, shape.feather);
        }
        let op = shape.opacity.clamp(0.0, 1.0);
        if op < 1.0 {
            for c in cov.iter_mut() {
                *c *= op;
            }
        }
        if shape.inverted {
            for c in cov.iter_mut() {
                *c = 1.0 - *c;
            }
        }
        if first && shape.mode == MaskMode::Subtract {
            acc.iter_mut().for_each(|a| *a = 1.0);
        }
        for (a, b) in acc.iter_mut().zip(cov.iter()) {
            *a = match shape.mode {
                MaskMode::Add | MaskMode::Lighten => *a + (*b * (1.0 - *a)),
                MaskMode::Subtract => *a * (1.0 - *b),
                MaskMode::Intersect | MaskMode::Darken => (*a).min(*b),
                MaskMode::Difference => (*a - *b).abs(),
                MaskMode::None => *a,
            };
        }
        any = true;
        first = false;
    }
    if !any {
        return None;
    }
    Some(acc)
}

/// Evaluates and rasterizes all enabled masks of `layer` at `frame` into an
/// output-sized RGBA8 coverage texture payload. Returns None when the layer
/// has no effective masks.
/// Evaluates enabled masks into scaled shapes and returns the input cache key.
/// None when the layer has no effective masks this frame.
pub(crate) fn collect_mask_shapes(
    layer: &crate::core::timeline::Layer,
    frame: u32,
    out_w: u32,
    out_h: u32,
    comp_w: u32,
    comp_h: u32,
) -> Option<(u64, Vec<MaskShape>)> {
    use crate::core::mask::MaskMode;
    if layer.masks.is_empty() || out_w == 0 || out_h == 0 {
        return None;
    }
    let sx = out_w as f32 / comp_w.max(1) as f32;
    let sy = out_h as f32 / comp_h.max(1) as f32;
    let mut shapes = Vec::new();
    for mask in &layer.masks {
        if !mask.enabled || mask.mode == MaskMode::None {
            continue;
        }
        let mut poly = mask.path.to_polygon(frame, 12);
        // to_polygon may repeat the first point to close the loop — drop it
        if poly.len() > 1 && poly[0] == *poly.last().unwrap() {
            poly.pop();
        }
        if poly.len() < 3 {
            continue;
        }
        shapes.push(MaskShape {
            poly: poly.iter().map(|p| [p[0] * sx, p[1] * sy]).collect(),
            mode: mask.mode,
            opacity: (mask.opacity.evaluate(frame) / 100.0).clamp(0.0, 1.0),
            inverted: mask.inverted,
            feather: mask.feather.evaluate(frame).max(0.0),
        });
    }
    if shapes.is_empty() {
        return None;
    }
    Some((mask_input_key(&shapes, out_w, out_h), shapes))
}

/// Combines + packs previously collected shapes into the uploadable raster.
/// `key` must come from [`collect_mask_shapes`] for the same inputs.
pub(crate) fn rasterize_from_shapes(
    shapes: &[MaskShape],
    out_w: u32,
    out_h: u32,
) -> Option<MaskRaster> {
    let coverage = combine_mask_shapes(shapes, out_w, out_h)?;
    let mut pixels = vec![0u8; coverage.len() * 4];
    for (i, c) in coverage.iter().enumerate() {
        let a = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        pixels[i * 4] = 255;
        pixels[i * 4 + 1] = 255;
        pixels[i * 4 + 2] = 255;
        pixels[i * 4 + 3] = a;
    }
    Some(MaskRaster {
        key: mask_input_key(shapes, out_w, out_h),
        pixels,
    })
}

#[cfg(test)]
mod gpu_mask_tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<[f32; 2]> {
        vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
    }

    #[test]
    fn evenodd_fill_rect_interior_only() {
        let cov = rasterize_polygon_evenodd(&rect(2.0, 2.0, 4.0, 4.0), 8, 8);
        assert_eq!(cov.len(), 64);
        let get = |px: usize, py: usize| cov[py * 8 + px];
        assert_eq!(get(3, 3), 1.0, "interior must be covered");
        assert_eq!(get(0, 0), 0.0, "outside must be uncovered");
        assert_eq!(get(7, 7), 0.0, "far corner must be uncovered");
    }

    #[test]
    fn degenerate_polygon_yields_no_coverage() {
        let cov = rasterize_polygon_evenodd(&[[0.0, 0.0], [5.0, 5.0]], 8, 8);
        assert!(cov.iter().all(|&c| c == 0.0));
    }

    #[test]
    fn add_mode_unions_adjacent_rects() {
        let shapes = [
            MaskShape { poly: rect(0.0, 0.0, 4.0, 2.0), mode: crate::core::mask::MaskMode::Add, opacity: 1.0, inverted: false, feather: 0.0 },
            MaskShape { poly: rect(4.0, 0.0, 4.0, 2.0), mode: crate::core::mask::MaskMode::Add, opacity: 1.0, inverted: false, feather: 0.0 },
        ];
        let cov = combine_mask_shapes(&shapes, 8, 2).expect("masks present");
        assert!(cov.iter().all(|&c| c == 1.0), "union of both halves covers the row");
    }

    #[test]
    fn subtract_first_mask_carves_full_frame() {
        let shapes = [MaskShape {
            poly: rect(2.0, 2.0, 4.0, 4.0),
            mode: crate::core::mask::MaskMode::Subtract,
            opacity: 1.0,
            inverted: false,
            feather: 0.0,
        }];
        let cov = combine_mask_shapes(&shapes, 8, 8).expect("masks present");
        let get = |px: usize, py: usize| cov[py * 8 + px];
        assert_eq!(get(3, 3), 0.0, "carved hole must be transparent");
        assert_eq!(get(0, 0), 1.0, "frame outside the hole stays opaque");
    }

    #[test]
    fn inverted_first_add_reveals_outside_only() {
        let shapes = [MaskShape {
            poly: rect(2.0, 2.0, 4.0, 4.0),
            mode: crate::core::mask::MaskMode::Add,
            opacity: 1.0,
            inverted: true,
            feather: 0.0,
        }];
        let cov = combine_mask_shapes(&shapes, 8, 8).expect("masks present");
        let get = |px: usize, py: usize| cov[py * 8 + px];
        assert_eq!(get(3, 3), 0.0);
        assert_eq!(get(0, 0), 1.0);
    }

    #[test]
    fn intersect_mode_takes_minimum() {
        let shapes = [
            MaskShape { poly: rect(0.0, 0.0, 6.0, 2.0), mode: crate::core::mask::MaskMode::Add, opacity: 1.0, inverted: false, feather: 0.0 },
            MaskShape { poly: rect(2.0, 0.0, 6.0, 2.0), mode: crate::core::mask::MaskMode::Intersect, opacity: 1.0, inverted: false, feather: 0.0 },
        ];
        let cov = combine_mask_shapes(&shapes, 8, 2).expect("masks present");
        assert_eq!(cov[1], 0.0, "only in first");
        assert_eq!(cov[4], 1.0, "in both");
        assert_eq!(cov[7], 0.0, "only in second");
    }

    #[test]
    fn none_and_empty_masks_return_none() {
        assert!(combine_mask_shapes(&[], 8, 8).is_none());
        let disabled = [MaskShape {
            poly: rect(0.0, 0.0, 4.0, 4.0),
            mode: crate::core::mask::MaskMode::None,
            opacity: 1.0,
            inverted: false,
            feather: 0.0,
        }];
        assert!(combine_mask_shapes(&disabled, 8, 8).is_none());
    }

    #[test]
    fn raster_key_is_deterministic_input_hash() {
        let shape = |feather: f32, mode: crate::core::mask::MaskMode| MaskShape {
            poly: rect(1.0, 1.0, 4.0, 4.0),
            mode,
            opacity: 1.0,
            inverted: false,
            feather,
        };
        let a = mask_input_key(&[shape(0.0, crate::core::mask::MaskMode::Add)], 8, 8);
        let b = mask_input_key(&[shape(0.0, crate::core::mask::MaskMode::Add)], 8, 8);
        assert_eq!(a, b, "identical inputs must hash identically");
        assert_ne!(
            mask_input_key(&[shape(2.5, crate::core::mask::MaskMode::Add)], 8, 8),
            a,
            "feather participates in the key"
        );
        assert_ne!(
            mask_input_key(
                &[shape(0.0, crate::core::mask::MaskMode::Subtract)],
                8,
                8
            ),
            a,
            "mode participates in the key"
        );
        assert_ne!(mask_input_key(&[shape(0.0, crate::core::mask::MaskMode::Add)], 16, 8), a);
    }

    #[test]
    fn mask_raster_cache_fifo_evicts_oldest() {
        let mut cache = MaskRasterCache::default();
        let dummy = |k: u64| MaskRaster {
            key: k,
            pixels: vec![255, 255, 255, k as u8],
        };
        for k in 0..MaskRasterCache::CAP as u64 {
            cache.insert(std::sync::Arc::new(dummy(k)));
        }
        assert!(cache.get(0).is_some(), "all six fit within capacity");
        // Insert one more → oldest (key 0) evicted, newest present.
        cache.insert(std::sync::Arc::new(dummy(MaskRasterCache::CAP as u64)));
        assert!(cache.get(0).is_none(), "FIFO must drop the oldest");
        assert!(cache.get(MaskRasterCache::CAP as u64).is_some());
    }

    #[test]
    fn edt_distance_matches_manhattan_reference() {
        // 4x4 grid with a single covered pixel at (1,1): Euclidean distance to
        // it must beat or equal any Manhattan path and match exact diagonals.
        let bin = vec![
            false, false, false, false,
            false, true,  false, false,
            false, false, false, false,
            false, false, false, false,
        ];
        let d = edt_2d(&bin, 4, 4, true);
        assert_eq!(d[4 + 1], 0.0);
        assert_eq!(d[4 + 2], 1.0);
        assert!((d[2 * 4 + 2] - std::f32::consts::SQRT_2).abs() < 1e-5);
        assert!((d[3 * 4 + 3] - 8.0f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn feather_zero_keeps_hard_edge() {
        let shapes = [MaskShape {
            poly: rect(2.0, 2.0, 4.0, 4.0),
            mode: crate::core::mask::MaskMode::Add,
            opacity: 1.0,
            inverted: false,
            feather: 0.0,
        }];
        let out = combine_mask_shapes(&shapes, 8, 8).expect("mask present");
        let raw = rasterize_polygon_evenodd(&rect(2.0, 2.0, 4.0, 4.0), 8, 8);
        for (a, b) in raw.iter().zip(out.iter()) {
            assert_eq!(a, b, "zero feather must keep binary coverage intact");
        }
    }

    #[test]
    fn feather_creates_ramp_centered_on_boundary() {
        // Rect with > feather/2 margins on every side so the probe saturates.
        let poly = rect(4.0, 4.0, 14.0, 8.0);
        let shapes = [MaskShape {
            poly,
            mode: crate::core::mask::MaskMode::Add,
            opacity: 1.0,
            inverted: false,
            feather: 4.0,
        }];
        let out = combine_mask_shapes(&shapes, 24, 16).expect("mask present");
        let get = |px: usize, py: usize| out[py * 24 + px];
        assert_eq!(get(11, 8), 1.0, "feather/2 inside saturates opaque");
        assert_eq!(get(23, 8), 0.0, "far outside stays transparent");
        // Right boundary sits between columns 17 and 18: both land mid-ramp.
        assert!(get(17, 8) > 0.0 && get(17, 8) < 1.0, "inner edge pixel");
        assert!(get(18, 8) > 0.0 && get(18, 8) < 1.0, "outer edge pixel");
        assert!((get(17, 8) - get(18, 8)).abs() < 0.35, "ramp symmetric around edge");
    }
}

/// End-to-end parity checks between the CPU reference renderer's mask
/// handling and this file's coverage rasterizer (which feeds the GPU path).
///
/// Known CPU divergences (documented, not asserted here): the software
/// renderer honors only the FIRST enabled non-None mask and ignores
/// mode-based combining, per-mask opacity, expansion, and wiggle — the GPU
/// rasterizer implements the full superset.
#[cfg(test)]
mod cpu_parity_tests {
    use super::*;
    use crate::core::mask::Mask;
    use crate::core::timeline::{Composition, Layer, LayerType};

    const W: u32 = 64;
    const H: u32 = 64;

    fn solid_with_rect_mask(mode: crate::core::mask::MaskMode, inverted: bool) -> Composition {
        let mut comp = Composition::new("c".into(), "Parity".into(), W, H, 30, 30);
        comp.background_color = [0.0, 0.0, 0.0, 1.0];
        let mut layer = Layer::new(
            "l".into(),
            "Solid".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            30,
        );
        layer.transform.position =
            crate::core::property::Animatable::new_constant([32.0, 32.0]);
        let mut mask = Mask::new_rect("m".into(), "M".into(), 8.0, 8.0, 24.0, 40.0);
        mask.mode = mode;
        mask.inverted = inverted;
        layer.masks.push(mask);
        comp.layers.push(layer);
        comp
    }

    /// Renders the comp through the CPU reference and through our coverage
    /// rasterizer, then asserts both agree at an interior and an exterior
    /// probe pixel.
    fn assert_cpu_and_gpu_agree(inverted: bool) {
        use crate::core::mask::MaskMode;
        let comp = solid_with_rect_mask(MaskMode::Add, inverted);
        let cpu =
            crate::core::software_renderer::render_frame_to_pixels(&comp, 0, W, H, 0.0, 0);

        let (key, shapes) =
            collect_mask_shapes(&comp.layers[0], 0, W, H, W, H).expect("mask present");
        let raster = rasterize_from_shapes(&shapes, W, H).expect("coverage");
        assert_eq!(raster.key, key, "key must derive from the same inputs");

        // Rect spans x[8..32) y[8..48): interior (20,32), exterior (52,32).
        for &(px, py, inside_polygon) in
            &[(20usize, 32usize, true), (52usize, 32usize, false)]
        {
            let i = (py * W as usize + px) * 4;
            let cpu_red = cpu[i];
            let my_alpha = raster.pixels[i + 3];
            let expect_white = inside_polygon != inverted;
            if expect_white {
                assert!(
                    cpu_red >= 250,
                    "CPU should show the solid at ({},{}), red={}",
                    px,
                    py,
                    cpu_red
                );
                assert_eq!(my_alpha, 255, "GPU coverage opaque at ({},{})", px, py);
            } else {
                assert!(
                    cpu_red <= 8,
                    "CPU should show background at ({},{}), red={}",
                    px,
                    py,
                    cpu_red
                );
                assert_eq!(my_alpha, 0, "GPU coverage clear at ({},{})", px, py);
            }
        }
    }

    #[test]
    fn add_mask_matches_cpu_reference() {
        assert_cpu_and_gpu_agree(false);
    }

    #[test]
    fn inverted_add_mask_matches_cpu_reference() {
        assert_cpu_and_gpu_agree(true);
    }
}

/// Bakes a text stroke into a rasterized text bitmap: dilates the fill alpha by the
/// stroke radius, colors it with stroke_color, and composites it behind the fill.
/// Returns padded (width, height, pixels) so the stroke is not clipped.
fn bake_text_stroke(
    pixels: &[u8],
    width: u32,
    height: u32,
    stroke_color: [f32; 4],
    stroke_width: f32,
) -> (u32, u32, Vec<u8>) {
    let radius = (stroke_width * 0.5).ceil().max(1.0) as i32;
    let pad = radius + 1;
    let (nw, nh) = (width + (pad * 2) as u32, height + (pad * 2) as u32);
    let mut out = vec![0u8; (nw * nh * 4) as usize];

    let sr = (stroke_color[0].clamp(0.0, 1.0) * 255.0) as u8;
    let sg = (stroke_color[1].clamp(0.0, 1.0) * 255.0) as u8;
    let sb = (stroke_color[2].clamp(0.0, 1.0) * 255.0) as u8;
    let stroke_a = stroke_color[3];

    let w = width as i32;
    let h = height as i32;
    for py in 0..nh as i32 {
        for px in 0..nw as i32 {
            let tx = px - pad;
            let ty = py - pad;
            let oidx = ((py as u32 * nw + px as u32) * 4) as usize;

            // Fill sample (offset back by pad)
            let fill_alpha = if tx >= 0 && ty >= 0 && tx < w && ty < h {
                let idx = ((ty * w + tx) * 4) as usize;
                pixels[idx + 3] as f32 / 255.0
            } else {
                0.0
            };

            // Stroke: max over neighbors within radius of fill alpha, feathered by distance
            let mut stroke_alpha = 0.0f32;
            if fill_alpha < 0.999 {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = tx + dx;
                        let ny = ty + dy;
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            let dist = ((dx * dx + dy * dy) as f32).sqrt();
                            if dist <= stroke_width * 0.5 {
                                let nidx = ((ny * w + nx) * 4) as usize;
                                let n_alpha = pixels[nidx + 3] as f32 / 255.0;
                                if n_alpha > 0.001 {
                                    let edge = (stroke_width * 0.5 - dist) / (stroke_width * 0.25).max(0.5);
                                    stroke_alpha = stroke_alpha.max(edge.clamp(0.0, 1.0));
                                }
                            }
                        }
                    }
                }
            }

            // Composite stroke behind fill (premultiplied-style over)
            let stroke_a_px = stroke_alpha * stroke_a;
            let out_a = fill_alpha + stroke_a_px * (1.0 - fill_alpha);
            if out_a > 0.001 {
                let fr = pixels.get(((ty.max(0) * w + tx.max(0)) * 4) as usize).copied().unwrap_or(0);
                let fg = pixels.get(((ty.max(0) * w + tx.max(0)) * 4 + 1) as usize).copied().unwrap_or(0);
                let fb = pixels.get(((ty.max(0) * w + tx.max(0)) * 4 + 2) as usize).copied().unwrap_or(0);
                // Stroke color behind fill color
                let mix_r = (sr as f32 * (1.0 - fill_alpha) + fr as f32 * fill_alpha) as u8;
                let mix_g = (sg as f32 * (1.0 - fill_alpha) + fg as f32 * fill_alpha) as u8;
                let mix_b = (sb as f32 * (1.0 - fill_alpha) + fb as f32 * fill_alpha) as u8;
                out[oidx] = mix_r;
                out[oidx + 1] = mix_g;
                out[oidx + 2] = mix_b;
                out[oidx + 3] = (out_a * 255.0) as u8;
            }
        }
    }
    (nw, nh, out)
}

struct TextRasterParams {
    text: String,
    font_size: u32,
    color: [f32; 4],
    font_family: String,
    tracking: f32,
    leading: f32,
    align: usize,
    stroke_color: [f32; 4],
    stroke_width: f32,
}

type RenderKey = (u64, u32, u32, u32, (u32, u32));
type TextTextureKey = (String, String, u32, [u32; 4], u32);
type TextTextureCache = std::collections::HashMap<TextTextureKey, (wgpu::Texture, std::sync::Arc<wgpu::BindGroup>, u32, u32)>;
type VideoFrameKey = (String, String, u32);
type VideoFrameCache = std::collections::HashMap<VideoFrameKey, (std::sync::Arc<wgpu::Texture>, std::sync::Arc<wgpu::BindGroup>)>;

/// Maximum cached video frame textures before oldest entries are evicted.
/// 200 frames at 1080p RGBA is ~830 MB of VRAM; evicted frames re-upload
/// cheaply on demand, so this only bounds memory, not correctness.
pub const MAX_VIDEO_FRAME_TEXTURES: usize = 200;

#[allow(dead_code)]
pub struct WgpuRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,

    layer_buffer: wgpu::Buffer,
    layer_bind_group: wgpu::BindGroup,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    dummy_texture_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,

    // Shared layer-mask resources (group 3)
    mask_bind_group_layout: wgpu::BindGroupLayout,
    mask_bind_group: wgpu::BindGroup,
    mask_texture: Option<wgpu::Texture>,
    mask_view: Option<wgpu::TextureView>,
    mask_size: (u32, u32),
    /// Content-addressed cache so static masks skip EDT re-raster during
    /// playback of other layers.
    mask_raster_cache: std::cell::RefCell<MaskRasterCache>,

    // Target offscreen texture
    pub target_texture: Option<wgpu::Texture>,
    pub target_view: Option<wgpu::TextureView>,
    pub target_size: (u32, u32),

    // Snapshot target offscreen texture
    pub snapshot_texture: Option<wgpu::Texture>,
    pub snapshot_view: Option<wgpu::TextureView>,

    // Dirty-checking: skip re-render when inputs are unchanged.
    // Keyed by (version, frame, exposure bits, lut, target size) per target type.
    last_main_key: Option<RenderKey>,
    last_snapshot_key: Option<RenderKey>,

    // ── RAM preview ring ──
    // Pre-rendered frames for smooth playback, produced by render_ram_preview.
    // Invalidated wholesale when the project version changes.
    ram_ring: Vec<(u32, Option<(wgpu::Texture, wgpu::TextureView)>)>,
    ram_ring_version: u64,
    /// Index into ram_ring used while rendering a pre-pass frame.
    ram_render_idx: usize,

    /// Optional cap on preview render width (px). When set, large compositions
    /// are rendered at a downscaled resolution — the viewport samples the
    /// texture at display size anyway, so this is visually near-free and can
    /// cut fill-rate by 4-16x on 4K comps.
    preview_max_width: Option<u32>,

    // GPU text rendering: cache of CPU-rasterized text textures keyed by (layer_id, text, font_size)
    text_texture_cache: std::cell::RefCell<TextTextureCache>,
    video_frame_cache: std::cell::RefCell<VideoFrameCache>,
    video_cache_version: std::cell::Cell<u64>,
}

impl WgpuRenderer {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Shaders compile
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Renderer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Buffers
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals Buffer"),
            size: std::mem::size_of::<GlobalsUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layer_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Layer Buffer"),
            size: (std::mem::size_of::<LayerUniform>() * 256) as u64, // Pre-allocate up to 256 layers
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind Group Layouts
        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("globals_bind_group_layout"),
            });

        let layer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true, // Enable dynamic uniform offsets
                        min_binding_size: Some(LAYER_UNIFORM_SIZE),
                    },
                    count: None,
                }],
                label: Some("layer_bind_group_layout"),
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // Diffuse texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // Diffuse sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let mask_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("mask_bind_group_layout"),
            });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
            label: Some("globals_bind_group"),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create dummy mask texture for default binds
        let dummy_mask_size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let dummy_mask_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Dummy Mask Texture"),
            size: dummy_mask_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &dummy_mask_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            dummy_mask_size,
        );
        let dummy_mask_view = dummy_mask_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mask_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &mask_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&dummy_mask_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("mask_bind_group"),
        });

        let layer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &layer_buffer,
                    offset: 0,
                    size: Some(LAYER_UNIFORM_SIZE),
                }),
            }],
            label: Some("layer_bind_group"),
        });

        // Create dummy texture for default binds
        let dummy_size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Dummy Texture"),
            size: dummy_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &dummy_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            dummy_size,
        );
        let dummy_texture_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let dummy_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&dummy_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&dummy_mask_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("dummy_texture_bind_group"),
        });

        // Pipeline Layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &globals_bind_group_layout,
                    &layer_bind_group_layout,
                    &texture_bind_group_layout,
                    &mask_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // Pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            device,
            queue,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            globals_buffer,
            globals_bind_group,
            layer_buffer,
            layer_bind_group,
            texture_bind_group_layout,
            dummy_texture_bind_group,
            sampler,
            mask_bind_group_layout,
            mask_bind_group,
            mask_texture: None,
            mask_view: None,
            mask_size: (0, 0),
            mask_raster_cache: std::cell::RefCell::new(MaskRasterCache::default()),
            target_texture: None,
            target_view: None,
            target_size: (0, 0),
            snapshot_texture: None,
            snapshot_view: None,
            text_texture_cache: std::cell::RefCell::new(std::collections::HashMap::new()),
            video_frame_cache: std::cell::RefCell::new(std::collections::HashMap::new()),
            video_cache_version: std::cell::Cell::new(0),
            last_main_key: None,
            last_snapshot_key: None,
            ram_ring: Vec::new(),
            ram_ring_version: 0,
            ram_render_idx: usize::MAX,
            preview_max_width: None,
        }
    }

    /// Creates/replaces the shared layer-mask texture when its size changed.
    fn ensure_mask_texture(&mut self, width: u32, height: u32) {
        if self.mask_size == (width, height) && self.mask_texture.is_some() {
            return;
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Layer Mask Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.mask_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some("mask_bind_group_live"),
        });
        self.mask_texture = Some(texture);
        self.mask_view = Some(view);
        self.mask_bind_group = bind_group;
        self.mask_size = (width, height);
    }

    /// Prepares/resizes the offscreen target texture if needed.
    /// Returns true if the texture was recreated.
    /// Rasterizes text on CPU and uploads it as a GPU texture, cached by (layer_id, text, font_size).
    /// Returns (width, height, bind_group) for the text texture, or None if rasterization fails.
    fn get_or_create_text_texture(
        &self,
        layer_id: &str,
        params: &TextRasterParams,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let (text, font_size, color, font_family, tracking, leading, align) =
            (params.text.as_str(), params.font_size, params.color, params.font_family.as_str(), params.tracking, params.leading, params.align);
        let (stroke_color, stroke_width) = (params.stroke_color, params.stroke_width);
        // Floats hashed via bit patterns (f32 is not Hash)
        let key = (
            layer_id.to_string(),
            text.to_string(),
            font_size,
            [stroke_color[0].to_bits(), stroke_color[1].to_bits(), stroke_color[2].to_bits(), stroke_color[3].to_bits()],
            stroke_width.to_bits(),
        );
        // Cached: return stored dimensions — no CPU rasterization on hits
        if let Some(bind_group) = self
            .text_texture_cache
            .borrow()
            .get(&key)
            .map(|(_, bg, w, h)| (bg.clone(), *w, *h))
        {
            let (bind_group, w, h) = bind_group;
            return Some((w, h, bind_group));
        }

        let alignment = match align {
            1 => crate::core::text_layout::TextAlign::Center,
            2 => crate::core::text_layout::TextAlign::Right,
            _ => crate::core::text_layout::TextAlign::Left,
        };
        let rasterized = crate::core::font_rasterizer::with_font_rasterizer(|r| {
            let family = r.resolve_family(font_family);
            r.rasterize_text_formatted(&family, text, font_size as f32, color, tracking, leading, 0.0, alignment)
        })?;
        if rasterized.0 == 0 || rasterized.1 == 0 || rasterized.2.is_empty() {
            return None;
        }
        // Bake stroke (if any) into the bitmap with padding
        let (tw, th, pixels) = if stroke_width > 0.1 {
            bake_text_stroke(&rasterized.2, rasterized.0, rasterized.1, stroke_color, stroke_width)
        } else {
            rasterized
        };

        let size = wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Layer Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(tw * 4),
                rows_per_image: Some(th),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
            label: Some("text_texture_bind_group"),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        self.text_texture_cache
            .borrow_mut()
            .insert(key, (texture, bind_group.clone(), tw, th));
        Some((tw, th, bind_group))
    }

    pub fn ensure_target_size(&mut self, width: u32, height: u32) -> bool {        if self.target_size == (width, height) && self.target_texture.is_some() {
            return false;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Renderer Offscreen Target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let snap_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Renderer Snapshot Target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let snap_view = snap_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.target_texture = Some(texture);
        self.target_view = Some(view);
        self.snapshot_texture = Some(snap_texture);
        self.snapshot_view = Some(snap_view);
        self.target_size = (width, height);
        true
    }

    /// Internal core rendering implementation for both primary preview and snapshot target views.
    fn render_internal(&mut self, comp: &Composition, frame: u32, exposure_ev: f32, lut_mode: u32, target_snapshot: bool) -> bool {
        // Dirty-checking: the viewport calls render() at display refresh rate even
        // when nothing changed. Skip the full encode/upload/draw pass when the
        // project version, frame, exposure, LUT, and target size are unchanged.
        // Effective preview resolution: downscale large comps to the viewport cap.
        let (eff_w, eff_h) = match self.preview_max_width {
            Some(cap) if comp.width > cap => {
                let s = cap as f32 / comp.width as f32;
                (
                    cap.max(1),
                    ((comp.height as f32 * s) as u32).max(1),
                )
            }
            _ => (comp.width, comp.height),
        };

        let render_key: RenderKey = (
            crate::core::frame_cache::current_version(),
            frame,
            exposure_ev.to_bits(),
            lut_mode,
            (eff_w, eff_h),
        );
        let last_key = if target_snapshot { &self.last_snapshot_key } else { &self.last_main_key };
        let ram_mode = self.ram_render_idx != usize::MAX;
        if !ram_mode && *last_key == Some(render_key) {
            return false; // nothing changed — reuse the existing target texture
        }

        // Clamp to both our sanity limit and the device's texture limit —
        // oversized textures would trip wgpu validation and abort the process.
        let max_dim = self.device.limits().max_texture_dimension_2d.min(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        let width = eff_w.clamp(1, max_dim);
        let height = eff_h.clamp(1, max_dim);
        let recreated = self.ensure_target_size(width, height);

        // Update Globals Uniform
        let globals = GlobalsUniform {
            viewport_size: [width as f32, height as f32],
            exposure_ev,
            lut_mode,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Per-layer text texture bind groups (declared before the render pass so they outlive it)
        let mut layer_textures: Vec<Option<std::sync::Arc<wgpu::BindGroup>>> = Vec::new();

        // Target must exist; per-run views are borrowed fresh inside the Step-3
        // loop so mask texture uploads (&mut self) fit between submits.
        if (target_snapshot && self.snapshot_view.is_none())
            || (!target_snapshot && self.target_view.is_none())
        {
            return false;
        }

        {

            // Viewport projection matrix:
            // Maps [0, width] to [-1, 1] on X, and [0, height] to [1, -1] on Y.
            let m_proj = [
                [2.0 / width as f32, 0.0, 0.0, 0.0],
                [0.0, -2.0 / height as f32, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0, 1.0], // Column-major translation: -1 on x, 1 on y
            ];

            // Step 1: Pre-evaluate active layer transform matrices and effect properties
            let mut active_layers = Vec::new();
            let mut uniforms = Vec::new();
            let mut layer_mask_plans: Vec<Option<std::sync::Arc<MaskRaster>>> = Vec::new();

            for layer in &comp.layers {
                if !layer.is_active(frame) {
                    continue;
                }

                // Retrieve transform values at the current frame.
                // Parented layers and expression-driven layers resolve through the full
                // composition-aware path (position/scale/rotation/opacity inherit).
                let layer_has_exprs = layer.transform.position_expression.is_some()
                    || layer.transform.rotation_expression.is_some()
                    || layer.transform.scale_expression.is_some()
                    || layer.transform.opacity_expression.is_some();
                let (pos, scale, rotation, opacity) = if layer.parent_id.is_some() || layer_has_exprs {
                    let (p, s, r, o) = comp.resolve_world_transform(layer, frame);
                    (p, s, r, o / 100.0)
                } else {
                    (
                        layer.transform.position.evaluate(frame),
                        layer.transform.scale.evaluate(frame),
                        layer.transform.rotation.evaluate(frame),
                        layer.transform.opacity.evaluate(frame),
                    )
                };

                // Default layer dimensions (solid size or fallback)
                let (mut layer_w, mut layer_h) = match &layer.layer_type {
                    LayerType::Solid { .. } => (1.0, 1.0),
                    LayerType::Image { .. } => (1.0, 1.0),
                    LayerType::Video { .. } => (1.0, 1.0),
                    LayerType::Text { font_size, .. } => (1.0, *font_size as f32 * 10.0), // Overridden below if text texture rasterization succeeds
                    LayerType::Shape { .. } => (1.0, 1.0),
                    LayerType::Null => (0.0, 0.0),
                    LayerType::PreComp { .. } => (comp.width as f32, comp.height as f32),
                    LayerType::AdjustmentLayer => (comp.width as f32, comp.height as f32),
                    LayerType::Audio { .. } => (0.0, 0.0),
                    LayerType::Particle { .. } => (comp.width as f32, comp.height as f32),
                };

                // GPU text rendering: rasterize text to a cached texture; if successful,
                // size the quad to the text bitmap and render via the image sampling path.
                let mut text_bind_group: Option<std::sync::Arc<wgpu::BindGroup>> = None;
                let mut is_textured_text = false;
                // Video layers sample their frame sequence via the image path too,
                // but keep composition-sized geometry (unlike text, which fits the bitmap).
                let mut is_video_frame = false;
                if let LayerType::Video { frames_dir, frame_count, .. } = &layer.layer_type {
                    let seq_frame = frame.min(frame_count.saturating_sub(1));
                    if let Some((_, _, bg)) =
                        self.get_or_create_video_frame_texture(&layer.id, frames_dir, seq_frame, crate::core::frame_cache::current_version())
                    {
                        text_bind_group = Some(bg);
                        is_video_frame = true;
                    }
                }
                if let LayerType::Text { text, font_size, color, font_family, tracking, leading, align, stroke_color, stroke_width, .. } = &layer.layer_type {
                    let params = TextRasterParams {
                        text: text.clone(), font_size: *font_size, color: *color,
                        font_family: font_family.clone(), tracking: *tracking, leading: *leading, align: *align,
                        stroke_color: *stroke_color, stroke_width: *stroke_width,
                    };
                    if let Some((tw, th, bg)) = self.get_or_create_text_texture(&layer.id, &params) {
                        layer_w = tw as f32;
                        layer_h = th as f32;
                        text_bind_group = Some(bg);
                        is_textured_text = true;
                    }
                }
                layer_textures.push(text_bind_group);


                let anc = layer.transform.anchor_point.evaluate(frame);

                // Compute layer-to-world transformation matrix
                let m_size = [
                    [layer_w, 0.0, 0.0, 0.0],
                    [0.0, layer_h, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [layer_w * 0.5, layer_h * 0.5, 0.0, 1.0],
                ];

                let m_anc = [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [-anc[0], -anc[1], 0.0, 1.0],
                ];

                let m_scale = [
                    [scale[0] / 100.0, 0.0, 0.0, 0.0],
                    [0.0, scale[1] / 100.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ];

                let rad = rotation.to_radians();
                let cos_r = rad.cos();
                let sin_r = rad.sin();
                let m_rot = [
                    [cos_r, sin_r, 0.0, 0.0],
                    [-sin_r, cos_r, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ];

                let m_pos = [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [pos[0], pos[1], 0.0, 1.0],
                ];

                let m_model = mat4_mul(
                    m_pos,
                    mat4_mul(m_rot, mat4_mul(m_scale, mat4_mul(m_anc, m_size))),
                );

                // Total projection * model matrix
                let transform_matrix = if layer.is_3d {
                    comp.resolve_world_transform_3d(layer, frame)
                } else {
                    mat4_mul(m_proj, m_model)
                };

                // Prepare Layer Uniform details
                let (mut layer_type, shape_type, mut color) = match &layer.layer_type {
                    LayerType::Solid { color } => (0u32, 0u32, *color),
                    LayerType::Image { .. } | LayerType::Video { .. } => (1u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                    LayerType::Shape { shape_type, color, .. } => {
                        let st = match shape_type {
                            ShapeType::Rectangle { .. } => 0u32,
                            ShapeType::Ellipse { .. } => 1u32,
                            ShapeType::Star { .. } => 2u32,
                            ShapeType::Polygon { .. } => 3u32,
                        };
                        (2u32, st, *color)
                    }
                    LayerType::Text { color, .. } => (3u32, 0u32, *color),                    LayerType::Null => (4u32, 0u32, [0.0, 0.0, 0.0, 0.0]),
                    LayerType::PreComp { .. } => (5u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                    LayerType::AdjustmentLayer => (7u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                    LayerType::Audio { .. } => (6u32, 0u32, [0.0, 0.0, 0.0, 0.0]),
                    LayerType::Particle { .. } => (8u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                };

                // Textured text uses the image sampling path with unmodified texture colors
                if is_textured_text || is_video_frame {
                    layer_type = 1u32;
                    color = [1.0, 1.0, 1.0, 1.0];
                }


                let ep = evaluate_effects(&layer.effects, frame);

                // Shape parameters for GPU SDFs: polygon/star point count, rectangle corner radius
                let shape_params_eval: [f32; 4] = match &layer.layer_type {
                    LayerType::Shape { shape_type, .. } => match shape_type {
                        ShapeType::Polygon { sides, .. } => [sides.evaluate(frame), 0.0, 0.0, 0.0],
                        ShapeType::Star { points, .. } => [points.evaluate(frame), 0.0, 0.0, 0.0],
                        ShapeType::Rectangle { corner_radius, width, height, .. } => {
                            let cr = corner_radius.evaluate(frame);
                            let w = width.evaluate(frame).max(1.0);
                            let h = height.evaluate(frame).max(1.0);
                            // Normalize corner radius to 0..0.5 of the smaller half-size
                            [0.0, (cr / w.min(h)).clamp(0.0, 0.5), 0.0, 0.0]
                        }
                        _ => [0.0; 4],
                    },
                    _ => [0.0; 4],
                };

                let layer_uniform = LayerUniform {
                    transform_matrix,
                    color,
                    opacity,
                    layer_type,
                    shape_type,
                    effect_tint_enabled: ep.tint_enabled,
                    effect_tint_color: ep.tint_color,
                    effect_tint_intensity: ep.tint_intensity,
                    effect_blur_enabled: ep.blur_enabled,
                    effect_blur_radius: ep.blur_radius,
                    effect_shadow_enabled: ep.shadow_enabled,
                    effect_shadow_color: ep.shadow_color,
                    effect_shadow_opacity: ep.shadow_opacity,
                    effect_shadow_direction: ep.shadow_direction,
                    effect_shadow_distance: ep.shadow_distance,
                    effect_shadow_softness: ep.shadow_softness,
                    effect_ca_enabled: ep.chromatic_enabled,
                    effect_ca_shift_r: ep.chromatic_shift_r,
                    effect_ca_shift_b: ep.chromatic_shift_b,
                    effect_ca_edge_falloff: ep.chromatic_edge_falloff,
                    effect_vignette_enabled: ep.vignette_enabled,
                    effect_vignette_intensity: ep.vignette_intensity,
                    effect_vignette_roundness: ep.vignette_roundness,
                    effect_vignette_feather: ep.vignette_feather,
                    effect_vignette_color: ep.vignette_color,
                    blend_mode: match layer.blend_mode {
                        crate::core::timeline::BlendMode::Normal => 0,
                        crate::core::timeline::BlendMode::Multiply => 1,
                        crate::core::timeline::BlendMode::Screen => 2,
                        crate::core::timeline::BlendMode::Overlay => 3,
                        crate::core::timeline::BlendMode::Add => 4,
                        crate::core::timeline::BlendMode::Darken => 5,
                        crate::core::timeline::BlendMode::Lighten => 6,
                        crate::core::timeline::BlendMode::SoftLight => 7,
                        crate::core::timeline::BlendMode::HardLight => 8,
                        crate::core::timeline::BlendMode::Difference => 9,
                        crate::core::timeline::BlendMode::Exclusion => 10,
                        crate::core::timeline::BlendMode::Divide => 11,
                        crate::core::timeline::BlendMode::Subtract => 12,
                    },
                    levels_enabled: ep.levels_enabled,
                    levels_in_black: ep.levels_in_black,
                    levels_in_white: ep.levels_in_white,
                    levels_gamma: ep.levels_gamma,
                    levels_out_black: ep.levels_out_black,
                    levels_out_white: ep.levels_out_white,
                    huesat_enabled: ep.huesat_enabled,
                    huesat_hue: ep.huesat_hue,
                    huesat_sat: ep.huesat_sat,
                    huesat_light: ep.huesat_light,
                    glow_enabled: ep.glow_enabled,
                    glow_threshold: ep.glow_threshold,
                    glow_radius: ep.glow_radius,
                    glow_intensity: ep.glow_intensity,
                    glow_color: ep.glow_color,
                    grain_enabled: ep.grain_enabled,
                    grain_intensity: ep.grain_intensity,
                    grain_size: ep.grain_size,
                    shape_params: shape_params_eval,
                    track_matte_mode: match layer.track_matte {
                        crate::core::timeline::TrackMatteMode::None => 0,
                        crate::core::timeline::TrackMatteMode::AlphaMatte => 1,
                        crate::core::timeline::TrackMatteMode::AlphaMatteInverted => 2,
                        crate::core::timeline::TrackMatteMode::LumaMatte => 3,
                        crate::core::timeline::TrackMatteMode::LumaMatteInverted => 4,
                    },
                    meshwarp_enabled: ep.meshwarp_enabled,
                    corner_top_left: ep.corner_top_left,
                    corner_top_right: ep.corner_top_right,
                    corner_bottom_left: ep.corner_bottom_left,
                    corner_bottom_right: ep.corner_bottom_right,
                    mask_enabled: 0,
                    mask_mode: 0,
                    mask_inverted: 0,
                    mask_feather: 0.0,
                    _padding_align: [[0.0; 4]; 9],
                };

                layer_mask_plans.push(collect_mask_shapes(
                    layer, frame, width, height, comp.width, comp.height,
                )
                .map(|(key, shapes)| {
                    if let Some(hit) = self.mask_raster_cache.borrow_mut().get(key) {
                        return hit;
                    }
                    // Drop the short-lived borrow before the (possibly slow)
                    // raster so we never hold the cache across heavy work.
                    let raster = std::sync::Arc::new(
                        rasterize_from_shapes(&shapes, width, height)
                            .expect("non-empty shapes always yield coverage"),
                    );
                    self.mask_raster_cache.borrow_mut().insert(raster.clone());
                    raster
                }));
                uniforms.push(layer_uniform);
                active_layers.push(layer);
            }

            // Per-layer mask rasters were computed in Step 1. Every masked
            // layer gets mask_enabled=1; each distinct raster is uploaded in
            // its own submit during Step 3 (uploads execute just before their
            // own submit, so painter order across runs is preserved).
            for (u, plan) in uniforms.iter_mut().zip(layer_mask_plans.iter()) {
                let active = plan.is_some();
                u.mask_enabled = u32::from(active);
                u.mask_mode = u32::from(active); // 1 = alpha channel
                u.mask_inverted = 0; // inversion is baked per-mask into the raster
                u.mask_feather = 0.0; // feather is baked into the coverage ramp
            }

            // Step 2: Upload all Layer Uniforms in a single GPU command write
            if !uniforms.is_empty() {
                if uniforms.len() > 256 {
                    log::warn!(
                        "[WgpuRenderer] Active layer count ({}) exceeds 256 layer limit; extra layers will be truncated",
                        uniforms.len()
                    );
                }
                let upload_len = uniforms.len().min(256);
                self.queue.write_buffer(
                    &self.layer_buffer,
                    0,
                    bytemuck::cast_slice(&uniforms[0..upload_len]),
                );
            }

            // Step 3: One submit per contiguous mask-key run. Layers sharing a
            // raster draw together after that raster's upload; unmasked layers
            // batch freely. LoadOp::Load on later runs preserves earlier
            // draws, keeping bottom-up painter order across submits.
            #[derive(Clone, Copy)]
            struct MaskRun {
                start: usize,
                end: usize,
                key: Option<u64>,
            }
            let draw_count = active_layers.len().min(256);
            let mut runs: Vec<MaskRun> = Vec::new();
            for i in 0..draw_count {
                let k = layer_mask_plans.get(i).and_then(|p| p.as_ref()).map(|r| r.key);
                match runs.last_mut() {
                    Some(r) if r.key == k => r.end = i + 1,
                    _ => runs.push(MaskRun { start: i, end: i + 1, key: k }),
                }
            }
            if runs.is_empty() {
                // No layers this frame — still clear to the background color.
                runs.push(MaskRun { start: 0, end: 0, key: None });
            }

            for run in &runs {
                if let Some(_key) = run.key {
                    if let Some(raster) = layer_mask_plans[run.start].as_ref() {
                        self.ensure_mask_texture(width, height);
                        let size = wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        };
                        self.queue.write_texture(
                            wgpu::ImageCopyTexture {
                                texture: self.mask_texture.as_ref().expect("mask texture just ensured"),
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            &raster.pixels,
                            wgpu::ImageDataLayout {
                                offset: 0,
                                bytes_per_row: Some(width * 4),
                                rows_per_image: Some(height),
                            },
                            size,
                        );
                    }
                }

                let view = if target_snapshot {
                    self.snapshot_view.as_ref()
                } else {
                    self.target_view.as_ref()
                };
                let Some(view) = view else { return false };

                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some(if target_snapshot { "Snapshot Render Encoder" } else { "Render Encoder" }),
                    });

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some(if target_snapshot { "Snapshot Render Pass" } else { "Render Pass" }),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: if run.start == 0 {
                                    wgpu::LoadOp::Clear(wgpu::Color {
                                        r: comp.background_color[0].clamp(0.0, 1.0) as f64,
                                        g: comp.background_color[1].clamp(0.0, 1.0) as f64,
                                        b: comp.background_color[2].clamp(0.0, 1.0) as f64,
                                        a: comp.background_color[3].clamp(0.0, 1.0) as f64,
                                    })
                                } else {
                                    wgpu::LoadOp::Load
                                },
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    render_pass.set_pipeline(&self.render_pipeline);
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.set_bind_group(0, &self.globals_bind_group, &[]);

                    for i in run.start..run.end {
                        // Bind resources using dynamic uniform offset
                        let dynamic_offset = (i * std::mem::size_of::<LayerUniform>()) as u32;
                        render_pass.set_bind_group(1, &self.layer_bind_group, &[dynamic_offset]);

                        // Texture binding (use per-layer text texture when available, dummy for solid/SDF shapes)
                        let tex_bg: &wgpu::BindGroup = match layer_textures.get(i) {
                            Some(Some(bg)) => bg,
                            _ => &self.dummy_texture_bind_group,
                        };
                        render_pass.set_bind_group(2, tex_bg, &[]);
                        render_pass.set_bind_group(3, &self.mask_bind_group, &[]);

                        // Draw!
                        render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
                    }
                }

                self.queue.submit(std::iter::once(encoder.finish()));
            }
        }

        // Remember inputs so redundant renders can be skipped.
        // RAM pre-pass frames never touch the live-view keys.
        if !ram_mode {
            if target_snapshot {
                self.last_snapshot_key = Some(render_key);
            } else {
                self.last_main_key = Some(render_key);
            }
        }
        recreated
    }

    /// Pre-renders `from..=to` into the RAM preview ring. Called incrementally
    /// (a few frames per UI frame) so the UI stays responsive during the pass.
    ///
    /// `total_count` is the final ring size for this pre-pass; it is allocated on
    /// the first call and reused afterwards. Frames already in the ring are
    /// skipped, so repeated calls with advancing ranges are cheap.
    pub fn render_ram_preview_range(
        &mut self,
        comp: &Composition,
        from: u32,
        to: u32,
        exposure_ev: f32,
        lut_mode: u32,
        total_count: u32,
    ) {
        let ver = crate::core::frame_cache::current_version();
        if self.ram_ring_version != ver {
            self.ram_ring.clear();
            self.ram_ring_version = ver;
        }
        let count = total_count.max(1) as usize;
        if self.ram_ring.len() != count {
            self.ram_ring.clear();
            for _ in 0..count {
                self.ram_ring.push((u32::MAX, None));
            }
        }

        // Match the live-view resolution logic (preview cap + device limits)
        let max_dim = self.device.limits().max_texture_dimension_2d
            .min(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        let (eff_w, eff_h) = match self.preview_max_width {
            Some(cap) if comp.width > cap => {
                let s = cap as f32 / comp.width as f32;
                (cap.max(1), ((comp.height as f32 * s) as u32).max(1))
            }
            _ => (comp.width, comp.height),
        };
        let width = eff_w.clamp(1, max_dim);
        let height = eff_h.clamp(1, max_dim);

        for frame in from..=to {
            // Find or allocate this frame's slot (frames map to slots by order of
            // first appearance; the cursor advances monotonically).
            let slot_idx = match self.ram_ring.iter().position(|(f, _)| *f == frame) {
                Some(i) => i,
                None => match self.ram_ring.iter().position(|(_, t)| t.is_none()) {
                    Some(i) => i,
                    None => continue, // ring full — skip
                },
            };

            if self.ram_ring[slot_idx].1.is_none() {
                let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("RAM Preview Slot"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.ram_ring[slot_idx] = (frame, Some((texture, view)));
            } else {
                self.ram_ring[slot_idx].0 = frame;
            }
            self.ram_render_idx = slot_idx;
            self.render_internal(comp, frame, exposure_ev, lut_mode, false);
        }
        self.ram_render_idx = usize::MAX;
    }

    /// Texture view for a pre-rendered frame, if present and still valid.
    pub fn ram_frame_view(&self, frame: u32) -> Option<&wgpu::TextureView> {
        if self.ram_ring_version != crate::core::frame_cache::current_version() {
            return None;
        }
        self.ram_ring
            .iter()
            .find(|(f, t)| *f == frame && t.is_some())
            .and_then(|(_, t)| t.as_ref().map(|(_, v)| v))
    }

    /// Drops all cached RAM preview frames (e.g. when playback stops).
    pub fn clear_ram_preview(&mut self) {
        self.ram_ring.clear();
        self.ram_ring_version = 0;
    }

    /// Uploads a video frame PNG as a GPU texture, cached by (layer, dir, frame).
    /// Pixels come from the shared CPU image cache, so playback reuses decoded data.
    fn get_or_create_video_frame_texture(
        &self,
        layer_id: &str,
        frames_dir: &str,
        frame_idx: u32,
        version: u64,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let key: VideoFrameKey = (layer_id.to_string(), frames_dir.to_string(), frame_idx);
        // Re-imports write into the same frames_dir; the version check ensures
        // stale pixels from a previous project state are never displayed.
        if self.video_cache_version.get() != version {
            self.video_frame_cache.borrow_mut().clear();
            self.video_cache_version.set(version);
        }
        if let Some(bg) = self.video_frame_cache.borrow().get(&key).map(|(_, bg)| bg.clone()) {
            return Some((1, 1, bg));
        }
        let png_path = std::path::Path::new(frames_dir)
            .join(format!("frame_{:05}.png", frame_idx))
            .to_string_lossy()
            .to_string();
        let (tw, th, pixels) = {
            use crate::core::image_cache::with_image_cache;
            with_image_cache(|cache| {
                cache.load_image(&png_path).map(|img| {
                    (img.width, img.height, img.pixels.clone())
                })
            })?
        };

        let size = wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Video Frame Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(tw * 4),
                rows_per_image: Some(th),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
            label: Some("video_frame_bind_group"),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        {
            let mut cache = self.video_frame_cache.borrow_mut();
            cache.insert(key, (std::sync::Arc::new(texture), bind_group.clone()));
            // Simple FIFO eviction: HashMap order is arbitrary but bounded memory
            // matters more than exact LRU here (frames re-upload cheaply).
            while cache.len() > MAX_VIDEO_FRAME_TEXTURES {
                if let Some(oldest) = cache.keys().next().cloned() {
                    cache.remove(&oldest);
                } else {
                    break;
                }
            }
        }
        Some((tw, th, bind_group))
    }

    /// Caps preview render width (px). `None` renders at composition resolution.
    pub fn set_preview_max_width(&mut self, cap: Option<u32>) {
        if self.preview_max_width != cap {
            self.preview_max_width = cap;
            // Resolution change must invalidate the dirty-check keys
            self.last_main_key = None;
            self.last_snapshot_key = None;
        }
    }

    /// Renders the given composition at the specified frame, returning the texture view.
    pub fn render(&mut self, comp: &Composition, frame: u32, exposure_ev: f32, lut_mode: u32) -> (&wgpu::TextureView, bool) {
        let recreated = self.render_internal(comp, frame, exposure_ev, lut_mode, false);
        if self.target_view.is_none() {
            log::error!("[WgpuRenderer] render(): target view missing; using fallback view");
            self.dummy_view_or_create(false);
        }
        (
            self.target_view.as_ref().expect("fallback view just created"),
            recreated,
        )
    }

    /// Renders the given composition at the specified frame to the snapshot target, returning the snapshot texture view.
    pub fn render_snapshot_frame(&mut self, comp: &Composition, frame: u32, exposure_ev: f32, lut_mode: u32) -> (&wgpu::TextureView, bool) {
        let recreated = self.render_internal(comp, frame, exposure_ev, lut_mode, true);
        if self.snapshot_view.is_none() {
            log::error!("[WgpuRenderer] render_snapshot_frame(): snapshot view missing; using fallback view");
            self.dummy_view_or_create(true);
        }
        (
            self.snapshot_view.as_ref().expect("fallback view just created"),
            recreated,
        )
    }

    /// Last-resort 1x1 fallback view so a missing target can never panic the UI thread.
    fn dummy_view_or_create(&mut self, snapshot: bool) -> &wgpu::TextureView {
        let slot = if snapshot { &mut self.snapshot_view } else { &mut self.target_view };
        if slot.is_none() {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Fallback 1x1 Target"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            *slot = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        }
        // Invariant: the slot was just populated above (or already held a view).
        slot.as_ref().expect("fallback target view was just created")
    }
}

/// Helper to align dynamic uniform buffer byte offsets against WGPU hardware limits.
/// Dynamically adapts to device.limits().min_uniform_buffer_offset_alignment (e.g. 64B, 256B, 512B).
#[allow(dead_code)]
pub fn align_uniform_buffer_offset(offset: u64, alignment: u32) -> u64 {
    let align = alignment as u64;
    if align == 0 {
        return offset;
    }
    (offset + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_uniform_memory_alignment() {
        let size = std::mem::size_of::<LayerUniform>();
        assert_eq!(
            size % 256,
            0,
            "LayerUniform size ({} bytes) must be a multiple of 256 for WGPU dynamic uniform offset alignment",
            size
        );
        for align in [64, 256, 512] {
            assert_eq!(align_uniform_buffer_offset(size as u64, align) % align as u64, 0);
        }
    }
}

#[cfg(test)]
mod video_cache_tests {
    #[test]
    fn test_video_texture_budget_is_sane() {
        // 600 frames x 1080p RGBA ≈ 1.5GB — must stay under 2GB
        // (and hold ≥150 frames ≈ 5s at 30fps, enforced by the constant itself).
        let bytes = crate::core::renderer::MAX_VIDEO_FRAME_TEXTURES as u64 * 1920 * 1080 * 4;
        assert!(bytes < 2 * 1024 * 1024 * 1024, "budget {} bytes too large", bytes);
    }
}

// ── GPU Compositing Pipeline (future) ──
// The full GPU compositing pipeline would:
// 1. Upload each layer's rasterized content as a separate texture
// 2. Use the composite.wgsl compute shader to blend all layers on GPU
// 3. Apply effects as fragment shader passes between blends
// 4. Output directly to the display texture without CPU roundtrip
//
// Current status: layers are composited on CPU by software_renderer.rs,
// then the result is uploaded as a single texture via WgpuRenderer.
// The GPU path handles UI rendering, video frame textures, text textures,
// and RAM preview ring buffers.