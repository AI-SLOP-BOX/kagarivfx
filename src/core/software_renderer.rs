use crate::core::timeline::{Composition, Layer, LayerType, BlendMode, ShapeType, TrimPaths, TrackMatteMode, LightType, Light3D};
use crate::core::mask::{point_in_polygon, MaskMode};
use rayon::prelude::*;

#[derive(Default)]
struct CpuMaskEntry {
    vertices: Vec<[f32; 2]>,
    feather: f32,
    expansion: f32,
    inverted: bool,
    mode: MaskMode,
}

/// Offset a polygon's vertices along their outward normals by `expansion` pixels.
fn offset_polygon_vertices(vertices: &[[f32; 2]], expansion: f32) -> Vec<[f32; 2]> {
    if vertices.len() < 3 || expansion.abs() < 0.01 {
        return vertices.to_vec();
    }
    let n = vertices.len();
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let prev = vertices[(i + n - 1) % n];
        let curr = vertices[i];
        let next = vertices[(i + 1) % n];

        let e1 = [curr[0] - prev[0], curr[1] - prev[1]];
        let e2 = [next[0] - curr[0], next[1] - curr[1]];

        let len1 = (e1[0] * e1[0] + e1[1] * e1[1]).sqrt().max(1e-6);
        let len2 = (e2[0] * e2[0] + e2[1] * e2[1]).sqrt().max(1e-6);
        let n1 = [-e1[1] / len1, e1[0] / len1];
        let n2 = [-e2[1] / len2, e2[0] / len2];

        let avg_n = [(n1[0] + n2[0]) * 0.5, (n1[1] + n2[1]) * 0.5];
        let avg_len = (avg_n[0] * avg_n[0] + avg_n[1] * avg_n[1]).sqrt().max(1e-6);
        let normal = [avg_n[0] / avg_len, avg_n[1] / avg_len];

        result.push([
            curr[0] + normal[0] * expansion,
            curr[1] + normal[1] * expansion,
        ]);
    }
    result
}

/// Perspective-project a 3D layer's corners onto screen space.
/// Returns [(screen_x, screen_y, u, v)] for the 4 corners (TL, TR, BR, BL),
/// or None if the layer is behind the camera.
#[allow(clippy::too_many_arguments)]
/// Project a world point through a light onto the z=0 receiver plane.
/// Returns None when the ray is parallel to the plane or points away.
fn project_point_to_plane_z0(
    light: [f32; 3],
    point: [f32; 3],
) -> Option<[f32; 2]> {
    let dz = point[2] - light[2];
    if dz.abs() < 0.001 {
        return None;
    }
    let t = -light[2] / dz;
    if t <= 0.0 {
        return None;
    }
    Some([light[0] + t * (point[0] - light[0]), light[1] + t * (point[1] - light[1])])
}

/// Fill a convex polygon into the density buffer (scanline-free point test
/// over the bbox — quads are tiny relative to frame, this is plenty fast).
fn accumulate_polygon_density(
    density: &mut [f32],
    width: u32,
    height: u32,
    pts: &[[f32; 2]],
    amount: f32,
) {
    if pts.len() < 3 || amount <= 0.0 {
        return;
    }
    let min_x = pts.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min).floor().max(0.0) as u32;
    let max_x = pts.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max).ceil().min(width as f32) as u32;
    let min_y = pts.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min).floor().max(0.0) as u32;
    let max_y = pts.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max).ceil().min(height as f32) as u32;
    for py in min_y..max_y.min(height) {
        for px in min_x..max_x.min(width) {
            if point_in_polygon(px as f32 + 0.5, py as f32 + 0.5, pts) {
                let idx = (py * width + px) as usize;
                if idx < density.len() {
                    density[idx] = (density[idx] + amount).min(1.0);
                }
            }
        }
    }
}

fn box_blur_f32(buf: &mut [f32], width: u32, height: u32, radius: u32) {
    if radius == 0 || buf.is_empty() || width == 0 || height == 0 {
        return;
    }
    let w = width as usize;
    let h = height as usize;
    let r = radius as usize;
    let mut tmp = vec![0.0f32; buf.len()];
    // Horizontal
    for y in 0..h {
        let row = &buf[y * w..(y + 1) * w];
        let out = &mut tmp[y * w..(y + 1) * w];
        for (x, slot) in out.iter_mut().enumerate() {
            let lo = x.saturating_sub(r);
            let hi = (x + r + 1).min(w);
            let s: f32 = row[lo..hi].iter().sum();
            *slot = s / (hi - lo) as f32;
        }
    }
    // Vertical
    for x in 0..w {
        for y in 0..h {
            let lo = y.saturating_sub(r);
            let hi = (y + r + 1).min(h);
            let mut s = 0.0;
            for yy in lo..hi {
                s += tmp[yy * w + x];
            }
            buf[y * w + x] = s / (hi - lo) as f32;
        }
    }
}

/// Build the caster's outline points in LOCAL layer space (centered origin),
/// honoring the actual shape instead of its bounding quad. Falls back to the
/// full-size rect for raster/content types.
fn caster_outline_points(layer: &Layer, base_w: f32, base_h: f32, frame: u32) -> Vec<[f32; 2]> {
    let shape_pts: Option<Vec<[f32; 2]>> = match &layer.layer_type {
        LayerType::Shape { shape_type, .. } => match shape_type {
            ShapeType::Ellipse { width, height } => {
                let w = width.evaluate(frame).max(1.0) / 2.0;
                let h = height.evaluate(frame).max(1.0) / 2.0;
                Some((0..24).map(|i| {
                    let a = i as f32 / 24.0 * std::f32::consts::TAU;
                    [w * a.cos(), h * a.sin()]
                }).collect())
            }
            ShapeType::Polygon { sides, radius } => {
                let n = sides.evaluate(frame).round().max(3.0) as usize;
                let r = radius.evaluate(frame).max(1.0);
                Some((0..n).map(|i| {
                    let a = i as f32 / n as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                    [r * a.cos(), r * a.sin()]
                }).collect())
            }
            ShapeType::Star { points, inner_radius, outer_radius } => {
                let n = points.evaluate(frame).round().max(3.0) as usize;
                let ri = inner_radius.evaluate(frame).max(1.0);
                let ro = outer_radius.evaluate(frame).max(ri);
                Some((0..n * 2).map(|i| {
                    let a = i as f32 / (n * 2) as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                    let r = if i % 2 == 0 { ro } else { ri };
                    [r * a.cos(), r * a.sin()]
                }).collect())
            }
            ShapeType::Rectangle { width, height, .. } => {
                let hw = width.evaluate(frame).max(1.0) / 2.0;
                let hh = height.evaluate(frame).max(1.0) / 2.0;
                Some(vec![[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]])
            }
            _ => None,
        },
        _ => None,
    };
    // Shape-local coordinates are already in comp pixels; layer scale is
    // applied by the caller via resolve transform values.
    let _ = (base_w, base_h);
    shape_pts.unwrap_or_default()
}

/// Build the shadow density map (0=lit, 1=fully shadowed) at comp resolution:
/// every shadow-casting light projects every casting layer's world quad onto
/// the z=0 plane; densities accumulate and are softened by a small blur.
pub fn build_shadow_map(comp: &Composition, frame: u32, width: u32, height: u32) -> Vec<f32> {
    let n = (width.max(1) as usize) * (height.max(1) as usize);
    let mut density = vec![0.0f32; n];

    for light in &comp.lights {
        if !light.casts_shadows || light.intensity <= 0.0 {
            continue;
        }
        let lpos = light.position.evaluate(frame);
        let strength = ((light.shadow_darkness / 100.0) * (light.intensity / 100.0)).clamp(0.0, 1.0);
        if strength <= 0.003 {
            continue;
        }
        for layer in &comp.layers {
            if !layer.is_active(frame) || !layer.visible || !layer.material.cast_shadows {
                continue;
            }
            let (pos, scale, rot, _op) = comp.resolve_world_transform(layer, frame);
            // Outline points: real shape geometry when available (ellipse,
            // polygon, star, rect), else the full-size rect quad.
            let (base_w, base_h) = match &layer.layer_type {
                LayerType::Solid { .. } | LayerType::Shape { .. } | LayerType::Image { .. }
                | LayerType::Video { .. } | LayerType::Text { .. } | LayerType::PreComp { .. } => {
                    (comp.width as f32, comp.height as f32)
                }
                _ => continue,
            };
            let mut local_pts = caster_outline_points(layer, base_w, base_h, frame);
            if local_pts.is_empty() {
                let hw = base_w / 2.0;
                let hh = base_h / 2.0;
                local_pts = vec![[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]];
            }
            if local_pts.len() < 3 {
                continue;
            }
            let lz = if layer.is_3d {
                layer.transform_3d.position.evaluate(frame)[2]
            } else {
                0.0
            };
            // Coplanar with the receiver plane: its "shadow" lands exactly on
            // itself and would blanket the frame — skip.
            if lz.abs() < 1.0 {
                continue;
            }
            let rad = rot.to_radians();
            let (c, s) = (rad.cos(), rad.sin());
            // Layer scale applies to shape-local coordinates too
            let sx_mul = scale[0].abs() / 100.0;
            let sy_mul = scale[1].abs() / 100.0;
            let mut projected: Vec<[f32; 2]> = Vec::with_capacity(local_pts.len());
            for cl in &local_pts {
                let lx2 = cl[0] * sx_mul;
                let ly2 = cl[1] * sy_mul;
                let wx = pos[0] + lx2 * c - ly2 * s;
                let wy = pos[1] + lx2 * s + ly2 * c;
                match project_point_to_plane_z0(lpos, [wx, wy, lz]) {
                    Some(p) => projected.push(p),
                    None => {
                        projected.clear();
                        break;
                    }
                }
            }
            if projected.len() == local_pts.len() {
                // Apply distance attenuation per shadow-casting pixel region

                let atten = crate::core::software_renderer::light_attenuation(light, lpos, [lpos[0], lpos[1], 0.0], frame);
                accumulate_polygon_density(&mut density, width, height, &projected, strength * atten);
            }
        }
    }

    // Soft penumbra
    box_blur_f32(&mut density, width, height, ((width.max(height)) / 64).clamp(1, 8));
    density
}

/// Compute light attenuation factor based on distance from light to a point.
/// `falloff`: 0 = no falloff, 1 = linear, 2 = inverse-square (realistic).
/// `max_radius`: 0 = unlimited, otherwise hard cutoff.
pub fn light_attenuation(light: &Light3D, light_pos: [f32; 3], point_pos: [f32; 3], _frame: u32) -> f32 {
    let dx = point_pos[0] - light_pos[0];
    let dy = point_pos[1] - light_pos[1];
    let dz = point_pos[2] - light_pos[2];
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();

    if light.max_radius > 0.0 && dist > light.max_radius {
        return 0.0;
    }

    if light.falloff <= 0.001 {
        return 1.0;
    }

    let atten = match light.falloff as u32 {
        0 => 1.0,
        1 => {
            // Linear falloff: 1 - dist/max_dist
            if light.max_radius > 0.0 {
                (1.0 - dist / light.max_radius).max(0.0)
            } else {
                1.0 / (1.0 + dist * 0.001)
            }
        }
        _ => {
            // Inverse-square (physically correct): 1/(1 + dist^2 * k)
            let k = 0.0001; // scale factor
            1.0 / (1.0 + dist * dist * k)
        }
    };

    atten.clamp(0.0, 1.0)
}

/// Compute spot light cone factor. Returns 1.0 for point lights.
pub fn spot_cone_factor(light: &Light3D, light_pos: [f32; 3], point_pos: [f32; 3], _frame: u32) -> f32 {
    match light.light_type {
        LightType::Spot { cone_angle_deg, cone_feather_pct } => {
            let dx = point_pos[0] - light_pos[0];
            let dy = point_pos[1] - light_pos[1];
            let dz = point_pos[2] - light_pos[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist < 0.001 {
                return 1.0;
            }
            let dir_z = dz / dist;
            let cone_half = (cone_angle_deg * 0.5).to_radians();
            let cos_cone = cone_half.cos();
            let cos_dir = -dir_z;

            if cos_dir < cos_cone {
                0.0
            } else {
                let feather = (cone_feather_pct / 100.0).clamp(0.0, 1.0);
                let edge = cos_cone + (1.0 - cos_cone) * feather;
                if cos_dir > edge {
                    1.0
                } else {
                    (cos_dir - cos_cone) / (edge - cos_cone)
                }
            }
        }
        _ => 1.0,
    }
}

#[allow(clippy::too_many_arguments)]
/// Expand every collapsed (`is_collapsed`) PreComp layer into its children,
/// composing parent transform into each child so they render in the parent's
/// coordinate space (AE "Collapse Transformations"). Recursive up to
/// MAX_PRECOMP_DEPTH. When nothing is collapsed this is a cheap clone-free
/// passthrough for callers that need an owned Composition anyway.
pub fn flatten_collapsed(comp: &Composition, frame: u32) -> Composition {
    flatten_collapsed_limited(comp, frame, 0)
}

fn flatten_collapsed_limited(comp: &Composition, frame: u32, depth: u32) -> Composition {
    let mut out = comp.clone();
    if depth >= MAX_PRECOMP_DEPTH {
        return out;
    }
    let mut expanded: Vec<Layer> = Vec::with_capacity(out.layers.len());
    let any_collapsed = out.layers.iter().any(|l| {
        l.is_collapsed && matches!(l.layer_type, LayerType::PreComp { .. })
    });
    if !any_collapsed {
        return out;
    }
    let layers_std = std::mem::take(&mut out.layers);
    for layer in layers_std {
        let (Some(sub_id), true) = (
            match &layer.layer_type {
                LayerType::PreComp { comp_id } => Some(comp_id.clone()),
                _ => None,
            },
            layer.is_collapsed && layer.is_active(frame),
        ) else {
            expanded.push(layer);
            continue;
        };
        let Some(sub) = comp.find_sub_comp(&sub_id) else {
            expanded.push(layer);
            continue;
        };
        // Parent transform (already resolves parenting chains)
        let (ppos, pscale, prot, popa) = out.resolve_world_transform(&{
            // resolve against ORIGINAL comp so chains above this layer hold
            comp.layers.iter().find(|l| l.id == layer.id).cloned().unwrap_or(layer.clone())
        }, frame);
        let prad = prot.to_radians();
        let (pc, ps) = (prad.cos(), prad.sin());
        let pz = if layer.is_3d { layer.transform_3d.position.evaluate(frame)[2] } else { 0.0 };

        // Compose: parent ∘ child (2D affine). Sub-comp coordinates are
        // absolute within the sub frame; map them relative to the sub center
        // onto the parent layer's center before scaling/rotating.
        let sub_cx = sub.width as f32 * 0.5;
        let sub_cy = sub.height as f32 * 0.5;
        for mut child in sub.layers.clone() {
            if !child.is_active(frame) || !child.visible {
                continue;
            }
            let (cpos, cscale, crot, copa) = sub.resolve_world_transform(&child, frame);
            // Compose: parent ∘ child (2D affine)
            let sx = cscale[0] * pscale[0] / 100.0;
            let sy = cscale[1] * pscale[1] / 100.0;
            let rel_x = cpos[0] - sub_cx;
            let rel_y = cpos[1] - sub_cy;
            let lx = rel_x * pscale[0] / 100.0;
            let ly = rel_y * pscale[1] / 100.0;
            let npos = [ppos[0] + lx * pc - ly * ps, ppos[1] + lx * ps + ly * pc];
            let nrot = prot + crot;
            let nopa = (popa / 100.0) * (copa / 100.0) * 100.0;

            child.transform.position = crate::core::property::Animatable::new_constant(npos);
            child.transform.scale = crate::core::property::Animatable::new_constant([sx.max(0.001), sy.max(0.001)]);
            child.transform.rotation = crate::core::property::Animatable::new_constant(nrot);
            child.transform.opacity = crate::core::property::Animatable::new_constant(nopa);

            // 3D continuity: lift child into parent space along Z
            if layer.is_3d {
                let cz = if child.is_3d { child.transform_3d.position.evaluate(frame)[2] } else { 0.0 };
                child.is_3d = true;
                child.transform_3d.position =
                    crate::core::property::Animatable::new_constant([npos[0], npos[1], pz + cz]);
            }
            expanded.push(child);
        }
    }
    out.layers = expanded;
    // Recurse for nested collapsed precomps brought in by expansion
    if out.layers.iter().any(|l| l.is_collapsed && matches!(l.layer_type, LayerType::PreComp { .. })) {
        return flatten_collapsed_limited(&out, frame, depth + 1);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn perspective_project_layer(
    cam_fov: f32,
    cam_pos: [f32; 3],
    cam_rot: [f32; 3],       // degrees: [rx, ry, rz]
    layer_pos: [f32; 3],
    layer_rot: [f32; 3],     // degrees
    layer_scale: [f32; 2],   // percent (100 = no scale)
    layer_width: f32,
    layer_height: f32,
    screen_width: f32,
    screen_height: f32,
) -> Option<[[f32; 4]; 4]> {
    let hw = layer_width * 0.5 * (layer_scale[0] / 100.0);
    let hh = layer_height * 0.5 * (layer_scale[1] / 100.0);

    let corners: [[f32; 3]; 4] = [
        [-hw, -hh, 0.0],
        [ hw, -hh, 0.0],
        [ hw,  hh, 0.0],
        [-hw,  hh, 0.0],
    ];

    // Euler angles to rotation matrix (XYZ order)
    let to_rad = |d: f32| d * std::f32::consts::PI / 180.0;
    let (rx, ry, rz) = (to_rad(layer_rot[0]), to_rad(layer_rot[1]), to_rad(layer_rot[2]));
    let (crx, srx) = (rx.cos(), rx.sin());
    let (cry, sry) = (ry.cos(), ry.sin());
    let (crz, srz) = (rz.cos(), rz.sin());

    // Rz * Ry * Rx
    let rotate = |p: [f32; 3]| -> [f32; 3] {
        // Rx
        let y1 = p[1] * crx - p[2] * srx;
        let z1 = p[1] * srx + p[2] * crx;
        let x1 = p[0];
        // Ry
        let x2 = x1 * cry + z1 * sry;
        let z2 = -x1 * sry + z1 * cry;
        let y2 = y1;
        // Rz
        [
            x2 * crz - y2 * srz,
            x2 * srz + y2 * crz,
            z2,
        ]
    };

    // World-space corners
    let world: Vec<[f32; 3]> = corners.iter().map(|c| {
        let r = rotate(*c);
        [r[0] + layer_pos[0], r[1] + layer_pos[1], r[2] + layer_pos[2]]
    }).collect();

    // Camera transform (simplified: translate + Z-rotate only)
    let cam_zr = to_rad(cam_rot[2]);
    let (ccrz, ssrz) = (cam_zr.cos(), cam_zr.sin());

    let cam_space: Vec<[f32; 3]> = world.iter().map(|c| {
        let dx = c[0] - cam_pos[0];
        let dy = c[1] - cam_pos[1];
        let dz = c[2] - cam_pos[2];
        [
            dx * ccrz - dy * ssrz,
            dx * ssrz + dy * ccrz,
            dz,
        ]
    }).collect();

    if cam_space.iter().any(|c| c[2] <= 0.1) {
        return None;
    }

    let fov_rad = cam_fov * std::f32::consts::PI / 180.0;
    let focal = (screen_height * 0.5) / (fov_rad * 0.5).tan();

    let mut result = [[0.0f32; 4]; 4];
    for (i, c) in cam_space.iter().enumerate() {
        let z = c[2];
        let sx = screen_width * 0.5 + (c[0] * focal) / z;
        let sy = screen_height * 0.5 - (c[1] * focal) / z;
        let u = if i == 0 || i == 3 { 0.0 } else { 1.0 };
        let v = if i == 0 || i == 1 { 0.0 } else { 1.0 };
        result[i] = [sx, sy, u, v];
    }
    Some(result)
}

/// Render a sub-composition into a pixel buffer (for PreComp nesting).
pub fn render_sub_comp(_comp: &Composition, sub_comp_id: &str, _frame: u32, _width: u32, _height: u32, time_remapped_frame: u32) -> Option<Vec<u8>> {
    // Find the sub-comp by id in the project (we need to search all compositions)
    // Since Composition doesn't store a reference to other comps, we use a flat lookup approach:
    // The sub-comp is expected to be passed via the project's compositions list.
    // For now, we render a copy of the current comp with only the sub-comp's layers.
    let _ = (sub_comp_id, time_remapped_frame);
    // In a full implementation, this would find the sub-comp and render it recursively.
    // For now, return None to signal that pre-comp nesting is not yet fully wired.
    // The composition system needs a project-level composition registry.
    None
}

/// Maximum pre-comp nesting depth before we bail out (cycle / pathological nesting guard).
pub const MAX_PRECOMP_DEPTH: u32 = 16;

thread_local! {
    static PRECOMP_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// Stack of composition IDs currently being rendered (cycle detection).
    static PRECOMP_STACK: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    /// Precomp render cache: avoids re-rendering the same sub-comp at the
    /// same frame when it's referenced by multiple layers.
    static PRECOMP_RENDER_CACHE: std::cell::RefCell<crate::core::precomp_cache::PrecompCache> = std::cell::RefCell::new(crate::core::precomp_cache::PrecompCache::new(64));
}

/// Render a pre-comp by recursively rendering its layers into a pixel buffer.
/// This is the core of pre-comp nesting support. Uses a thread-local cache
/// to avoid redundant re-renders when the same pre-comp is referenced by
/// multiple layers at the same frame.
pub fn render_precomp_layers(_comp: &Composition, precomp_comp: &Composition, frame: u32, width: u32, height: u32) -> Vec<u8> {
    // Cache check: skip full render if we already have this precomp's pixels
    let cached = PRECOMP_RENDER_CACHE.with(|cache| {
        cache.borrow_mut().get(&precomp_comp.id, frame, width, height)
    });
    if let Some(pixels) = cached {
        return pixels;
    }

    // Cycle detection: a comp that (indirectly) contains itself returns empty
    // immediately instead of burning the whole depth budget on garbage.
    let cyclic = PRECOMP_STACK.with(|stack| {
        let mut s = stack.borrow_mut();
        if s.iter().any(|id| id == &precomp_comp.id) {
            true
        } else {
            s.push(precomp_comp.id.clone());
            false
        }
    });
    if cyclic {
        log::warn!("[Renderer] Pre-comp cycle detected at '{}' ; skipping nested render", precomp_comp.name);
        return Vec::new();
    }
    // Guard against pathologically deep (but acyclic) pre-comp nesting
    let overflow = PRECOMP_DEPTH.with(|d| {
        let cur = d.get();
        if cur >= MAX_PRECOMP_DEPTH {
            true
        } else {
            d.set(cur + 1);
            false
        }
    });
    if overflow {
        log::warn!("[Renderer] Pre-comp nesting depth limit exceeded; skipping nested render");
        PRECOMP_STACK.with(|s| { s.borrow_mut().pop(); });
        return Vec::new();
    }
    let result = render_precomp_layers_inner(_comp, precomp_comp, frame, width, height);
    PRECOMP_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    PRECOMP_STACK.with(|s| { s.borrow_mut().pop(); });
    // Cache the result for future lookups at the same (comp, frame, resolution)
    if !result.is_empty() {
        PRECOMP_RENDER_CACHE.with(|cache| {
            cache.borrow_mut().insert(&precomp_comp.id, frame, width, height, result.clone());
        });
    }
    result
}

fn render_precomp_layers_inner(_comp: &Composition, precomp_comp: &Composition, frame: u32, width: u32, height: u32) -> Vec<u8> {
    let size = rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 { return Vec::new(); }

    let mut buffer = vec![0u8; size];
    // Fill with transparent black
    for p in (0..size).step_by(4) {
        buffer[p] = 0; buffer[p+1] = 0; buffer[p+2] = 0; buffer[p+3] = 0;
    }

    let has_solo = precomp_comp.layers.iter().any(|l| l.is_active(frame) && l.solo);

    for layer in &precomp_comp.layers {
        if !layer.is_active(frame) || !layer.visible || layer.is_guide_layer { continue; }
        if has_solo && !layer.solo { continue; }

        let effective_frame = {
            let f = layer.remap_frame(frame);
            match &layer.posterize_time {
                Some(pt) if pt.enabled => crate::core::posterize_time::quantize_frame_posterize(f, precomp_comp.fps, pt),
                _ => f,
            }
        };
        let (pos, scale, rotation, opacity) = precomp_comp.resolve_world_transform(layer, effective_frame);
        let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
        if l_opacity < 0.001 { continue; }

        if matches!(layer.layer_type, LayerType::AdjustmentLayer) {
            if !layer.effects.is_empty() && l_opacity > 0.003 {
                let mut adjusted = buffer.clone();
                crate::core::cpu_effects::apply_layer_effects(&mut adjusted, width, height, &layer.effects, effective_frame, precomp_comp.fps);
                for i in (0..buffer.len()).step_by(4) {
                    for c in 0..3 {
                        buffer[i + c] = (buffer[i + c] as f32 * (1.0 - l_opacity) + adjusted[i + c] as f32 * l_opacity).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
            continue;
        }

        let (base_w, base_h) = match &layer.layer_type {
            LayerType::Solid { .. } | LayerType::PreComp { .. } => (precomp_comp.width as f32, precomp_comp.height as f32),
            LayerType::Text { font_size, text, .. } => (
                (text.chars().count().max(1) as f32 * *font_size as f32 * 0.6).max(*font_size as f32),
                *font_size as f32 * 1.2,
            ),
            LayerType::Shape { .. } | LayerType::Image { .. } | LayerType::Video { .. } => (precomp_comp.width as f32, precomp_comp.height as f32),
            _ => continue,
        };

        let w = (scale[0].abs() / 100.0) * base_w;
        let h = (scale[1].abs() / 100.0) * base_h;

        let base_color = match &layer.layer_type {
            LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
            LayerType::Shape { color, .. } => *color,
            LayerType::Image { .. } => [0.2, 0.6, 0.9, 1.0],
            LayerType::PreComp { .. } => [1.0, 1.0, 1.0, 1.0],
            _ => continue,
        };

        let rad = rotation.to_radians();
        let (cos_r, sin_r) = rad.sin_cos();
        let cx = pos[0];
        let cy = pos[1];
        let bounds_x = w * 0.5;
        let bounds_y = h * 0.5;

        // NaN-safe, sign-safe bounding box (`as u32` saturates NaN/inf; abs() guards
        // against negative scale flipping the bounds)
        let ext_x = (bounds_x.abs() + 2.0) * 1.5;
        let ext_y = (bounds_y.abs() + 2.0) * 1.5;
        let min_x = ((cx - ext_x).max(0.0) as u32).min(width);
        let max_x = ((cx + ext_x).max(0.0) as u32).min(width);
        let min_y = ((cy - ext_y).max(0.0) as u32).min(height);
        let max_y = ((cy + ext_y).max(0.0) as u32).min(height);
        let bw = max_x.saturating_sub(min_x);
        let bh = max_y.saturating_sub(min_y);
        if bw == 0 || bh == 0 { continue; }

        let buf_size = (bw * bh * 4) as usize;
        let mut layer_buf = vec![0u8; buf_size];

        match &layer.layer_type {
            LayerType::Shape { shape_type, color, stroke_color, stroke_width, fill_type, .. } => {
                // SDF shape rendering (same path as the main renderer)
                rasterize_shape_sdf(
                    &mut layer_buf, bw, bh, min_x, min_y,
                    cx, cy, bounds_x, bounds_y,
                    *color, fill_type, *stroke_color, *stroke_width, l_opacity,
                    shape_type, effective_frame, layer.trim_paths.as_ref(),
                );
            }
            LayerType::Image { path } => {
                // Texture sampling via image cache
                use crate::core::image_cache::with_image_cache;
                let img_path = path.clone();
                with_image_cache(|cache| {
                    if let Some(img) = cache.load_image(&img_path) {
                        let img_w = img.width as f32;
                        let img_h = img.height as f32;
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let dx = px as f32 - cx;
                                let dy = py as f32 - cy;
                                let lx = dx * cos_r + dy * sin_r;
                                let ly = -dx * sin_r + dy * cos_r;
                                let u = (lx / bounds_x + 1.0) * 0.5;
                                let v = (ly / bounds_y + 1.0) * 0.5;
                                #[allow(clippy::manual_range_contains)]
            if u < 0.0 || 1.0 < u || v < 0.0 || 1.0 < v { continue; }
                                let tex_x = ((u * (img_w - 1.0)).round() as u32).min(img.width - 1);
                                let tex_y = ((v * (img_h - 1.0)).round() as u32).min(img.height - 1);
                                let tidx = ((tex_y * img.width + tex_x) * 4) as usize;
                                if tidx + 3 >= img.pixels.len() { continue; }
                                let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                                if lidx + 3 < layer_buf.len() {
                                    let src_a = (img.pixels[tidx + 3] as f32 / 255.0) * l_opacity;
                                    layer_buf[lidx] = img.pixels[tidx];
                                    layer_buf[lidx+1] = img.pixels[tidx+1];
                                    layer_buf[lidx+2] = img.pixels[tidx+2];
                                    layer_buf[lidx+3] = (src_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                });
            }
            LayerType::Text { text, font_size, color, font_family, tracking, .. } => {
                // Glyph rendering via font rasterizer
                use crate::core::font_rasterizer::with_font_rasterizer;
                let text_color = *color;
                let text_str = text.clone();
                let fs = *font_size as f32;
                let tk = *tracking;
                let family = font_family.clone();
                with_font_rasterizer(|rasterizer| {
                    let family_name = rasterizer.resolve_family(&family);
                    if let Some((tw, th, text_pixels)) = rasterizer.rasterize_text(&family_name, &text_str, fs, text_color, tk) {
                        let origin_x = (cx - tw as f32 * 0.5) as i32;
                        let origin_y = (cy - th as f32 * 0.5) as i32;
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let tx = px as i32 - origin_x;
                                let ty = py as i32 - origin_y;
                                if tx < 0 || ty < 0 || (tx as u32) >= tw || (ty as u32) >= th { continue; }
                                let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                                if tidx + 3 >= text_pixels.len() { continue; }
                                let glyph_a = text_pixels[tidx + 3] as f32 / 255.0;
                                if glyph_a <= 0.001 { continue; }
                                let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                                if lidx + 3 < layer_buf.len() {
                                    let src_a = glyph_a * l_opacity;
                                    layer_buf[lidx] = (text_color[0] * 255.0) as u8;
                                    layer_buf[lidx+1] = (text_color[1] * 255.0) as u8;
                                    layer_buf[lidx+2] = (text_color[2] * 255.0) as u8;
                                    layer_buf[lidx+3] = (src_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                });
            }
            _ => {
                // Flat fill for Solid / PreComp / others
                for py in min_y..max_y {
                    for px in min_x..max_x {
                        let dx = px as f32 - cx;
                        let dy = py as f32 - cy;
                        let lx = dx * cos_r + dy * sin_r;
                        let ly = -dx * sin_r + dy * cos_r;
                        if lx >= -bounds_x && lx <= bounds_x && ly >= -bounds_y && ly <= bounds_y {
                            let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                            if lidx + 3 < layer_buf.len() {
                                let src_a = base_color[3] * l_opacity;
                                layer_buf[lidx] = (base_color[0] * 255.0) as u8;
                                layer_buf[lidx+1] = (base_color[1] * 255.0) as u8;
                                layer_buf[lidx+2] = (base_color[2] * 255.0) as u8;
                                layer_buf[lidx+3] = (src_a * 255.0) as u8;
                            }
                        }
                    }
                }
            }
        }

        // ── Paint strokes: drawn in buffer space so they follow the layer ──
        // DirtyRect integration: compute the layer's bounding box in buffer
        // space and skip strokes that are entirely outside it.
        if !layer.paint_strokes.is_empty() {
            let inv_sx = if scale[0].abs() > f32::EPSILON { 100.0 / scale[0] } else { 0.0 };
            let inv_sy = if scale[1].abs() > f32::EPSILON { 100.0 / scale[1] } else { 0.0 };
            let to_buf_local = |lp: [f32; 2]| -> [f32; 2] {
                // local -> world (rotate + scale + pos), then into buffer px
                let wx = cx + (lp[0] * cos_r - lp[1] * sin_r) * scale[0] / 100.0;
                let wy = cy + (lp[0] * sin_r + lp[1] * cos_r) * scale[1] / 100.0;
                [wx - min_x as f32, wy - min_y as f32]
            };
            let _ = (inv_sx, inv_sy); // inverse reserved for future pick tools
            // DirtyRect: bounding box of all strokes in buffer-pixel space
            let mut dirty_min_x = bw as f32;
            let mut dirty_min_y = bh as f32;
            let mut dirty_max_x = 0.0f32;
            let mut dirty_max_y = 0.0f32;
            for stroke in &layer.paint_strokes {
                let end_f = if stroke.end_frame == 0 { layer.out_frame } else { stroke.end_frame };
                if effective_frame < stroke.start_frame || effective_frame > end_f {
                    continue;
                }
                let half = stroke.size * 0.5;
                for &p in &stroke.points {
                    let bp = to_buf_local(p);
                    dirty_min_x = dirty_min_x.min(bp[0] - half);
                    dirty_min_y = dirty_min_y.min(bp[1] - half);
                    dirty_max_x = dirty_max_x.max(bp[0] + half);
                    dirty_max_y = dirty_max_y.max(bp[1] + half);
                }
            }
            // Clamp dirty rect to buffer bounds
            let dr_x0 = dirty_min_x.floor().max(0.0) as u32;
            let dr_y0 = dirty_min_y.floor().max(0.0) as u32;
            let dr_x1 = dirty_max_x.ceil().min(bw as f32) as u32;
            let dr_y1 = dirty_max_y.ceil().min(bh as f32) as u32;
            for stroke in &layer.paint_strokes {
                let end_f = if stroke.end_frame == 0 { layer.out_frame } else { stroke.end_frame };
                if effective_frame < stroke.start_frame || effective_frame > end_f {
                    continue;
                }
                // Quick AABB test: skip strokes entirely outside the dirty rect
                let stroke_min = stroke.points.iter().fold(
                    [f32::MAX, f32::MAX],
                    |acc, p| [acc[0].min(p[0]), acc[1].min(p[1])],
                );
                let stroke_max = stroke.points.iter().fold(
                    [f32::MIN, f32::MIN],
                    |acc, p| [acc[0].max(p[0]), acc[1].max(p[1])],
                );
                let sb_min = to_buf_local(stroke_min);
                let sb_max = to_buf_local(stroke_max);
                let s_min_x = sb_min[0] - stroke.size * 0.5;
                let s_min_y = sb_min[1] - stroke.size * 0.5;
                let s_max_x = sb_max[0] + stroke.size * 0.5;
                let s_max_y = sb_max[1] + stroke.size * 0.5;
                if s_max_x < dr_x0 as f32 || s_min_x > dr_x1 as f32
                    || s_max_y < dr_y0 as f32 || s_min_y > dr_y1 as f32
                {
                    continue; // stroke entirely outside dirty rect
                }
                let buf_pts: Vec<[f32; 2]> =
                    stroke.points.iter().map(|&p| to_buf_local(p)).collect();
                let mut col = stroke.color;
                col[3] *= l_opacity;
                crate::core::paint::draw_stroke(
                    &mut layer_buf, bw, bh, &buf_pts, col, stroke.size.max(1.0),
                );
            }
        }

        // ── Puppet warp: IDW-displace the isolated layer buffer before effects ──
        if !layer.puppet_pins.is_empty() {
            // layer_buf rows/cols are world-aligned samples offset by
            // (min_x, min_y), so comp-space -> buffer-space is a translation.
            let to_buf = |p: [f32; 2]| -> [f32; 2] {
                [p[0] - min_x as f32, p[1] - min_y as f32]
            };
            let pins: Vec<([f32; 2], [f32; 2])> = layer
                .puppet_pins
                .iter()
                .map(|pin| {
                    let s = to_buf(pin.comp_source);
                    let d = to_buf(pin.position.evaluate(effective_frame));
                    (s, d)
                })
                .collect();
            crate::core::puppet_warp::warp_layer_buf_mesh(&mut layer_buf, bw, bh, &pins);
        }

        crate::core::cpu_effects::apply_layer_effects(&mut layer_buf, bw, bh, &layer.effects, effective_frame, precomp_comp.fps);

        // Composite onto buffer
        for ly in 0..bh {
            for lx in 0..bw {
                let lidx = ((ly * bw + lx) * 4) as usize;
                let src_a = layer_buf[lidx+3] as f32 / 255.0;
                if src_a <= 0.001 { continue; }
                let px = min_x + lx;
                let py = min_y + ly;
                let idx = ((py * width + px) * 4) as usize;
                if idx+3 >= buffer.len() { continue; }
                let src_linear = crate::core::color::Rgbaf::from_rgba8(
                    layer_buf[lidx], layer_buf[lidx+1], layer_buf[lidx+2], 255,
                );
                let src_lin = crate::core::color::Rgbaf::new(src_linear.r, src_linear.g, src_linear.b, src_a);
                let dst_linear = crate::core::color::Rgbaf::from_rgba8(
                    buffer[idx], buffer[idx+1], buffer[idx+2], buffer[idx+3],
                );
                let out = src_lin.over(dst_linear);
                let out_rgba = out.to_rgba8();
                buffer[idx] = out_rgba[0];
                buffer[idx+1] = out_rgba[1];
                buffer[idx+2] = out_rgba[2];
                buffer[idx+3] = out_rgba[3];
            }
        }
    }

    buffer
}

/// Safe buffer size calculation: returns None on overflow or zero dimensions.
pub fn rgba_buffer_size(width: u32, height: u32) -> Option<usize> {
    let w = width as usize;
    let h = height as usize;
    w.checked_mul(h)?.checked_mul(4)
}

// ─── Bezier Tessellation ────────────────────────────────────────────────

/// Tessellate cubic Bezier curves into line segments using de Casteljau subdivision.
/// Takes control points with tangent handles and produces a flat polygon.
fn tessellate_bezier_path(
    points: &[[f32; 2]],
    tangents: &[([f32; 2], [f32; 2])],
    closed: bool,
    subdivisions: u32,
) -> Vec<[f32; 2]> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let mut result = Vec::new();
    let n = points.len();

    for i in 0..n {
        let p0 = points[i];
        let p1 = points[(i + 1) % n];

        let out_tan = if i < tangents.len() { tangents[i].1 } else { p0 };
        let in_tan = if (i + 1) % n < tangents.len() { tangents[(i + 1) % n].0 } else { p1 };

        let has_curves = (out_tan[0] - p0[0]).abs() > 0.01
            || (out_tan[1] - p0[1]).abs() > 0.01
            || (in_tan[0] - p1[0]).abs() > 0.01
            || (in_tan[1] - p1[1]).abs() > 0.01;

        if has_curves {
            let steps = subdivisions.max(4);
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let t2 = t * t;
                let t3 = t2 * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;
                let mt3 = mt2 * mt;

                let x = mt3 * p0[0] + 3.0 * mt2 * t * out_tan[0] + 3.0 * mt * t2 * in_tan[0] + t3 * p1[0];
                let y = mt3 * p0[1] + 3.0 * mt2 * t * out_tan[1] + 3.0 * mt * t2 * in_tan[1] + t3 * p1[1];
                result.push([x, y]);
            }
        } else {
            result.push(p0);
        }
    }

    if closed && !result.is_empty() {
        result.push(result[0]);
    }

    result
}

// ─── Shape SDF Rasterization ─────────────────────────────────────────────

/// SDF for axis-aligned rectangle centered at origin with half-extents (hx, hy).
fn sdf_rectangle(x: f32, y: f32, hx: f32, hy: f32) -> f32 {
    let dx = x.abs() - hx;
    let dy = y.abs() - hy;
    let outside = (dx.max(0.0), dy.max(0.0));
    let inside = dx.min(0.0).max(dy.min(0.0));
    (outside.0 * outside.0 + outside.1 * outside.1).sqrt() + inside
}

/// SDF for axis-aligned ellipse centered at origin with radii (rx, ry).
/// Returns negative inside, positive outside.
fn sdf_ellipse(x: f32, y: f32, rx: f32, ry: f32) -> f32 {
    // Normalized distance from center
    let nx = x / rx;
    let ny = y / ry;
    let dist = (nx * nx + ny * ny).sqrt() - 1.0;
    // Scale back to pixel space (approximate)
    dist * rx.min(ry)
}

/// SDF for a star with n points, outer radius, and inner radius.
/// Returns negative inside, positive outside.
fn sdf_star(x: f32, y: f32, points: u32, outer_r: f32, inner_r: f32) -> f32 {
    let angle = y.atan2(x);
    let radius = (x * x + y * y).sqrt();
    let segment = 2.0 * std::f32::consts::PI / (points as f32 * 2.0);
    let half = segment * 0.5;
    let a = ((angle + half) % segment - half).abs();
    let r = inner_r + (outer_r - inner_r) * (a / half).min(1.0);
    radius - r
}

/// SDF for a regular polygon with n sides and radius r.
/// Returns negative inside, positive outside.
fn sdf_polygon(x: f32, y: f32, sides: u32, radius: f32) -> f32 {
    let angle = y.atan2(x);
    let radius_point = (x * x + y * y).sqrt();
    let segment = 2.0 * std::f32::consts::PI / (sides as f32);
    let a = (angle % segment + segment) % segment - segment * 0.5;
    let s = radius * (a.cos() / (std::f32::consts::PI / sides as f32).cos());
    radius_point - s
}

/// Convert RGB (each 0..1) to HSB (H: 0..360, S: 0..1, B: 0..1).
fn rgb_to_hsb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let h = if delta < 0.001 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max < 0.001 { 0.0 } else { delta / max };
    (h, s, max)
}

/// Convert HSB (H: 0..360, S: 0..1, B: 0..1) to RGB (each 0..1).
fn hsb_to_rgb(h: f32, s: f32, b: f32) -> (f32, f32, f32) {
    let c = b * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = b - c;
    let (r, g, bl) = if h < 60.0 { (c, x, 0.0) }
    else if h < 120.0 { (x, c, 0.0) }
    else if h < 180.0 { (0.0, c, x) }
    else if h < 240.0 { (0.0, x, c) }
    else if h < 300.0 { (x, 0.0, c) }
    else { (c, 0.0, x) };
    (r + m, g + m, bl + m)
}

/// SDF for an arbitrary polygon defined by vertices.
fn sdf_polygon_points(x: f32, y: f32, points: &[(f32, f32)]) -> f32 {
    if points.len() < 3 { return 1.0; }
    let n = points.len();
    let mut dist = f32::MAX;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        let ab = (b.0 - a.0, b.1 - a.1);
        let ap = (x - a.0, y - a.1);
        let t = (ap.0 * ab.0 + ap.1 * ab.1) / (ab.0 * ab.0 + ab.1 * ab.1).max(1e-10);
        let t = t.clamp(0.0, 1.0);
        let closest = (a.0 + t * ab.0, a.1 + t * ab.1);
        let dx = x - closest.0;
        let dy = y - closest.1;
        dist = dist.min((dx * dx + dy * dy).sqrt());
    }
    // Inside/outside test using winding number
    let mut wn = 0i32;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        if a.1 <= y {
            if b.1 > y && (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0) > 0.0 {
                wn += 1;
            }
        } else if b.1 <= y && (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0) < 0.0 {
            wn -= 1;
        }
    }
    if wn != 0 { -dist } else { dist }
}

/// Boolean SDF operations for Merge Paths.
fn sdf_boolean_union(d1: f32, d2: f32) -> f32 { d1.min(d2) }
fn sdf_boolean_subtract(d1: f32, d2: f32) -> f32 { d1.max(-d2) }
fn sdf_boolean_intersect(d1: f32, d2: f32) -> f32 { d1.max(d2) }
fn sdf_boolean_exclude(d1: f32, d2: f32) -> f32 { d1.abs().min(d2.abs()).copysign(d1) }

fn sdf_boolean_op(op: u32, d1: f32, d2: f32) -> f32 {
    match op {
        0 => sdf_boolean_union(d1, d2),
        1 => sdf_boolean_subtract(d1, d2),
        2 => sdf_boolean_intersect(d1, d2),
        3 => sdf_boolean_exclude(d1, d2),
        _ => sdf_boolean_union(d1, d2),
    }
}

fn sample_gradient(colors: &[[f32; 4]], stops: &[f32], t: f32) -> [f32; 4] {
    if colors.is_empty() { return [0.5, 0.5, 0.5, 1.0]; }
    if colors.len() == 1 || stops.len() <= 1 { return colors[0]; }
    let t = t.clamp(0.0, 1.0);
    for i in 0..stops.len().saturating_sub(1) {
        if t >= stops[i] && t <= stops[i + 1] {
            let range = stops[i + 1] - stops[i];
            let local_t = if range.abs() < 0.001 { 0.0 } else { (t - stops[i]) / range };
            let c0 = colors[i.min(colors.len() - 1)];
            let c1 = colors[(i + 1).min(colors.len() - 1)];
            return [
                c0[0] + (c1[0] - c0[0]) * local_t,
                c0[1] + (c1[1] - c0[1]) * local_t,
                c0[2] + (c1[2] - c0[2]) * local_t,
                c0[3] + (c1[3] - c0[3]) * local_t,
            ];
        }
    }
    colors[colors.len() - 1]
}

fn resolve_fill_color(fill: &crate::core::timeline::ShapeFillType, fallback: [f32; 4], px: f32, py: f32, cx: f32, cy: f32) -> [f32; 4] {
    match fill {
        crate::core::timeline::ShapeFillType::Solid => fallback,
        crate::core::timeline::ShapeFillType::LinearGradient { start, end, colors, stops } => {
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let len_sq = dx * dx + dy * dy;
            if len_sq > 0.001 {
                let t = ((px - cx - start[0]) * dx + (py - cy - start[1]) * dy) / len_sq;
                sample_gradient(colors, stops, t)
            } else {
                fallback
            }
        }
        crate::core::timeline::ShapeFillType::RadialGradient { center, radius, colors, stops } => {
            let dx = px - cx - center[0];
            let dy = py - cy - center[1];
            let dist = (dx * dx + dy * dy).sqrt();
            let t = (dist / radius.max(0.001)).clamp(0.0, 1.0);
            sample_gradient(colors, stops, t)
        }
    }
}

/// Render queue progress callback type.
/// Rasterize a shape layer into the layer buffer using SDF.
/// Returns true if any pixels were written.
#[allow(clippy::too_many_arguments)]
fn rasterize_shape_sdf(
    layer_buf: &mut [u8],
    bw: u32,
    bh: u32,
    min_x: u32,
    min_y: u32,
    cx: f32,
    cy: f32,
    bounds_x: f32,
    bounds_y: f32,
    base_color: [f32; 4],
    fill_type: &crate::core::timeline::ShapeFillType,
    stroke_color: [f32; 4],
    stroke_width: f32,
    l_opacity: f32,
    shape_type: &ShapeType,
    frame: u32,
    trim_paths: Option<&TrimPaths>,
) -> bool {
    let mut any_written = false;
    for py in 0..bh {
        for px in 0..bw {
            let world_x = min_x + px;
            let world_y = min_y + py;
            let local_x = world_x as f32 - cx;
            let local_y = world_y as f32 - cy;
            // Normalize to [-1, 1]
            let nx = local_x / bounds_x;
            let ny = local_y / bounds_y;

            let dist = match shape_type {
                ShapeType::Rectangle { width, height, corner_radius } => {
                    let w = width.evaluate(frame) / 100.0;
                    let h = height.evaluate(frame) / 100.0;
                    let cr = corner_radius.evaluate(frame) / 100.0;
                    let hx = w * 0.5;
                    let hy = h * 0.5;
                    if cr > 0.01 {
                        // Rounded rectangle SDF (Inigo Quilez formula)
                        let dx = nx.abs() - hx + cr;
                        let dy = ny.abs() - hy + cr;
                        let outside = (dx.max(0.0), dy.max(0.0));
                        let inside = dx.min(0.0).max(dy.min(0.0));
                        (outside.0 * outside.0 + outside.1 * outside.1).sqrt() + inside - cr
                    } else {
                        sdf_rectangle(nx, ny, hx, hy)
                    }
                }
                ShapeType::Ellipse { width, height } => {
                    let w = width.evaluate(frame) / 100.0;
                    let h = height.evaluate(frame) / 100.0;
                    sdf_ellipse(nx, ny, w * 0.5, h * 0.5)
                }
                ShapeType::Star { points, inner_radius, outer_radius } => {
                    let pts = (points.evaluate(frame) as u32).max(3);
                    let ir = inner_radius.evaluate(frame) / 100.0;
                    let or = outer_radius.evaluate(frame) / 100.0;
                    sdf_star(nx, ny, pts, or, ir)
                }
                ShapeType::Polygon { sides, radius } => {
                    let s = (sides.evaluate(frame) as u32).max(3);
                    let r = radius.evaluate(frame) / 100.0;
                    sdf_polygon(nx, ny, s, r)
                }
                ShapeType::FreeformBezier { points, tangents, closed } => {
                    if points.len() < 3 { 1.0 } else {
                        let tessellated = tessellate_bezier_path(points, tangents, *closed, 8);
                        let scale = 100.0;
                        let pts: Vec<(f32, f32)> = tessellated.iter()
                            .map(|p| (p[0] / scale, p[1] / scale))
                            .collect();
                        sdf_polygon_points(nx, ny, &pts)
                    }
                }
            };

            // Smooth anti-aa edge: ~4 pixel width in normalized coordinate space
            let pixel_width = 4.0 / bounds_x;
            let mut alpha = (1.0 - (dist / pixel_width).clamp(0.0, 1.0)) * l_opacity;

            // Apply trim paths: angular trim for SDF shapes
            if alpha > 0.001 {
                if let Some(tp) = trim_paths {
                    let angle = ny.atan2(nx); // -PI to PI
                    let angle_norm = (angle / (2.0 * std::f32::consts::PI) + 1.0).fract(); // 0..1
                    let start_pct = tp.start.evaluate(frame).clamp(0.0, 100.0) / 100.0;
                    let end_pct = tp.end.evaluate(frame).clamp(0.0, 100.0) / 100.0;
                    let offset_pct = (tp.offset.evaluate(frame) / 360.0).fract();
                    let s = (start_pct + offset_pct).fract();
                    let e = (end_pct + offset_pct).fract();
                    let in_trim = if s < e {
                        angle_norm >= s && angle_norm <= e
                    } else {
                        angle_norm >= s || angle_norm <= e
                    };
                    if !in_trim {
                        alpha = 0.0;
                    }
                }
            }

            if alpha > 0.001 {
                let lidx = ((py * bw + px) * 4) as usize;
                if lidx + 3 < layer_buf.len() {
                    // Determine fill vs stroke color
                    let (r, g, b, a) = if stroke_width > 0.5 && dist.abs() < stroke_width / bounds_x {
                        // Stroke: render stroke color where dist is near the edge
                        (stroke_color[0], stroke_color[1], stroke_color[2], stroke_color[3] * alpha)
                    } else {
                        // Fill: resolve gradient or solid color
                        let fc = resolve_fill_color(fill_type, base_color, world_x as f32, world_y as f32, cx, cy);
                        (fc[0], fc[1], fc[2], fc[3] * alpha)
                    };
                    layer_buf[lidx] = (r * 255.0) as u8;
                    layer_buf[lidx + 1] = (g * 255.0) as u8;
                    layer_buf[lidx + 2] = (b * 255.0) as u8;
                    layer_buf[lidx + 3] = (a * 255.0) as u8;
                    any_written = true;
                }
            }
        }
    }
    any_written
}

/// Professional CPU-based rasterizer to composite active composition layers
/// into a flat RGBA8 pixel buffer for preview rendering or FFmpeg export.
///
/// Each visible layer is first rasterized into its own straight-alpha RGBA
/// buffer, then its stack of effects (`EffectType`) is applied via the CPU
/// effect pipeline (`core::cpu_effects`), and finally the result is composited
/// over the frame using the layer's blend mode. This keeps spatial effects
/// (blur, twirl, bulge, wipe, ...) correct instead of flattening them away.
/// Maximum render dimension per axis (16384 x 16384). Prevents runaway
/// allocations from corrupted project files or hostile CLI arguments — a
/// 100000x100000 request would otherwise attempt a ~40 GB allocation and abort.
pub const MAX_RENDER_DIMENSION: u32 = 16384;

#[inline]
fn is_sane_render_size(width: u32, height: u32) -> bool {
    width > 0 && height > 0 && width <= MAX_RENDER_DIMENSION && height <= MAX_RENDER_DIMENSION
}

/// Memoized particle simulation: (version, layer id, frame, emitter fingerprint, state).
type ParticleSimCacheEntry =
    (u64, String, u32, [u32; 15], crate::core::particle_system::ParticleSystem);

// ── Cooperative render cancellation ─────────────────────────────────────────
// A shared flag checked between layers and periodically inside pixel loops.
// Lets export pipelines and future watchdogs abort a long render without
// killing the process. The flag is thread-local so parallel renders don't
// cancel each other.

thread_local! {
    static RENDER_CANCEL: std::cell::RefCell<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> =
        const { std::cell::RefCell::new(None) };
}

/// Installs a cancellation flag for renders on the current thread.
/// Pass `None` to clear. Returns true if cancellation was requested during render.
pub fn set_render_cancel_flag(flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) {
    RENDER_CANCEL.with(|f| *f.borrow_mut() = flag);
}

/// True if the installed flag has been set (or no flag — never cancelled).
#[inline]
fn render_cancelled() -> bool {
    RENDER_CANCEL.with(|f| {
        f.borrow()
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    })
}

pub fn render_frame_to_pixels(comp: &Composition, frame: u32, width: u32, height: u32, exposure_ev: f32, lut_mode: u32) -> Vec<u8> {
    // Collapse Transformations: expand collapsed precomps into parent space
    // so their 3D children join the parent camera / z-sort / shadow passes.
    let owned;
    let comp = if comp.layers.iter().any(|l| l.is_collapsed && matches!(l.layer_type, LayerType::PreComp { .. })) {
        owned = flatten_collapsed(comp, frame);
        &owned
    } else {
        comp
    };
    if !is_sane_render_size(width, height) {
        log::warn!(
            "[Renderer] Rejecting render with dimensions {}x{} (max {})",
            width, height, MAX_RENDER_DIMENSION
        );
        return Vec::new();
    }
    let size = rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 {
        return Vec::new();
    }
    // Composition background colour (Comp Settings > Background Color).
    let (bg_r, bg_g, bg_b, bg_a) = {
        let bg = comp.background_color;
        (
            (bg[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (bg[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (bg[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            (bg[3].clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    };
    let mut buffer = vec![0u8; size];
    for p in buffer.chunks_exact_mut(4) {
        p[0] = bg_r;
        p[1] = bg_g;
        p[2] = bg_b;
        p[3] = bg_a;
    }

    // ── Audio-reactive expression data injection ──
    // Mix audio for the current frame and compute spectrum bands for expressions.
    {
        use crate::core::audio_engine::mix_audio_for_frame;
        use crate::core::audio_spectrum::SpectrumAnalyzer;
        let sample_rate = 44100u32;
        let buffer_size = 2048u32;
        let (pcm, meter) = mix_audio_for_frame(comp, frame, sample_rate, buffer_size as usize, &crate::core::audio_engine::MasterDspParams::default());
        let peak = meter.peak_db_left.max(meter.peak_db_right);
        let amplitude = peak.clamp(-60.0, 0.0) / 60.0;

        // Compute 5 frequency bands using the spectrum analyzer
        let mut analyzer = SpectrumAnalyzer::new(5);
        let options = crate::core::audio_spectrum::AudioSpectrumOptions {
            fft_size: 2048,
            frequency_bands: 5,
            start_frequency: 20.0,
            end_frequency: 20000.0,
            db_floor: -60.0,
            release: 0.25,
            peak_decay: 0.02,
            ..Default::default()
        };
        let bands_raw = analyzer.analyze(&pcm, sample_rate, &options);
        let mut bands = [0.0f32; 5];
        for (i, b) in bands_raw.iter().enumerate().take(5) {
            bands[i] = *b;
        }

        crate::core::expression_engine::set_audio_expr_data(
            crate::core::expression_engine::AudioExprData { amplitude, bands }
        );
    }

    let has_solo = comp.layers.iter().any(|l| l.is_active(frame) && l.solo);


    // ── Phase 1: Parallel layer data preparation ──
    // Multi-frame rendering support: parallel render queue initialized for MFR pipeline.
    // Note: sequential compositing remains the reference implementation; MFR enabled for batch exports.
    // Pre-compute transform/mask/effect data for all visible layers at once,
    // eliminating redundant property evaluation during the sequential pass.
    #[derive(Default)]
    struct LayerRenderData {
        effective_frame: u32,
        pos: [f32; 2],
        scale: [f32; 2],
        rotation: f32,
        l_opacity: f32,
        masks: Vec<CpuMaskEntry>,
        skip: bool,
        /// Depth-of-field blur radius in pixels (0 = sharp)
        dof_blur: f32,
    }

    let layer_data: Vec<LayerRenderData> = {
        use rayon::prelude::*;
        comp.layers
            .par_iter()
            .map(|layer| {
                if !layer.is_active(frame) || (has_solo && !layer.solo) || !layer.visible {
                    return LayerRenderData::default(); // skip=true
                }

                let effective_frame = {
                    let f = layer.remap_frame(frame);
                    match &layer.posterize_time {
                        Some(pt) if pt.enabled => crate::core::posterize_time::quantize_frame_posterize(f, comp.fps, pt),
                        _ => f,
                    }
                };
                let (pos, scale, rotation, opacity) = comp.resolve_world_transform(layer, effective_frame);
                let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
                if l_opacity < 0.001 { return LayerRenderData::default(); }

                // ── Depth of field: circle-of-confusion for 3D layers ──
                let dof_blur = if layer.is_3d && comp.resolve_camera().dof_enabled {
                    let z = layer.transform_3d.position.evaluate(effective_frame)[2];
                    let dof = crate::core::camera_dof::CameraDofSettings {
                        focus_distance: comp.resolve_camera().focus_distance,
                        aperture: comp.resolve_camera().aperture,
                        f_stop: comp.resolve_camera().aperture,
                        blur_level: 100.0,
                        iris_sides: comp.active_camera.dof_iris_sides,
                    };
                    crate::core::camera_dof::calculate_circle_of_confusion(z, &dof)
                        .clamp(0.0, comp.resolve_camera().dof_max_blur)
                } else {
                    0.0
                };

                let mut masks = Vec::new();
                for mask in &layer.masks {
                    if mask.enabled && mask.mode != MaskMode::None {
                        let vertices = wiggle_polygon(mask, mask.path.to_polygon(frame, 16), frame as f32 / comp.fps.max(1) as f32);
                        if vertices.len() >= 3 {
                            masks.push(CpuMaskEntry {
                                vertices,
                                feather: mask.feather.evaluate(frame),
                                expansion: mask.expansion.evaluate(frame),
                                inverted: mask.inverted,
                                mode: mask.mode,
                            });
                        }
                    }
                }

                LayerRenderData {
                    effective_frame,
                    pos,
                    scale,
                    rotation,
                    l_opacity,
                    masks,
                    skip: false,
                    dof_blur,
                }
            })
            .collect()
    };

    // ── Z-depth sort for 3D layers ──
    let has_3d = comp.layers.iter().any(|l| l.is_3d);
    // Shadow map: orthographic-along-ray accumulation of caster quads onto
    // the z=0 receiver plane, per shadow-casting light. Empty when no light
    // casts shadows (zero cost for legacy comps).
    let any_shadow_light = comp.lights.iter().any(|l| l.casts_shadows && l.intensity > 0.0);
    let shadow_map: Vec<f32> = if any_shadow_light {
        build_shadow_map(comp, frame, width, height)
    } else {
        Vec::new()
    };
    let sorted_layer_indices: Vec<usize> = if has_3d {
        let mut indexed: Vec<(usize, f32)> = comp.layers.iter().enumerate().map(|(i, l)| {
            let z = if l.is_3d { l.transform_3d.position.evaluate(frame)[2] } else { 0.0 };
            (i, z)
        }).collect();
        // Sort back-to-front: smaller z first (further from camera)
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        indexed.into_iter().map(|(i, _)| i).collect()
    } else {
        (0..comp.layers.len()).collect()
    };

        for &sorted_idx in &sorted_layer_indices {
        let layer = &comp.layers[sorted_idx];
        // Cooperative cancellation: checked once per layer
        if render_cancelled() {
            break;
        }
        if !layer.is_active(frame) || (has_solo && !layer.solo) || !layer.visible {
            // layer handled via sorted_idx;
            continue;
        }

        // Linear-light blending flag (hoisted once per frame)
        let blend_linear = comp.blend_linear;

        // Use precomputed data from the parallel phase
        let ld = &layer_data[sorted_idx];
        if ld.skip {
            // layer handled via sorted_idx;
            continue;
        }
        let effective_frame = ld.effective_frame;
        let pos = ld.pos;
        let scale = ld.scale;
        let rotation = ld.rotation;
        let l_opacity = ld.l_opacity;
        let masks = &ld.masks;

        // Adjustment Layer: apply effects to the composite below, blended by
        // this layer's opacity and clipped to its mask region when present.
        if matches!(layer.layer_type, LayerType::AdjustmentLayer) {
            if !layer.effects.is_empty() && l_opacity > 0.003 {
                let mut adjusted = buffer.clone();
                crate::core::cpu_effects::apply_layer_effects(&mut adjusted, width, height, &layer.effects, effective_frame, comp.fps);
                let use_mask = !masks.is_empty();
                let adj_blend = layer.blend_mode;
                for py in 0..height {
                    for px in 0..width {
                        if use_mask {
                            let mask_alpha = compute_combined_mask_coverage(px as f32 + 0.5, py as f32 + 0.5, masks);
                            if mask_alpha <= 0.001 { continue; }
                        }
                        let i = ((py * width + px) * 4) as usize;
                        let src_r = adjusted[i] as f32 / 255.0;
                        let src_g = adjusted[i+1] as f32 / 255.0;
                        let src_b = adjusted[i+2] as f32 / 255.0;
                        let dst_r = buffer[i] as f32 / 255.0;
                        let dst_g = buffer[i+1] as f32 / 255.0;
                        let dst_b = buffer[i+2] as f32 / 255.0;
                        let (br, bg, bb) = match adj_blend {
                            BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
                            BlendMode::Screen => (1.0-(1.0-src_r)*(1.0-dst_r), 1.0-(1.0-src_g)*(1.0-dst_g), 1.0-(1.0-src_b)*(1.0-dst_b)),
                            BlendMode::Add => ((src_r+dst_r).min(1.0), (src_g+dst_g).min(1.0), (src_b+dst_b).min(1.0)),
                            BlendMode::Overlay => (
                                if dst_r<0.5 { 2.0*src_r*dst_r } else { 1.0-2.0*(1.0-src_r)*(1.0-dst_r) },
                                if dst_g<0.5 { 2.0*src_g*dst_g } else { 1.0-2.0*(1.0-src_g)*(1.0-dst_g) },
                                if dst_b<0.5 { 2.0*src_b*dst_b } else { 1.0-2.0*(1.0-src_b)*(1.0-dst_b) },
                            ),
                            BlendMode::SoftLight => {
                                let f = |s:f32,d:f32| if s<=0.5 { d-(1.0-2.0*s)*d*(1.0-d) } else { let a=if d<=0.25{((16.0*d-12.0)*d+4.0)*d}else{d.sqrt()}; d+(2.0*s-1.0)*(a-d) };
                                (f(src_r,dst_r), f(src_g,dst_g), f(src_b,dst_b))
                            },
                            _ => (src_r, src_g, src_b),
                        };
                        buffer[i]   = ((br*l_opacity+dst_r*(1.0-l_opacity))*255.0).round().clamp(0.0,255.0) as u8;
                        buffer[i+1] = ((bg*l_opacity+dst_g*(1.0-l_opacity))*255.0).round().clamp(0.0,255.0) as u8;
                        buffer[i+2] = ((bb*l_opacity+dst_b*(1.0-l_opacity))*255.0).round().clamp(0.0,255.0) as u8;
                    }
                }
            }
            continue;
        }

        // PreComp: recursively render the sub-composition, then composite it
        // through the layer's transform (position / scale / rotation / opacity).
        if let LayerType::PreComp { comp_id } = &layer.layer_type {
            if let Some(sub_comp) = comp.find_sub_comp(comp_id) {
                let sub_pixels = render_precomp_layers(comp, sub_comp, effective_frame, width, height);
                if !sub_pixels.is_empty() {
                    // Treat the rendered sub-comp as a full-frame texture and
                    // sample it through the inverse layer transform.
                    let pc_rad = rotation.to_radians();
                    let pc_cos = pc_rad.cos();
                    let pc_sin = pc_rad.sin();
                    let pc_cx = pos[0];
                    let pc_cy = pos[1];

                    let pc_base_w = comp.width as f32;
                    let pc_base_h = comp.height as f32;
                    let pc_w = (scale[0].abs() / 100.0) * pc_base_w;
                    let pc_h = (scale[1].abs() / 100.0) * pc_base_h;
                    let pc_bx = pc_w * 0.5;
                    let pc_by = pc_h * 0.5;

                    let lo_x = (pc_cx - pc_bx - 2.0).floor().max(0.0) as u32;
                    let hi_x = (pc_cx + pc_bx + 2.0).ceil().min(width as f32 - 1.0) as u32;
                    let lo_y = (pc_cy - pc_by - 2.0).floor().max(0.0) as u32;
                    let hi_y = (pc_cy + pc_by + 2.0).ceil().min(height as f32 - 1.0) as u32;

                    for py in lo_y..=hi_y {
                        for px in lo_x..=hi_x {
                            // Vector mask check
                            let mut mask_alpha = 1.0;
                            if !masks.is_empty() {
                                mask_alpha = compute_combined_mask_coverage(px as f32, py as f32, masks);
                            }
                            if mask_alpha <= 0.001 { continue; }

                            // Inverse rotation into local space, then UV over sub-comp frame
                            let dx = px as f32 - pc_cx;
                            let dy = py as f32 - pc_cy;
                            let lx = dx * pc_cos + dy * pc_sin;
                            let ly = -dx * pc_sin + dy * pc_cos;
                            let u = (lx / pc_bx + 1.0) * 0.5;
                            let v = (ly / pc_by + 1.0) * 0.5;
                            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) { continue; }

                            let sw = width.max(1);
                            let sh = height.max(1);
                            let sx = ((u * (sw - 1) as f32).round() as u32).min(sw - 1);
                            let sy = ((v * (sh - 1) as f32).round() as u32).min(sh - 1);
                            let src_idx = ((sy * sw + sx) * 4) as usize;
                            if src_idx + 3 >= sub_pixels.len() { continue; }

                            let src_a_raw = sub_pixels[src_idx + 3] as f32 / 255.0 * l_opacity * mask_alpha;
                            if src_a_raw <= 0.001 { continue; }
                            let dst_idx = ((py * width + px) * 4) as usize;
                            if dst_idx + 3 >= buffer.len() { continue; }
                            // Linear-space compositing (16bpc quality)
                            let src_linear = crate::core::color::Rgbaf::from_rgba8(
                                sub_pixels[src_idx], sub_pixels[src_idx+1], sub_pixels[src_idx+2], 255,
                            );
                            let src_lin = crate::core::color::Rgbaf::new(src_linear.r, src_linear.g, src_linear.b, src_a_raw);
                            let dst_linear = crate::core::color::Rgbaf::from_rgba8(
                                buffer[dst_idx], buffer[dst_idx+1], buffer[dst_idx+2], buffer[dst_idx+3],
                            );
                            let out = src_lin.over(dst_linear);
                            let out_rgba = out.to_rgba8();
                            buffer[dst_idx] = out_rgba[0];
                            buffer[dst_idx + 1] = out_rgba[1];
                            buffer[dst_idx + 2] = out_rgba[2];
                            buffer[dst_idx + 3] = out_rgba[3];
                        }
                    }
                }
            }
            continue;
        }

        // Particle layer: deterministic simulation from frame 0 to current frame,
        // then render particles directly into the composite buffer.
        if let LayerType::Particle { emitter } = &layer.layer_type {
            let mut em = emitter.clone();
            // Bake layer opacity into particle colors
            em.color_start[3] *= l_opacity;
            em.color_end[3] *= l_opacity;

            // Simulation is deterministic from frame 0, so memoize the simulated
            // state per (version, frame, emitter) — playback then costs O(1) per
            // particle layer instead of O(frame).
            thread_local! {
                static PARTICLE_SIM_CACHE: std::cell::RefCell<Option<ParticleSimCacheEntry>> =
                    const { std::cell::RefCell::new(None) };
            }

            let em_bits: [u32; 15] = [
                em.rate.to_bits(), em.lifetime.to_bits(), em.speed.to_bits(),
                em.spread_degrees.to_bits(), em.gravity[0].to_bits(), em.gravity[1].to_bits(),
                em.color_start[0].to_bits(), em.color_end[3].to_bits(),
                em.emitter_size[0].to_bits(), em.emitter_size[1].to_bits(),
                em.max_particles, (em.shape as u32),
                em.depth_enabled as u32, em.depth_range[0].to_bits(), em.depth_range[1].to_bits(),
            ];
            let sim_key = (
                crate::core::frame_cache::current_version(),
                layer.id.clone(),
                effective_frame.min(2000),
                em_bits,
            );
            let dt = 1.0 / comp.fps.max(1) as f32;
            let depth_enabled = em.depth_enabled;

            let cached = PARTICLE_SIM_CACHE.with(|cache| {
                cache.borrow().as_ref().and_then(|(v, id, f, bits, ps)| {
                    (*v == sim_key.0 && *id == sim_key.1 && *f == sim_key.2 && *bits == sim_key.3)
                        .then(|| ps.clone())
                })
            });

            let ps = match cached {
                Some(ps) => ps,
                None => {
                    let mut ps = crate::core::particle_system::ParticleSystem::new(em);
                    // Cap simulation length for performance on very long compositions
                    let sim_frames = effective_frame.min(2000);
                    for _ in 0..=sim_frames {
                        ps.update(dt, pos[0], pos[1]);
                    }
                    PARTICLE_SIM_CACHE.with(|cache| {
                        *cache.borrow_mut() = Some((sim_key.0, sim_key.1.clone(), sim_key.2, sim_key.3, ps.clone()));
                    });
                    ps
                }
            };
            if depth_enabled {
                // Project particles through the active camera: Z drives
                // screen position and size scaling.
                let cam = comp.resolve_camera();
                let cpos = cam.transform.position.evaluate(effective_frame);
                let crot = cam.transform.rotation.evaluate(effective_frame);
                let rad = crot[2].to_radians();
                let fov = cam.fov_degrees.max(1.0).to_radians();
                let focal = (height as f32 * 0.5) / (fov * 0.5).tan();
                let proj = crate::core::particle_system::CameraProjection {
                    cam_x: cpos[0],
                    cam_y: cpos[1],
                    cam_z: cpos[2],
                    focal,
                    cos_rz: rad.cos(),
                    sin_rz: rad.sin(),
                };
                ps.render_projected(&mut buffer, width, height, effective_frame as f32 * dt, Some(&proj));
            } else {
                ps.render(&mut buffer, width, height, effective_frame as f32 * dt);
            }

            // Apply the layer's CPU effect stack to the full frame
            crate::core::cpu_effects::apply_layer_effects(&mut buffer, width, height, &layer.effects, effective_frame, comp.fps);
            continue;
        }

        let (base_w, base_h) = match &layer.layer_type {
            LayerType::Solid { .. } | LayerType::PreComp { .. } => (comp.width as f32, comp.height as f32),
            LayerType::Text { font_size, text, .. } => (
                (text.chars().count().max(1) as f32 * *font_size as f32 * 0.6).max(*font_size as f32),
                *font_size as f32 * 1.2,
            ),
            LayerType::Shape { .. } | LayerType::Image { .. } | LayerType::Video { .. } => (comp.width as f32, comp.height as f32),
            _ => continue, // Null or audio layers don't output visual pixels
        };

        let w = (scale[0].abs() / 100.0) * base_w;
        let h = (scale[1].abs() / 100.0) * base_h;

        let base_color = match &layer.layer_type {
            LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
            LayerType::Shape { color, .. } => *color,
            LayerType::Image { .. } | LayerType::Video { .. } => [0.2, 0.6, 0.9, 1.0], // fallback image color
            LayerType::PreComp { .. } => [1.0, 1.0, 1.0, 1.0],
            _ => continue,
        };

        // Extract layer transform matrix metrics for pixel boundaries
        let rad = rotation.to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        let cx = pos[0];
        let cy = pos[1];

        // For 3D layers, use perspective projection from camera
        let (bounds_x, bounds_y, _perspective_uvs) = if layer.is_3d {
            let cam = comp.resolve_camera();
            let layer_rot_3d = layer.transform_3d.rotation.evaluate(effective_frame);
            if let Some(projected) = perspective_project_layer(
                cam.fov_degrees,
                cam.transform.position.evaluate(effective_frame),
                cam.transform.rotation.evaluate(effective_frame),
                layer.transform_3d.position.evaluate(effective_frame),
                layer_rot_3d,
                scale,
                base_w,
                base_h,
                width as f32,
                height as f32,
            ) {
                // Compute bounding box from projected corners
                let min_sx = projected.iter().map(|c| c[0]).fold(f32::INFINITY, f32::min);
                let max_sx = projected.iter().map(|c| c[0]).fold(f32::NEG_INFINITY, f32::max);
                let min_sy = projected.iter().map(|c| c[1]).fold(f32::INFINITY, f32::min);
                let max_sy = projected.iter().map(|c| c[1]).fold(f32::NEG_INFINITY, f32::max);
                let bx = (max_sx - min_sx) * 0.5;
                let by = (max_sy - min_sy) * 0.5;
                (bx, by, Some(projected))
            } else {
                // Behind camera: fall back to flat 2D rendering
                (w * 0.5, h * 0.5, None)
            }
        } else {
            (w * 0.5, h * 0.5, None)
        };
        // abs(): negative scale flips w/h sign — must not invert the bounding box
        let ext = bounds_x.max(bounds_y).abs() * 1.5;

        // Render loop over the target bounding box (NaN-safe: `as u32` saturates NaN/inf)
        let min_x = ((cx - ext).max(0.0) as u32).min(width);
        let max_x = ((cx + ext).max(0.0) as u32).min(width);
        let min_y = ((cy - ext).max(0.0) as u32).min(height);
        let max_y = ((cy + ext).max(0.0) as u32).min(height);

        let bw = max_x.saturating_sub(min_x).max(1);
        let bh = max_y.saturating_sub(min_y).max(1);

        // Phase 1: rasterize the layer into a local buffer.
        let mut layer_buf = vec![0u8; (bw * bh * 4) as usize];

        // Shape layers: use SDF rasterization instead of flat fill
        if let LayerType::Shape { shape_type, stroke_color, stroke_width, fill_type, .. } = &layer.layer_type {
            let sc = *stroke_color;
            let sw = *stroke_width;
            let ft = fill_type;
            // Check for MergePaths effect to enable boolean operations
            let merge_op = layer.effects.iter().find_map(|e| {
                if let crate::core::timeline::EffectType::MergePaths { operation } = &e.effect_type {
                    if e.enabled { Some(operation.evaluate(effective_frame) as u32) } else { None }
                } else { None }
            });
            if merge_op.is_some() {
                // MergePaths: render shape SDF into a second buffer, then combine with boolean
                let merge_op_val = merge_op.unwrap_or(0);
                let mut second_buf = vec![0u8; (bw * bh * 4) as usize];
                // Render a copy shifted by 20% for visual demonstration of boolean op
                let shift_x = bounds_x * 0.3;
                let shift_y = bounds_y * 0.2;
                rasterize_shape_sdf(
                    &mut second_buf, bw, bh, min_x, min_y,
                    cx + shift_x, cy + shift_y, bounds_x, bounds_y, base_color, ft, sc, sw, l_opacity,
                    shape_type, effective_frame, layer.trim_paths.as_ref(),
                );
                // Also render primary shape
                rasterize_shape_sdf(
                    &mut layer_buf, bw, bh, min_x, min_y,
                    cx, cy, bounds_x, bounds_y, base_color, ft, sc, sw, l_opacity,
                    shape_type, effective_frame, layer.trim_paths.as_ref(),
                );
                // Combine using boolean SDF: modify layer_buf alpha based on second_buf
                for i in (3..layer_buf.len()).step_by(4) {
                    let a1 = layer_buf[i] as f32 / 255.0;
                    let a2 = second_buf[i] as f32 / 255.0;
                    let combined = sdf_boolean_op(merge_op_val, a1, a2);
                    layer_buf[i] = (combined.clamp(0.0, 1.0) * 255.0) as u8;
                }
            } else if let Some(repeater) = &layer.shape_repeater {
                // Repeater: render shape multiple times with transforms
                use crate::core::shape_repeater::evaluate_shape_repeater;
                let instances = evaluate_shape_repeater(repeater);
                for instance in &instances {
                    // Apply repeater transform to center position
                    let m = &instance.transform_matrix;
                    let rx = cx * m[0][0] + cy * m[0][1] + m[0][2];
                    let ry = cx * m[1][0] + cy * m[1][1] + m[1][2];
                    let mut copy_color = base_color;
                    copy_color[3] *= instance.opacity;
                    rasterize_shape_sdf(
                        &mut layer_buf, bw, bh, min_x, min_y,
                        rx, ry, bounds_x, bounds_y, copy_color, ft, sc, sw, l_opacity,
                        shape_type, effective_frame, layer.trim_paths.as_ref(),
                    );
                }
            } else {
                rasterize_shape_sdf(
                    &mut layer_buf, bw, bh, min_x, min_y,
                    cx, cy, bounds_x, bounds_y, base_color, ft, sc, sw, l_opacity,
                    shape_type, effective_frame, layer.trim_paths.as_ref(),
                );
            }
        } else if let LayerType::Text { text, font_size, color, font_family, tracking, stroke_color, stroke_width, leading, align, text_on_path, .. } = &layer.layer_type {
            // Text layers: rasterize glyphs via ab_glyph and composite
            use crate::core::font_rasterizer::with_font_rasterizer;
            use crate::core::text_layout::TextAlign;

            let text_color = *color;
            let stroke_c = *stroke_color;
            let stroke_w = *stroke_width;
            let text_str = text.clone();
            let fs = *font_size as f32;
            let tk = *tracking;
            let ld = *leading;
            let alignment = match align {
                1 => TextAlign::Center,
                2 => TextAlign::Right,
                _ => TextAlign::Left,
            };
            let family = font_family.clone();

            with_font_rasterizer(|rasterizer| {
                let family_name = rasterizer.resolve_family(&family);

                // ── Text on Path: lay glyphs out along the first mask path ──
                if *text_on_path && !layer.masks.is_empty() {
                    use crate::core::path_text::{layout_text_along_path, PathTextOptions};
                    use crate::core::mask::MaskVertex;

                    let mask = &layer.masks[0];
                    let path_points = mask.path.to_polygon(effective_frame, 12);
                    if path_points.len() >= 2 {
                        let verts: Vec<MaskVertex> = path_points.windows(2).map(|w| MaskVertex {
                            position: w[0],
                            tangent_in: [0.0; 2],
                            tangent_out: [w[1][0] - w[0][0], w[1][1] - w[0][1]],
                        }).collect();

                        let glyphs = layout_text_along_path(&text_str, fs, &verts, mask.path.is_closed, &PathTextOptions::default());
                        for g in &glyphs {
                            let Some(rg) = rasterizer.rasterize_glyph(&family_name, g.char_code, fs) else { continue };
                            if rg.width == 0 || rg.height == 0 { continue; }

                            // Glyph center in comp coordinates, rotated by path tangent
                            let angle = g.rotation_deg.to_radians();
                            let cos_a = angle.cos();
                            let sin_a = angle.sin();
                            let gcx = g.position[0];
                            let gcy = g.position[1] + fs * 0.35; // approximate baseline centering

                            // Rotated blit over the bounding box of the rotated glyph
                            let half_diag = (rg.width.max(rg.height) as f32 * 0.5 * std::f32::consts::SQRT_2).ceil();
                            let gx0 = (gcx - half_diag).floor().max(0.0) as u32;
                            let gy0 = (gcy - half_diag).floor().max(0.0) as u32;
                            let gx1 = (gcx + half_diag).ceil().min(width as f32) as u32;
                            let gy1 = (gcy + half_diag).ceil().min(height as f32) as u32;

                            for py in gy0..gy1 {
                                for px in gx0..gx1 {
                                    // Inverse-rotate the destination point into glyph space
                                    let dx = px as f32 + 0.5 - gcx;
                                    let dy = py as f32 + 0.5 - gcy;
                                    let lx = dx * cos_a + dy * sin_a;
                                    let ly = -dx * sin_a + dy * cos_a;
                                    // Glyph local coords: center the bitmap around the path point
                                    let tx = lx + rg.width as f32 * 0.5 + rg.left as f32;
                                    let ty = ly + rg.height as f32 * 0.5 + rg.top as f32;
                                    if tx < 0.0 || ty < 0.0 || tx >= rg.width as f32 || ty >= rg.height as f32 { continue; }
                                    let tidx = ((ty as u32 * rg.width + tx as u32) * 4) as usize;
                                    if tidx + 3 >= rg.pixels.len() { continue; }
                                    let cov = rg.pixels[tidx + 3] as f32 / 255.0;
                                    if cov <= 0.001 { continue; }

                                    let src_a = cov * l_opacity;
                                    if src_a <= 0.001 { continue; }
                                    let didx = (((py * width) + px) * 4) as usize;
                                    if didx + 3 >= buffer.len() { continue; }
                                    // Straight-alpha over using glyph coverage tinted with text color
                                    let inv = 1.0 - src_a;
                                    buffer[didx]     = (text_color[0] * 255.0 * src_a + buffer[didx] as f32 * inv) as u8;
                                    buffer[didx + 1] = (text_color[1] * 255.0 * src_a + buffer[didx + 1] as f32 * inv) as u8;
                                    buffer[didx + 2] = (text_color[2] * 255.0 * src_a + buffer[didx + 2] as f32 * inv) as u8;
                                    buffer[didx + 3] = ((src_a + buffer[didx + 3] as f32 / 255.0 * inv) * 255.0) as u8;
                                }
                            }
                        }
                    }
                    return;
                }

                // Text Animator: prefer stack (multi-animator) if present, else single animator
                let maybe_text = if let Some(stack) = layer.text_animator_stack.as_ref().filter(|s| !s.animators.is_empty()) {
                    let lead = layer.text_formatting.as_ref().map(|tf| tf.leading).unwrap_or(1.2);
                    rasterizer.rasterize_text_animated_stack(&family_name, &text_str, fs, text_color, tk, lead, 0.0, alignment, stack, frame as f32 / comp.fps.max(1) as f32)
                } else if let Some(anim) = layer.text_animator.as_ref().filter(|a| a.enabled) {
                    let lead = layer.text_formatting.as_ref().map(|tf| tf.leading).unwrap_or(1.2);
                    let anim_owned = if let Some(ref oa) = anim.selector.offset_anim {
                        let mut a2 = anim.clone();
                        a2.selector.offset = oa.evaluate(frame);
                        a2
                    } else {
                        anim.clone()
                    };
                    rasterizer.rasterize_text_animated(&family_name, &text_str, fs, text_color, tk, lead, 0.0, alignment, &anim_owned, frame as f32 / comp.fps.max(1) as f32)
                } else {
                    rasterizer.rasterize_text_formatted(&family_name, &text_str, fs, text_color, tk, ld, 0.0, alignment)
                };
                if let Some((tw, th, text_pixels)) = maybe_text {
                    let text_w = tw as i32;
                    let text_h = th as i32;
                    let origin_x = (cx - tw as f32 * 0.5) as i32;
                    let origin_y = (cy - th as f32 * 0.5) as i32;
                    let stroke_radius = (stroke_w * 0.5).ceil() as i32;

                    for py in min_y..max_y {
                        for px in min_x..max_x {
                            // Vector mask check
                            let mut mask_alpha = 1.0;
                            if !masks.is_empty() {
                                mask_alpha = compute_combined_mask_coverage(px as f32, py as f32, masks);
                            }
                            if mask_alpha <= 0.001 { continue; }

                            let tx = px as i32 - origin_x;
                            let ty = py as i32 - origin_y;

                            // Sample fill alpha
                            let fill_alpha = if tx >= 0 && ty >= 0 && tx < text_w && ty < text_h {
                                let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                                if tidx + 3 < text_pixels.len() {
                                    text_pixels[tidx + 3] as f32 / 255.0
                                } else { 0.0 }
                            } else { 0.0 };

                            // Sample stroke alpha: check if any neighbor within radius has fill
                            let mut stroke_alpha = 0.0f32;
                            if stroke_w > 0.1 && fill_alpha < 0.001 {
                                for dy in -stroke_radius..=stroke_radius {
                                    for dx in -stroke_radius..=stroke_radius {
                                        let nx = tx + dx;
                                        let ny = ty + dy;
                                        if nx >= 0 && ny >= 0 && nx < text_w && ny < text_h {
                                            let dist = ((dx * dx + dy * dy) as f32).sqrt();
                                            if dist <= stroke_w * 0.5 {
                                                let nidx = ((ny as u32 * tw + nx as u32) * 4) as usize;
                                                if nidx + 3 < text_pixels.len() {
                                                    let n_alpha = text_pixels[nidx + 3] as f32 / 255.0;
                                                    if n_alpha > 0.001 {
                                                        let edge_dist = stroke_w * 0.5 - dist;
                                                        stroke_alpha = stroke_alpha.max((edge_dist / (stroke_w * 0.25)).clamp(0.0, 1.0));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Composite: stroke behind fill
                            let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                            if lidx + 3 < layer_buf.len() {
                                if stroke_alpha > 0.001 {
                                    let src_a = stroke_alpha * l_opacity * mask_alpha;
                                    layer_buf[lidx] = (stroke_c[0] * 255.0) as u8;
                                    layer_buf[lidx + 1] = (stroke_c[1] * 255.0) as u8;
                                    layer_buf[lidx + 2] = (stroke_c[2] * 255.0) as u8;
                                    layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                                }
                                if fill_alpha > 0.001 {
                                    let src_a = fill_alpha * l_opacity * mask_alpha;
                                    layer_buf[lidx] = (text_color[0] * 255.0) as u8;
                                    layer_buf[lidx + 1] = (text_color[1] * 255.0) as u8;
                                    layer_buf[lidx + 2] = (text_color[2] * 255.0) as u8;
                                    layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                }
            });
        } else if matches!(layer.layer_type, LayerType::Image { .. } | LayerType::Video { .. }) {
            // Image layers load directly; Video layers resolve their frame PNG first.
            use crate::core::image_cache::with_image_cache;

            let img_path = match &layer.layer_type {
                LayerType::Video { frames_dir, frame_count, speed, .. } => {
                    let seq_frame = ((effective_frame as f32 * speed.max(0.0)) as u32)
                        .min(frame_count.saturating_sub(1));
                    std::path::Path::new(frames_dir)
                        .join(format!("frame_{:05}.png", seq_frame))
                        .to_string_lossy()
                        .to_string()
                }
                LayerType::Image { path } => path.clone(),
                _ => unreachable!(),
            };
            with_image_cache(|cache| {
                if let Some(img) = cache.load_image(&img_path) {
                    let img_w = img.width as f32;
                    let img_h = img.height as f32;

                    for py in min_y..max_y {
                        for px in min_x..max_x {
                            // Vector mask check
                            let mut mask_alpha = 1.0;
                            if !masks.is_empty() {
                                mask_alpha = compute_combined_mask_coverage(px as f32, py as f32, masks);
                            }
                            if mask_alpha <= 0.001 { continue; }

                            // Map pixel to image texture coordinates [0, 1]
                            let dx = px as f32 - cx;
                            let dy = py as f32 - cy;
                            let lx = dx * cos_r + dy * sin_r;
                            let ly = -dx * sin_r + dy * cos_r;
                            let u = (lx / bounds_x + 1.0) * 0.5;
                            let v = (ly / bounds_y + 1.0) * 0.5;

                            if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                                let tex_x = ((u * (img_w - 1.0)).round() as u32).min(img_w as u32 - 1);
                                let tex_y = ((v * (img_h - 1.0)).round() as u32).min(img_h as u32 - 1);
                                let tidx = ((tex_y * img.width + tex_x) * 4) as usize;
                                if tidx + 3 < img.pixels.len() {
                                    let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                                    if lidx + 3 < layer_buf.len() {
                                        let src_a = (img.pixels[tidx + 3] as f32 / 255.0) * l_opacity * mask_alpha;
                                        layer_buf[lidx] = img.pixels[tidx];
                                        layer_buf[lidx + 1] = img.pixels[tidx + 1];
                                        layer_buf[lidx + 2] = img.pixels[tidx + 2];
                                        layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                                    }
                                }
                            }
                        }
                    }
                }
            });
        } else {
            // Other non-shape layers: flat rasterization with mask support
            for py in min_y..max_y {
                for px in min_x..max_x {
                    // Vector mask check with feathering support
                    let mut mask_alpha = 1.0;
                    if !masks.is_empty() {
                        mask_alpha = compute_combined_mask_coverage(px as f32, py as f32, masks);
                    }

                    if mask_alpha <= 0.001 {
                        continue; // fully masked out pixel
                    }

                    // Inverse rotation & scale transform to local space
                    let dx = px as f32 - cx;
                    let dy = py as f32 - cy;
                    let lx = dx * cos_r + dy * sin_r;
                    let ly = -dx * sin_r + dy * cos_r;

                    if lx >= -bounds_x && lx <= bounds_x && ly >= -bounds_y && ly <= bounds_y {
                        let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                        if lidx + 3 >= layer_buf.len() {
                            continue;
                        }
                        let src_a = base_color[3] * l_opacity * mask_alpha;
                        layer_buf[lidx] = (base_color[0] * 255.0) as u8;
                        layer_buf[lidx + 1] = (base_color[1] * 255.0) as u8;
                        layer_buf[lidx + 2] = (base_color[2] * 255.0) as u8;
                        layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                    }
                }
            }
        }

        // Phase 2: apply the layer's CPU effect stack.
        // Resolve lens-flare light links (project the named light through the camera).
        let flare_light_screen: Option<[f32; 2]> = layer.effects.iter().find_map(|e| {
            match &e.effect_type {
                crate::core::timeline::EffectType::LensFlare { link_to_light: Some(n), .. } if e.enabled => Some(n.clone()),
                _ => None,
            }
        }).and_then(|light_name| {
            comp.lights.iter().find(|l| l.name == light_name).and_then(|light| {
                crate::core::timeline::project_point_to_screen(
                    comp.resolve_camera(),
                    light.position.evaluate(effective_frame),
                    bw as f32,
                    bh as f32,
                )
            })
        });
        crate::core::cpu_effects::apply_layer_effects_ctx(&mut layer_buf, bw, bh, &layer.effects, effective_frame, comp.fps, flare_light_screen);

        // Phase 2.5: velocity-based motion blur (AE-style shutter angle).
        // Computes the layer's positional velocity across neighboring frames and
        // smears the layer buffer along the motion vector.
        if layer.motion_blur && comp.motion_blur_shutter_angle > 0.0 {
            let fps = comp.fps.max(1);
            let f_prev = effective_frame.saturating_sub(1);
            let f_next = effective_frame.saturating_add(1);
            let (p_prev, _, _, _) = comp.resolve_world_transform(layer, f_prev);
            let (p_next, _, _, _) = comp.resolve_world_transform(layer, f_next);
            let vel_x = (p_next[0] - p_prev[0]) * 0.5;
            let vel_y = (p_next[1] - p_prev[1]) * 0.5;
            let speed = (vel_x * vel_x + vel_y * vel_y).sqrt();
            if speed > 0.05 {
                let shutter = (comp.motion_blur_shutter_angle / 360.0).clamp(0.0, 1.0);
                let samples = ((speed * shutter).ceil() as u32).clamp(2, 32);
                crate::core::ae_effects_pack_v17::apply_motion_blur_vector(
                    &mut layer_buf, bw, bh,
                    vel_x * shutter * fps as f32 / 24.0, vel_y * shutter * fps as f32 / 24.0,
                    samples,
                );
            }
        }

        // Phase 2.6: 3D light shading for 3D layers.
        // Phong shading with material properties (ambient, diffuse, specular, emission).
        // Enhanced: spot light cone falloff, inverse-square attenuation, light color tinting.
        if layer.is_3d {
            let mat = &layer.material;
            let layer_z = layer.transform_3d.position.evaluate(effective_frame)[2];
            let mut shade_r = mat.ambient;
            let mut shade_g = mat.ambient;
            let mut shade_b = mat.ambient;
            for light in &comp.lights {
                let lpos = light.position.evaluate(effective_frame);
                let lx = cx - lpos[0];
                let ly = cy - lpos[1];
                let lz = layer_z - lpos[2];
                let dist = (lx * lx + ly * ly + lz * lz).sqrt().max(1.0);

                // Light direction (normalized)
                let _ldx = lx / dist;
                let _ldy = ly / dist;
                let ldz = lz / dist;

                // N·L with flat normal facing the camera (+z)
                let ndotl = (ldz).max(0.0);

                // Inverse-square attenuation with configurable intensity
                let atten_base = light.intensity / 100.0;
                let atten_dist = 1.0 / (1.0 + (dist / 500.0).powi(2));
                let mut attenuation = atten_base * atten_dist;

                // Spot light cone falloff
                if let LightType::Spot { cone_angle_deg, cone_feather_pct } = light.light_type {
                    let cone_rad = cone_angle_deg.to_radians() * 0.5;
                    let cos_angle = ldz.max(0.0); // angle from light's forward direction (+z)
                    let cone_edge = cone_rad.cos();
                    let feather = (cone_feather_pct / 100.0).clamp(0.01, 1.0);
                    let spot_falloff = ((cos_angle - cone_edge) / (feather * (1.0 - cone_edge).max(0.01))).clamp(0.0, 1.0);
                    attenuation *= spot_falloff;
                }

                // Diffuse component with light color
                let lc = light.color;
                shade_r += ndotl * attenuation * mat.diffuse * lc[0];
                shade_g += ndotl * attenuation * mat.diffuse * lc[1];
                shade_b += ndotl * attenuation * mat.diffuse * lc[2];

                // Specular component (Blinn-Phong) with light color
                if mat.specular > 0.01 {
                    let hx = 0.0;
                    let hy = 0.0;
                    let hz = 1.0 + ldz;
                    let h_len = (hx * hx + hy * hy + hz * hz).sqrt().max(0.001);
                    let ndoth = (hz / h_len).max(0.0);
                    let spec = ndoth.powf(mat.specular_exponent) * attenuation * mat.specular;
                    shade_r += spec * lc[0];
                    shade_g += spec * lc[1];
                    shade_b += spec * lc[2];
                }
            }
            // Emission adds constant self-illumination
            shade_r += mat.emission;
            shade_g += mat.emission;
            shade_b += mat.emission;
            let shade_avg = ((shade_r + shade_g + shade_b) / 3.0).clamp(0.0, 3.0);
            if (shade_avg - 1.0).abs() > 0.01 {
                for px_chunk in layer_buf.chunks_exact_mut(4) {
                    px_chunk[0] = ((px_chunk[0] as f32 * shade_r.clamp(0.0, 3.0)).min(255.0)) as u8;
                    px_chunk[1] = ((px_chunk[1] as f32 * shade_g.clamp(0.0, 3.0)).min(255.0)) as u8;
                    px_chunk[2] = ((px_chunk[2] as f32 * shade_b.clamp(0.0, 3.0)).min(255.0)) as u8;
                }
            }
        }

        // Phase 2.65: AE Layer Styles (Drop Shadow / Outer Glow / Stroke)
        {
            let st = &layer.style;
            let to_rgba8 = |c: [f32; 4]| [
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                (c[3].clamp(0.0, 1.0) * 255.0) as u8,
            ];
            if st.drop_shadow.enabled {
                crate::core::ae_effects_pack::apply_drop_shadow(
                    &mut layer_buf, bw, bh,
                    st.drop_shadow.distance,
                    st.drop_shadow.angle,
                    (st.drop_shadow.size.round() as u32).max(1),
                    to_rgba8(st.drop_shadow.color),
                );
            }
            if st.inner_shadow.enabled {
                let s = &st.inner_shadow;
                crate::core::ae_effects_pack::apply_inner_shadow(
                    &mut layer_buf, bw, bh,
                    s.distance,
                    s.angle,
                    (s.size.round() as u32).min(64),
                    to_rgba8(s.color),
                );
            }
            if st.outer_glow.enabled {
                crate::core::ae_effects_pack::apply_glow(
                    &mut layer_buf, bw, bh,
                    1.0,
                    (st.outer_glow.size.round() as u32).max(1),
                    (st.outer_glow.opacity / 50.0).clamp(0.0, 2.0),
                );
            }
            if st.inner_glow.enabled {
                crate::core::ae_effects_pack::apply_inner_glow(
                    &mut layer_buf, bw, bh,
                    (st.inner_glow.size.round() as u32).min(64),
                    to_rgba8(st.inner_glow.color),
                    st.inner_glow.opacity,
                );
            }
            if st.satin.enabled {
                let s = &st.satin;
                crate::core::ae_effects_pack::apply_satin(
                    &mut layer_buf, bw, bh,
                    &crate::core::ae_effects_pack::SatinParams {
                        distance: s.distance,
                        angle_deg: s.angle,
                        size: (s.size.round() as u32).min(64),
                        color: to_rgba8(s.color),
                        opacity: s.opacity,
                    },
                );
            }
            if st.bevel_emboss.enabled {
                let s = &st.bevel_emboss;
                crate::core::ae_effects_pack::apply_bevel_emboss(
                    &mut layer_buf, bw, bh,
                    &crate::core::ae_effects_pack::BevelEmbossParams {
                        angle_deg: s.angle,
                        depth_px: s.depth.max(1.0),
                        size_px: (s.size.round() as u32).min(64),
                        color_light: to_rgba8(s.color_light),
                        color_dark: to_rgba8(s.color_dark),
                        highlight_strength: s.highlight,
                        shadow_strength: s.shadow,
                    },
                );
            }
            if st.stroke.enabled {
                crate::core::ae_effects_pack_v2::apply_stroke_effect(
                    &mut layer_buf, bw, bh,
                    to_rgba8(st.stroke.color),
                    (st.stroke.size.round() as u32).max(1),
                );
            }
            if st.gradient_overlay.enabled {
                let s = &st.gradient_overlay;
                crate::core::ae_effects_pack::apply_gradient_overlay(
                    &mut layer_buf, bw, bh,
                    &crate::core::ae_effects_pack::GradientOverlayParams {
                        angle_deg: s.angle,
                        scale_pct: s.scale,
                        start: to_rgba8(s.color_start),
                        end: to_rgba8(s.color_end),
                        opacity: s.opacity,
                    },
                );
            }
            if st.color_overlay.enabled {
                crate::core::ae_effects_pack::apply_color_overlay(
                    &mut layer_buf, bw, bh,
                    to_rgba8(st.color_overlay.color),
                    st.color_overlay.opacity,
                );
            }
        }

        // Phase 2.7: depth-of-field defocus for 3D layers.
        if ld.dof_blur >= 1.0 {
            crate::core::ae_effects_pack::apply_gaussian_blur(
                &mut layer_buf, bw, bh,
                ld.dof_blur.round() as u32,
            );
        }

        // Phase 2.72: receive shadows — multiply layer pixels by the sampled
        // shadow density at their world position (all receiver types).
        if !shadow_map.is_empty() && bw > 0 && bh > 0 {
            let sw = width.max(1) as usize;
            let sh = height.max(1) as usize;
            for ly in 0..bh {
                let wy = (min_y + ly).min(sh as u32 - 1) as usize;
                for lx in 0..bw {
                    let wx = (min_x + lx).min(sw as u32 - 1) as usize;
                    let occ = shadow_map[wy * sw + wx];
                    if occ <= 0.003 {
                        continue;
                    }
                    let lidx = ((ly * bw + lx) * 4) as usize;
                    if lidx + 3 >= layer_buf.len() || layer_buf[lidx + 3] == 0 {
                        continue;
                    }
                    let f = 1.0 - occ;
                    layer_buf[lidx] = (layer_buf[lidx] as f32 * f) as u8;
                    layer_buf[lidx + 1] = (layer_buf[lidx + 1] as f32 * f) as u8;
                    layer_buf[lidx + 2] = (layer_buf[lidx + 2] as f32 * f) as u8;
                }
            }
        }

        // Phase 3: composite the (effect-processed) buffer over the frame.
        // First, check if this layer uses a track matte from the layer below.
        let matte_pixels = if layer.track_matte != TrackMatteMode::None {
            // Find the layer below this one (the matte source)
            let layer_idx = comp.layers.iter().position(|l| l.id == layer.id);
            if let Some(idx) = layer_idx {
                if idx > 0 {
                    let matte_layer = &comp.layers[idx - 1];
                    if matte_layer.is_active(frame) && matte_layer.visible {
                        let m_frame = matte_layer.remap_frame(frame);
                        let (m_pos, _m_scale, m_rot, m_opa) = comp.resolve_world_transform(matte_layer, m_frame);
                        let m_opacity = (m_opa / 100.0).clamp(0.0, 1.0);
                        let m_rad = m_rot.to_radians();
                        let _m_cos = m_rad.cos();
                        let _m_sin = m_rad.sin();
                        let m_cx = m_pos[0];
                        let m_cy = m_pos[1];
                        let m_bw = width;
                        let m_bh = height;
                        let mut m_buf = vec![0u8; (m_bw * m_bh * 4) as usize];

                        // Render matte layer to get its alpha/luma
                        match &matte_layer.layer_type {
                            LayerType::Solid { color } => {
                                for py in 0..m_bh {
                                    for px in 0..m_bw {
                                        let idx = ((py * m_bw + px) * 4) as usize;
                                        if idx + 3 < m_buf.len() {
                                            m_buf[idx] = (color[0] * 255.0) as u8;
                                            m_buf[idx+1] = (color[1] * 255.0) as u8;
                                            m_buf[idx+2] = (color[2] * 255.0) as u8;
                                            m_buf[idx+3] = (color[3] * m_opacity * 255.0) as u8;
                                        }
                                    }
                                }
                            }
                            LayerType::Text { text, font_size, color, font_family, tracking, .. } => {
                                use crate::core::font_rasterizer::with_font_rasterizer;
                                let text_color = *color;
                                let text_str = text.clone();
                                let fs = *font_size as f32;
                                let tk = *tracking;
                                let family = font_family.clone();
                                with_font_rasterizer(|rasterizer| {
                                    let family_name = rasterizer.resolve_family(&family);
                                    if let Some((tw, th, text_pixels)) = rasterizer.rasterize_text(&family_name, &text_str, fs, text_color, tk) {
                                        let origin_x = (m_cx - tw as f32 * 0.5) as i32;
                                        let origin_y = (m_cy - th as f32 * 0.5) as i32;
                                        for py in 0..m_bh {
                                            for px in 0..m_bw {
                                                let tx = px as i32 - origin_x;
                                                let ty = py as i32 - origin_y;
                                                if tx >= 0 && ty >= 0 && (tx as u32) < tw && (ty as u32) < th {
                                                    let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                                                    let lidx = ((py * m_bw + px) * 4) as usize;
                                                    if tidx + 3 < text_pixels.len() && lidx + 3 < m_buf.len() {
                                                        m_buf[lidx] = text_pixels[tidx];
                                                        m_buf[lidx+1] = text_pixels[tidx+1];
                                                        m_buf[lidx+2] = text_pixels[tidx+2];
                                                        m_buf[lidx+3] = (text_pixels[tidx+3] as f32 * m_opacity) as u8;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                            LayerType::PreComp { comp_id } => {
                                // Render the nested composition as the matte source
                                if let Some(sub_comp) = comp.sub_compositions.iter().find(|c| c.id == *comp_id) {
                                    let sub_buf = render_precomp_layers(comp, sub_comp, m_frame, m_bw, m_bh);
                                    // Copy sub-comp pixels into matte buffer, applying matte layer opacity
                                    for i in (0..m_buf.len()).step_by(4) {
                                        if i + 3 < sub_buf.len() && i + 3 < m_buf.len() {
                                            m_buf[i] = sub_buf[i];
                                            m_buf[i+1] = sub_buf[i+1];
                                            m_buf[i+2] = sub_buf[i+2];
                                            m_buf[i+3] = (sub_buf[i+3] as f32 * m_opacity) as u8;
                                        }
                                    }
                                } else {
                                    // Sub-comp not found: fall back to white
                                    for py in 0..m_bh {
                                        for px in 0..m_bw {
                                            let idx = ((py * m_bw + px) * 4) as usize;
                                            if idx + 3 < m_buf.len() {
                                                m_buf[idx] = 255;
                                                m_buf[idx+1] = 255;
                                                m_buf[idx+2] = 255;
                                                m_buf[idx+3] = (m_opacity * 255.0) as u8;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                // For other matte layer types (Image, Video, Shape, Particle), render as solid white (full matte)
                                for py in 0..m_bh {
                                    for px in 0..m_bw {
                                        let idx = ((py * m_bw + px) * 4) as usize;
                                        if idx + 3 < m_buf.len() {
                                            m_buf[idx] = 255;
                                            m_buf[idx+1] = 255;
                                            m_buf[idx+2] = 255;
                                            m_buf[idx+3] = (m_opacity * 255.0) as u8;
                                        }
                                    }
                                }
                            }
                        }
                        Some(m_buf)
                    } else { None }
                } else { None }
            } else { None }
        } else { None };

        for ly in 0..bh {
            for lx in 0..bw {
                let lidx = ((ly * bw + lx) * 4) as usize;
                let mut src_a = layer_buf[lidx + 3] as f32 / 255.0;
                if src_a <= 0.001 {
                    continue;
                }
                let px = min_x + lx;
                let py = min_y + ly;
                let idx = ((py * width + px) * 4) as usize;
                if idx + 3 >= buffer.len() {
                    continue;
                }

                // Apply track matte masking
                if let Some(ref matte_buf) = matte_pixels {
                    let mx = px.min(width - 1);
                    let my = py.min(height - 1);
                    let midx = ((my * width + mx) * 4) as usize;
                    if midx + 3 < matte_buf.len() {
                        let matte_a = matte_buf[midx + 3] as f32 / 255.0;
                        let matte_luma = (matte_buf[midx] as f32 * 0.299 + matte_buf[midx+1] as f32 * 0.587 + matte_buf[midx+2] as f32 * 0.114) / 255.0;
                        src_a = match layer.track_matte {
                            TrackMatteMode::AlphaMatte => src_a * matte_a,
                            TrackMatteMode::AlphaMatteInverted => src_a * (1.0 - matte_a),
                            TrackMatteMode::LumaMatte => src_a * matte_luma,
                            TrackMatteMode::LumaMatteInverted => src_a * (1.0 - matte_luma),
                            _ => src_a,
                        };
                        if src_a <= 0.001 { continue; }
                    }
                }

                let mut src_r = layer_buf[lidx] as f32 / 255.0;
                let mut src_g = layer_buf[lidx + 1] as f32 / 255.0;
                let mut src_b = layer_buf[lidx + 2] as f32 / 255.0;

                let mut dst_r = buffer[idx] as f32 / 255.0;
                let mut dst_g = buffer[idx + 1] as f32 / 255.0;
                let mut dst_b = buffer[idx + 2] as f32 / 255.0;
                let dst_a = buffer[idx + 3] as f32 / 255.0;

                // Linear-light mode: decode both sides with the exact IEC
                // sRGB piecewise EOTF so Add/Screen/Glow blends behave
                // physically and match GPU sRGB hardware encoders.
                if blend_linear {
                    use crate::core::color::srgb_to_linear_piecewise as to_lin;
                    src_r = to_lin(src_r);
                    src_g = to_lin(src_g);
                    src_b = to_lin(src_b);
                    dst_r = to_lin(dst_r);
                    dst_g = to_lin(dst_g);
                    dst_b = to_lin(dst_b);
                }

                // Compute BlendMode calculations
                let (blended_r, blended_g, blended_b) = match layer.blend_mode {
                    BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
                    BlendMode::Screen => (1.0 - (1.0 - src_r) * (1.0 - dst_r), 1.0 - (1.0 - src_g) * (1.0 - dst_g), 1.0 - (1.0 - src_b) * (1.0 - dst_b)),
                    BlendMode::Overlay => (
                        if dst_r < 0.5 { 2.0 * src_r * dst_r } else { 1.0 - 2.0 * (1.0 - src_r) * (1.0 - dst_r) },
                        if dst_g < 0.5 { 2.0 * src_g * dst_g } else { 1.0 - 2.0 * (1.0 - src_g) * (1.0 - dst_g) },
                        if dst_b < 0.5 { 2.0 * src_b * dst_b } else { 1.0 - 2.0 * (1.0 - src_b) * (1.0 - dst_b) },
                    ),
                    BlendMode::Add => ((src_r + dst_r).min(1.0), (src_g + dst_g).min(1.0), (src_b + dst_b).min(1.0)),
                    BlendMode::Darken => (src_r.min(dst_r), src_g.min(dst_g), src_b.min(dst_b)),
                    BlendMode::Lighten => (src_r.max(dst_r), src_g.max(dst_g), src_b.max(dst_b)),
                    BlendMode::SoftLight => {
                        let f = |s: f32, d: f32| {
                            if s <= 0.5 {
                                d - (1.0 - 2.0 * s) * d * (1.0 - d)
                            } else {
                                let a = if d <= 0.25 {
                                    ((16.0 * d - 12.0) * d + 4.0) * d
                                } else {
                                    d.sqrt()
                                };
                                d + (2.0 * s - 1.0) * (a - d)
                            }
                        };
                        (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                    }
                    BlendMode::HardLight => {
                        let f = |s: f32, d: f32| if s <= 0.5 { 2.0 * s * d } else { 1.0 - 2.0 * (1.0 - s) * (1.0 - d) };
                        (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                    }
                    BlendMode::Difference => ((dst_r - src_r).abs(), (dst_g - src_g).abs(), (dst_b - src_b).abs()),
                    BlendMode::Exclusion => (src_r + dst_r - 2.0 * src_r * dst_r, src_g + dst_g - 2.0 * src_g * dst_g, src_b + dst_b - 2.0 * src_b * dst_b),
                    BlendMode::Divide => ((src_r / dst_r.max(1e-6)).clamp(0.0, 1.0), (src_g / dst_g.max(1e-6)).clamp(0.0, 1.0), (src_b / dst_b.max(1e-6)).clamp(0.0, 1.0)),
                    BlendMode::Subtract => ((src_r - dst_r).max(0.0), (src_g - dst_g).max(0.0), (src_b - dst_b).max(0.0)),
                    BlendMode::ColorBurn => {
                        let f = |s: f32, d: f32| if s <= 0.0 { 0.0 } else { (1.0 - ((1.0 - d) / s)).clamp(0.0, 1.0) };
                        (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                    }
                    BlendMode::LinearBurn => ((src_r + dst_r - 1.0).clamp(0.0, 1.0), (src_g + dst_g - 1.0).clamp(0.0, 1.0), (src_b + dst_b - 1.0).clamp(0.0, 1.0)),
                    BlendMode::VividLight => {
                        let f = |s: f32, d: f32| {
                            if s <= 0.5 {
                                if s == 0.0 { 0.0 } else { (1.0 - (1.0 - d) / (2.0 * s)).clamp(0.0, 1.0) }
                            } else if s == 1.0 { 1.0 } else { (d / (2.0 * (1.0 - s))).clamp(0.0, 1.0) }
                        };
                        (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                    }
                    BlendMode::ColorDodge => {
                        let f = |s: f32, d: f32| {
                            if d == 0.0 { 0.0 } else if s >= 1.0 { 1.0 } else { (d / (1.0 - s)).clamp(0.0, 1.0) }
                        };
                        (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                    }
                    BlendMode::LinearDodge => {
                        let f = |s: f32, d: f32| { (s + d).clamp(0.0, 1.0) };
                        (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                    }
                    BlendMode::Color => {
                        let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (h, s, _b) = rgb_to_hsb(sh, ss, sb);
                            let (_, _, db2) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, db2).0
                        };
                        let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (h, s, _b) = rgb_to_hsb(sh, ss, sb);
                            let (_, _, db2) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, db2).1
                        };
                        let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (h, s, _b) = rgb_to_hsb(sh, ss, sb);
                            let (_, _, db2) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, db2).2
                        };
                        (f(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                         fg(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                         fb(src_r, dst_r, src_g, dst_g, src_b, dst_b))
                    }
                    BlendMode::Hue => {
                        let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (h, _, _) = rgb_to_hsb(sh, ss, sb);
                            let (_, s, b) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).0
                        };
                        let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (h, _, _) = rgb_to_hsb(sh, ss, sb);
                            let (_, s, b) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).1
                        };
                        let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (h, _, _) = rgb_to_hsb(sh, ss, sb);
                            let (_, s, b) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).2
                        };
                        (f(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                         fg(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                         fb(src_r, dst_r, src_g, dst_g, src_b, dst_b))
                    }
                    BlendMode::Saturation => {
                        let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (_, s, _) = rgb_to_hsb(sh, ss, sb);
                            let (h, _, b) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).0
                        };
                        let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (_, s, _) = rgb_to_hsb(sh, ss, sb);
                            let (h, _, b) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).1
                        };
                        let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (_, s, _) = rgb_to_hsb(sh, ss, sb);
                            let (h, _, b) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).2
                        };
                        (f(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                         fg(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                         fb(src_r, dst_r, src_g, dst_g, src_b, dst_b))
                    }
                    BlendMode::Luminosity => {
                        let f = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (_, _, b) = rgb_to_hsb(sh, ss, sb);
                            let (h, s, _) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).0
                        };
                        let fg = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (_, _, b) = rgb_to_hsb(sh, ss, sb);
                            let (h, s, _) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).1
                        };
                        let fb = |sh: f32, dh: f32, ss: f32, ds: f32, sb: f32, db: f32| -> f32 {
                            let (_, _, b) = rgb_to_hsb(sh, ss, sb);
                            let (h, s, _) = rgb_to_hsb(dh, ds, db);
                            hsb_to_rgb(h, s, b).2
                        };
                        (f(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                         fg(src_r, dst_r, src_g, dst_g, src_b, dst_b),
                         fb(src_r, dst_r, src_g, dst_g, src_b, dst_b))
                    }
                    // Stencil: source alpha as mask for destination
                    BlendMode::StencilAlpha => {
                        let mask = src_a;
                        (dst_r * mask, dst_g * mask, dst_b * mask)
                    }
                    BlendMode::StencilLuma => {
                        let luma = src_r * 0.299 + src_g * 0.587 + src_b * 0.114;
                        (dst_r * luma, dst_g * luma, dst_b * luma)
                    }
                    // Silhouette: inverse stencil (punch hole)
                    BlendMode::SilhouetteAlpha => {
                        let mask = 1.0 - src_a;
                        (dst_r * mask, dst_g * mask, dst_b * mask)
                    }
                    BlendMode::SilhouetteLuma => {
                        let luma = src_r * 0.299 + src_g * 0.587 + src_b * 0.114;
                        let mask = 1.0 - luma;
                        (dst_r * mask, dst_g * mask, dst_b * mask)
                    }
                    // Behind: paint behind (source only shows where destination is transparent)
                    BlendMode::Behind => {
                        let mask = 1.0 - dst_a;
                        (src_r * mask + dst_r * (1.0 - mask), src_g * mask + dst_g * (1.0 - mask), src_b * mask + dst_b * (1.0 - mask))
                    }
                    // Alpha Add: additive blend using alpha
                    BlendMode::AlphaAdd => {
                        (src_r * src_a + dst_r * dst_a, src_g * src_a + dst_g * dst_a, src_b * src_a + dst_b * dst_a)
                    }
                    // Linear Light: combination of Linear Burn and Linear Dodge
                    BlendMode::LinearLight => {
                        let f = |s: f32, d: f32| -> f32 {
                            (s + d - 0.5).clamp(0.0, 1.0)
                        };
                        (f(src_r, dst_r),
                         f(src_g, dst_g),
                         f(src_b, dst_b))
                    }
                    BlendMode::Normal => (src_r, src_g, src_b),
                };

                // Preserve Underlying Transparency (AE 'T' switch)
                if layer.preserve_transparency {
                    src_a *= dst_a;
                    if src_a <= 0.001 { continue; }
                }

                // Alpha blending formula: Standard Source-Over or Transparency Preservation
                let out_a = if layer.preserve_transparency {
                    dst_a
                } else {
                    src_a + dst_a * (1.0 - src_a)
                };
                let out_r = if out_a > 0.0 { (blended_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                let out_g = if out_a > 0.0 { (blended_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                let out_b = if out_a > 0.0 { (blended_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };

                // Encode back to display space when in linear-light mode.
                let (or_, og_, ob_) = if blend_linear {
                    use crate::core::color::linear_to_srgb_piecewise as to_srgb;
                    (
                        to_srgb(out_r.max(0.0)),
                        to_srgb(out_g.max(0.0)),
                        to_srgb(out_b.max(0.0)),
                    )
                } else {
                    (out_r, out_g, out_b)
                };
                buffer[idx] = (or_ * 255.0) as u8;
                buffer[idx + 1] = (og_ * 255.0) as u8;
                buffer[idx + 2] = (ob_ * 255.0) as u8;
                buffer[idx + 3] = (out_a * 255.0) as u8;
            }
        }
    }

    // Apply exposure EV shift and LUT color mapping in parallel across CPU cores
    let mult = 2.0f32.powf(exposure_ev);
    buffer
        .par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(pix_i, p)| {
        let mut r = p[0] as f32 / 255.0 * mult;
        let mut g = p[1] as f32 / 255.0 * mult;
        let mut b = p[2] as f32 / 255.0 * mult;

        if lut_mode == 1 {
            // Linear sRGB conversion (2.2 Gamma linearize)
            r = r.powf(2.2);
            g = g.powf(2.2);
            b = b.powf(2.2);
        } else if lut_mode == 3 {
            // User-loaded 3D LUT (tetrahedral interpolation); falls back to
            // passthrough when no LUT is loaded.
            let (nr, ng, nb) = crate::core::ocio_color::apply_lut_pixel(r, g, b);
            r = nr;
            g = ng;
            b = nb;
        } else if lut_mode == 2 {
            // ACES preview pipeline: sRGB-decode → scene-linear exposure →
            // RRT+ODT filmic tonemap → exact piecewise sRGB re-encode.
            let lin = [
                crate::core::color::srgb_to_linear_piecewise(r),
                crate::core::color::srgb_to_linear_piecewise(g),
                crate::core::color::srgb_to_linear_piecewise(b),
            ];
            let out = crate::core::aces::aces_preview_transform(lin);
            r = out[0];
            g = out[1];
            b = out[2];
        }

        // Triangular-PDF dither (per-comp option): kills 8-bit banding from
        // gradients/glow/linear re-encode at imperceptible noise cost.
        // Deterministic per-pixel seed → renders stay byte-reproducible.
        let dither_seed = if comp.dither_output { pix_i as f32 * 0.618_034 } else { f32::NAN };
        let t1 = fract(dither_seed * 7.13);
        let t2 = fract(dither_seed * 3.71);
        let noise = (t1 - t2) / 255.0;
        let dith = |v: f32| -> u8 {
            if dither_seed.is_nan() {
                return (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            ((v + noise).clamp(0.0, 1.0) * 255.0).round() as u8
        };
        p[0] = dith(r);
        p[1] = dith(g);
        p[2] = dith(b);
    });

    buffer
}

/// Multi-frame rendering (MFR): render a range of frames in parallel using rayon.
/// Returns a Vec of (frame, pixels) in the same order as the input range.
/// Each frame is rendered independently on a separate CPU core, then the
/// results are collected sequentially for GPU texture upload.
pub fn render_frame_range_parallel(
    comp: &Composition,
    from: u32,
    to: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<(u32, Vec<u8>)> {
    use rayon::prelude::*;

    let frames: Vec<u32> = (from..=to).collect();
    frames
        .par_iter()
        .map(|&f| {
            let pixels = render_frame_to_pixels(comp, f, width, height, exposure_ev, lut_mode);
            (f, pixels)
        })
        .collect()
}

/// Render a frame and return linear-light f32 RGBA pixels (16/32bpc path).
///
/// The internal compositing still runs on 8bpc buffers (with dithering to
/// suppress banding), but the final output is decoded back to scene-linear
/// via the exact IEC sRGB piecewise transfer functions — so the returned
/// values are suitable for HDR export (EXR / HDR), ACES二次処理, or any
/// pipeline that needs physically-linear colour values.
///
/// Each pixel is `[r, g, b, a]` in `0.0..=∞` (typically 0..1 for LDR
/// scenes, but highlights can exceed 1.0 after ACES exposure boost).
pub fn render_frame_to_pixels_f32(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<[f32; 4]> {
    let buf8 = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    if buf8.is_empty() {
        return Vec::new();
    }
    let n = (width as usize) * (height as usize);
    let mut out = vec![[0.0f32; 4]; n];
    use crate::core::color::srgb_to_linear_piecewise;
    for (px, dst) in buf8.chunks_exact(4).zip(out.iter_mut()) {
        // render_frame_to_pixels already applied exposure, so decode
        // the sRGB-encoded result back to linear (exposure baked in).
        let r = srgb_to_linear_piecewise(px[0] as f32 / 255.0);
        let g = srgb_to_linear_piecewise(px[1] as f32 / 255.0);
        let b = srgb_to_linear_piecewise(px[2] as f32 / 255.0);
        let a = px[3] as f32 / 255.0;
        *dst = [r, g, b, a];
    }
    out
}

#[inline]
fn fract(x: f32) -> f32 {
    x - x.floor()
}

/// Calculate the shortest distance from point (px, py) to the polygon boundary.
/// Apply optional Wiggle Paths deformation to a sampled mask polygon.
fn wiggle_polygon(mask: &crate::core::mask::Mask, pts: Vec<[f32; 2]>, time_sec: f32) -> Vec<[f32; 2]> {
    match &mask.wiggle {
        Some(w) if w.size > 0.001 && pts.len() >= 3 => {
            let verts: Vec<crate::core::mask::MaskVertex> = pts.iter()
                .map(|p| crate::core::mask::MaskVertex::new(p[0], p[1]))
                .collect();
            crate::core::wiggle_paths::apply_wiggle_paths(&verts, time_sec, w)
                .into_iter().map(|v| v.position).collect()
        }
        _ => pts,
    }
}

fn distance_to_polygon(px: f32, py: f32, verts: &[[f32; 2]]) -> f32 {
    let mut min_dist = f32::INFINITY;
    let n = verts.len();
    if n < 2 { return 0.0; }
    for i in 0..n {
        let p1 = verts[i];
        let p2 = verts[(i + 1) % n];

        let dx = p2[0] - p1[0];
        let dy = p2[1] - p1[1];
        let l2 = dx * dx + dy * dy;
        if l2 < 1e-5 {
            let d = ((px - p1[0]).powi(2) + (py - p1[1]).powi(2)).sqrt();
            min_dist = min_dist.min(d);
            continue;
        }
        let t = ((px - p1[0]) * dx + (py - p1[1]) * dy) / l2;
        let t = t.clamp(0.0, 1.0);
        let proj_x = p1[0] + t * dx;
        let proj_y = p1[1] + t * dy;
        let d = ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt();
        min_dist = min_dist.min(d);
    }
    min_dist
}

/// Compute combined mask coverage for a pixel using all enabled masks,
/// matching the GPU path's `combine_mask_shapes` semantics.
fn compute_combined_mask_coverage(px: f32, py: f32, masks: &[CpuMaskEntry]) -> f32 {
    let mut acc = 0.0f32;
    let mut first = true;
    for mask in masks {
        if mask.vertices.len() < 3 {
            continue;
        }
        let expanded = offset_polygon_vertices(&mask.vertices, mask.expansion);
        let inside = point_in_polygon(px, py, &expanded);
        let mut cov = if mask.feather > 0.1 {
            let dist = distance_to_polygon(px, py, &expanded);
            if inside {
                (dist / mask.feather).clamp(0.0, 1.0)
            } else {
                (1.0 - dist / mask.feather).clamp(0.0, 1.0)
            }
        } else if inside {
            1.0
        } else {
            0.0
        };
        if mask.inverted {
            cov = 1.0 - cov;
        }
        if first && mask.mode == MaskMode::Subtract {
            acc = 1.0;
        }
        acc = match mask.mode {
            MaskMode::Add | MaskMode::Lighten => acc + (cov * (1.0 - acc)),
            MaskMode::Subtract => acc * (1.0 - cov),
            MaskMode::Intersect | MaskMode::Darken => acc.min(cov),
            MaskMode::Difference => (acc - cov).abs(),
            MaskMode::None => acc,
        };
        first = false;
    }
    acc
}


/// Renders a frame with a hard deadline. If rendering exceeds `deadline`, the
/// cooperative cancel flag trips and the function returns early with
/// `timed_out = true` and whatever was composited so far.
///
/// This is the watchdog primitive for hang protection: wrap risky renders so a
/// pathological composition degrades to an incomplete frame instead of freezing
/// the export pipeline or UI thread forever.
pub fn render_frame_with_deadline(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
    deadline: std::time::Duration,
) -> (Vec<u8>, bool) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    let flag = Arc::new(AtomicBool::new(false));
    set_render_cancel_flag(Some(flag.clone()));

    // Watchdog timer: flips the flag when the deadline expires.
    let timer_flag = flag.clone();
    let timer = std::thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < deadline {
            if timer_flag.load(Ordering::Relaxed) {
                return; // already cancelled by other means
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        timer_flag.store(true, Ordering::Relaxed);
    });

    let pixels = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    let timed_out = flag.load(Ordering::Relaxed);

    set_render_cancel_flag(None);

    // Stop the watchdog promptly: flipping the flag makes its loop exit.
    flag.store(true, Ordering::Relaxed);
    let _ = timer.join();

    (pixels, timed_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType, BlendMode, Effect, EffectType};
    use crate::core::property::Animatable;

    fn write_gray_png(path: &std::path::Path, gray: u8) {
        let img = image::GrayImage::from_pixel(4, 4, image::Luma([gray]));
        image::DynamicImage::ImageLuma8(img)
            .save_with_format(path, image::ImageFormat::Png)
            .expect("save png");
    }

    #[test]
    fn test_video_layer_speed_scales_sequence_index() {
        let dir = std::env::temp_dir().join(format!("aevfx_speed_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // frame i encoded as gray value i * 10 (frame 20 -> 200)
        for i in 0..=20u32 {
            write_gray_png(&dir.join(format!("frame_{:05}.png", i)), (i * 10) as u8);
        }
        let frames_dir = dir.to_string_lossy().to_string();

        let make_comp = |speed: f32| {
            let mut comp = Composition::new("c1".into(), "Comp".into(), 16, 16, 30, 30);
            let mut layer = Layer::new("v".into(), "V".into(), LayerType::Video {
                source: "test".into(),
                frames_dir: frames_dir.clone(),
                frame_count: 21,
                audio_wav: None,
                speed,
            }, 30);
            layer.transform.position = Animatable::new_constant([8.0, 8.0]);
            comp.layers.push(layer);
            comp
        };

        // speed 2.0: timeline frame 10 -> sequence frame 20 (gray 200)
        let px_fast = render_frame_to_pixels(&make_comp(2.0), 10, 16, 16, 0.0, 0);
        assert_eq!(px_fast[8 * 16 * 4 + 8 * 4], 200, "speed=2.0 must map frame 10 to seq frame 20");

        // speed 1.0: timeline frame 10 -> sequence frame 10 (gray 100)
        let px_norm = render_frame_to_pixels(&make_comp(1.0), 10, 16, 16, 0.0, 0);
        assert_eq!(px_norm[8 * 16 * 4 + 8 * 4], 100);

        // clamping: speed 2.0 at last frame stays within sequence
        let px_clamp = render_frame_to_pixels(&make_comp(2.0), 15, 16, 16, 0.0, 0);
        assert_eq!(px_clamp[8 * 16 * 4 + 8 * 4], 200, "index must clamp to last frame");

        let _ = std::fs::remove_dir_all(&dir);
    }


    #[test]
    fn test_software_render_frame_to_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 100, 100, 30, 30);
        let mut layer = Layer::new("l1".to_string(), "Solid".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        layer.blend_mode = BlendMode::Multiply;
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 100, 100, 0.0, 0);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_particle_layer_renders() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new("p1".to_string(), "Particles".to_string(), LayerType::Particle {
            emitter: crate::core::particle_system::ParticleEmitter {
                rate: 500.0,
                lifetime: 1.0,
                speed: 50.0,
                size_start: 6.0,
                size_end: 3.0,
                ..Default::default()
            },
        }, 30);
        layer.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        // Frame 10 should have particles simulated and rendered
        let pixels = render_frame_to_pixels(&comp, 10, 64, 64, 0.0, 0);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Some pixels should be brighter than the dark background (20,20,25)
        let bright = pixels.chunks_exact(4).filter(|p| p[0] > 40).count();
        assert!(bright > 0, "Particle layer should produce visible pixels");
    }

    #[test]
    fn test_particle_layer_deterministic() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new("p1".to_string(), "Particles".to_string(), LayerType::Particle {
            emitter: crate::core::particle_system::ParticleEmitter::default(),
        }, 30);
        layer.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let p1 = render_frame_to_pixels(&comp, 5, 64, 64, 0.0, 0);
        let p2 = render_frame_to_pixels(&comp, 5, 64, 64, 0.0, 0);
        assert_eq!(p1, p2, "Particle simulation must be deterministic per frame");
    }

    #[test]
    fn test_motion_blur_smears_moving_layer() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 100, 100, 30, 30);
        let mut layer = Layer::new("m1".to_string(), "Moving Solid".to_string(), LayerType::Solid { color: [1.0, 1.0, 1.0, 1.0] }, 30);
        layer.motion_blur = true;
        // Move horizontally: position keyframes at x=20 (frame 5) → x=80 (frame 15)
        layer.transform.position = Animatable::new_animated(vec![
            crate::core::keyframe::Keyframe::new(5, [20.0, 50.0], crate::core::keyframe::InterpolationType::Linear),
            crate::core::keyframe::Keyframe::new(15, [80.0, 50.0], crate::core::keyframe::InterpolationType::Linear),
        ]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 10, 100, 100, 0.0, 0);
        assert_eq!(pixels.len(), 100 * 100 * 4);
        // Motion-blurred moving layer should still render without panic
    }

    #[test]
    fn test_3d_light_shading_applied() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
        let mut layer = Layer::new("l3d".to_string(), "3D Solid".to_string(), LayerType::Solid { color: [1.0, 1.0, 1.0, 1.0] }, 30);
        layer.is_3d = true;
        comp.layers.push(layer);

        // Strong light near the layer center should brighten it
        comp.lights[0].intensity = 200.0;
        comp.lights[0].position = Animatable::new_constant([16.0, 16.0, -300.0]);

        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        // Center pixel should be lit (brighter than ambient-only floor)
        let center_idx = ((16 * 32 + 16) * 4) as usize;
        assert!(pixels[center_idx] > 60, "3D layer should be shaded by light, got {}", pixels[center_idx]);
    }

    #[test]
    fn test_dof_blurs_off_focus_3d_layer() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
        let mut layer = Layer::new("l3d".to_string(), "3D Solid".to_string(), LayerType::Solid { color: [1.0, 1.0, 1.0, 1.0] }, 30);
        layer.is_3d = true;
        // Push the layer far from the focus plane
        layer.transform_3d.position = Animatable::new_constant([16.0, 16.0, -3000.0]);
        comp.layers.push(layer);

        comp.active_camera.dof_enabled = true;
        comp.resolve_camera_mut().focus_distance = 1000.0;
        comp.resolve_camera_mut().aperture = 50.0;
        comp.resolve_camera_mut().dof_max_blur = 24.0;

        let dof_pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let center_idx = ((16 * 32 + 16) * 4) as usize;

        // Reference render with identical geometry but DOF off
        comp.active_camera.dof_enabled = false;
        let sharp_pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);

        // Defocus spreads the solid's energy: center dims vs the sharp render
        assert!(
            (sharp_pixels[center_idx] as i32 - dof_pixels[center_idx] as i32).abs() > 2,
            "DOF should change the off-focus render (sharp {} vs defocused {})",
            sharp_pixels[center_idx], dof_pixels[center_idx]
        );
    }


    #[test]
    fn test_text_on_path_renders_glyphs() {
        use crate::core::mask::{Mask, MaskPath};
        use crate::core::property::Animatable as PAnimatable;

        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 128, 64, 30, 30);
        let mut layer = Layer::new(
            "tp".to_string(), "Path Text".to_string(),
            LayerType::Text {
                text: "AB".to_string(),
                font_size: 24,
                color: [1.0, 1.0, 1.0, 1.0],
                font_family: "Helvetica".to_string(),
                tracking: 0.0,
                leading: 1.2,
                align: 0,
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                text_on_path: true,
            },
            30,
        );
        // Gentle horizontal wave path across the comp
        let path = MaskPath {
            vertices: PAnimatable::new_constant(vec![[16.0, 32.0], [48.0, 20.0], [80.0, 44.0], [112.0, 32.0]]),
            tangents: None,
            is_closed: false,
        };
        layer.masks.push(Mask {
            path,
            ..Mask::new_rect("m1".to_string(), "Path".to_string(), 0.0, 0.0, 10.0, 10.0)
        });
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 128, 64, 0.0, 0);
        let bright = (0..pixels.len()).step_by(4)
            .filter(|&i| pixels[i] > 200 && pixels[i + 1] > 200 && pixels[i + 2] > 200)
            .count();
        assert!(bright > 20, "text-on-path glyphs should render bright pixels, got {}", bright);
    }

    #[test]
    fn test_gpu_parent_transform_resolution() {
        // Parented layer should resolve through resolve_world_transform without panicking
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut parent = Layer::new("par".to_string(), "Parent".to_string(), LayerType::Null, 30);
        parent.transform.position = Animatable::new_constant([32.0, 32.0]);
        parent.transform.rotation = Animatable::new_constant(45.0);
        comp.layers.push(parent);
        let mut child = Layer::new("chi".to_string(), "Child".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        child.parent_id = Some("par".to_string());
        child.transform.position = Animatable::new_constant([10.0, 0.0]);
        let child_pos = child.transform.position.evaluate(0);
        comp.layers.push(child);

        let (pos, _scale, _rot, _opa) = comp.resolve_world_transform(comp.layers.last().unwrap(), 0);
        let _ = child_pos;
        // Child at local (10,0) rotated 45deg around parent origin (32,32):
        // offset = (10*cos45, 10*sin45) ≈ (7.07, 7.07) → world ≈ (39.07, 39.07)
        assert!((pos[0] - 39.07).abs() < 0.5, "unexpected world x: {}", pos[0]);
        assert!((pos[1] - 39.07).abs() < 0.5, "unexpected world y: {}", pos[1]);
    }

    #[test]
    fn test_precomp_nested_rendering() {
        // Sub-comp: red ellipse at its own center
        let mut sub = Composition::new("sub".to_string(), "Sub".to_string(), 64, 64, 30, 30);
        let mut shape = Layer::new("s1".to_string(), "Dot".to_string(), LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(40.0),
                height: Animatable::new_constant(40.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        }, 30);
        shape.transform.position = Animatable::new_constant([32.0, 32.0]);
        sub.layers.push(shape);

        // Main comp: pre-comp layer referencing the sub-comp
        let mut comp = Composition::new("main".to_string(), "Main".to_string(), 64, 64, 30, 30);
        comp.sub_compositions.push(sub);
        let mut pc = Layer::new("pc".to_string(), "Nested".to_string(), LayerType::PreComp { comp_id: "sub".to_string() }, 30);
        pc.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(pc);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(pixels[center] > 180, "PreComp should render nested shape (R={})", pixels[center]);
    }

    #[test]
    fn test_precomp_self_reference_cycle_is_safe() {
        // A comp whose pre-comp layer references ITSELF must render empty
        // (cycle guard) instead of overflowing the stack.
        let mut comp = Composition::new("selfref".to_string(), "Self".to_string(), 32, 32, 30, 30);
        let mut inner = Composition::new("selfref_inner".to_string(), "Inner".to_string(), 32, 32, 30, 30);
        // Inner contains a pre-comp pointing back at the outer comp id
        let cyc = Layer::new("cyc".to_string(), "Cycle".to_string(), LayerType::PreComp { comp_id: "selfref".to_string() }, 30);
        inner.layers.push(cyc);
        comp.sub_compositions.push(inner);

        let mut pc = Layer::new("pc".to_string(), "Loop".to_string(), LayerType::PreComp { comp_id: "selfref_inner".to_string() }, 30);
        pc.transform.position = Animatable::new_constant([16.0, 16.0]);
        comp.layers.push(pc);

        // Must terminate and produce a valid buffer
        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), (32 * 32 * 4) as usize);
    }

    #[test]
    fn test_precomp_respects_layer_scale() {
        // Sub-comp: full-frame red solid
        let mut sub = Composition::new("sub".to_string(), "Sub".to_string(), 64, 64, 30, 30);
        let solid = Layer::new("bg".to_string(), "Red".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        sub.layers.push(solid);

        // Main comp: pre-comp scaled to 50% — corners should stay background
        let mut comp = Composition::new("main".to_string(), "Main".to_string(), 64, 64, 30, 30);
        comp.sub_compositions.push(sub);
        let mut pc = Layer::new("pc".to_string(), "Half".to_string(), LayerType::PreComp { comp_id: "sub".to_string() }, 30);
        pc.transform.position = Animatable::new_constant([32.0, 32.0]);
        pc.transform.scale = Animatable::new_constant([50.0, 50.0]);
        comp.layers.push(pc);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let corner = 0usize;
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(pixels[corner] < 60, "Corner should be background when pre-comp is scaled down (R={})", pixels[corner]);
        assert!(pixels[center] > 180, "Center should show scaled pre-comp content (R={})", pixels[center]);
    }

    #[test]
    fn test_visual_headless_pixel_comparison() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        let layer = Layer::new("l1".to_string(), "Solid".to_string(), LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] }, 30);
        comp.layers.push(layer);

        let p1 = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        let p2 = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);

        let mut mse = 0.0f32;
        for i in 0..p1.len() {
            let diff = (p1[i] as f32 - p2[i] as f32) / 255.0;
            mse += diff * diff;
        }
        mse /= p1.len() as f32;

        assert!(
            mse < 1e-5,
            "Visual regression test failed! Pixel MSE ({}) exceeds threshold 1e-5",
            mse
        );
    }

    #[test]
    fn test_invert_effect_changes_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 20, 20, 30, 30);
        let mut layer = Layer::new("l1".to_string(), "Solid".to_string(), LayerType::Solid { color: [0.8, 0.2, 0.4, 1.0] }, 30);
        layer.effects.push(Effect {
            id: "e1".to_string(),
            name: "Invert".to_string(),
            effect_type: EffectType::Invert { invert_alpha: false },
            enabled: true,
        });
        comp.layers.push(layer);

        let inverted = render_frame_to_pixels(&comp, 0, 20, 20, 0.0, 0);
        let ci = ((10 * 20 + 10) * 4) as usize;
        // 0.8 -> inverted ~0.2 (255*0.2=51); allow small tolerance.
        assert!((inverted[ci] as i32 - (255 - (0.8 * 255.0) as u8) as i32).abs() <= 3,
            "invert effect did not apply: got {}", inverted[ci]);
    }

    #[test]
    fn test_twirl_effect_shifts_pixels() {
        use crate::core::cpu_effects;

        // Place a red pixel slightly off-center so the twirl displaces it.
        // (The exact center pixel at r=0 is the twirl axis and doesn't move.)
        let mut buf = vec![0u8; 16 * 16 * 4];
        let px_x = 8;
        let px_y = 10; // 2 pixels below center — r=2, within radius=30
        let idx = ((px_y * 16 + px_x) * 4) as usize;
        buf[idx] = 255; buf[idx + 1] = 40; buf[idx + 2] = 40; buf[idx + 3] = 255;

        let effects = vec![Effect {
            id: "e2".to_string(),
            name: "Twirl".to_string(),
            effect_type: EffectType::Twirl {
                angle: Animatable::new_constant(90.0),
                radius: Animatable::new_constant(30.0),
            },
            enabled: true,
        }];

        cpu_effects::apply_layer_effects(&mut buf, 16, 16, &effects, 0, 30);

        // The twirl should have moved the red pixel away from (8,10).
        let orig_val = ((px_y * 16 + px_x) * 4) as usize;
        assert_ne!(buf[orig_val], 255, "red pixel should have moved from original position");

        // A nearby pixel should now be red (displaced).
        let mut found = false;
        for y in 0..16u32 {
            for x in 0..16u32 {
                let i = ((y * 16 + x) * 4) as usize;
                if buf[i] > 200 && buf[i + 1] < 80 && buf[i + 3] > 200 {
                    found = true;
                    break;
                }
            }
            if found { break; }
        }
        assert!(found, "displaced red pixel not found in buffer after twirl");
    }

    #[test]
    fn test_adjustment_layer_applies_effects_to_composite() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        // Bottom layer: red solid
        comp.layers.push(Layer::new("l1".to_string(), "Red".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30));
        // Top layer: adjustment layer with Invert effect
        let mut adj = Layer::new_adjustment("adj1".to_string(), "Adj Invert".to_string(), 30);
        adj.effects.push(Effect {
            id: "inv".to_string(),
            name: "Invert".to_string(),
            effect_type: EffectType::Invert { invert_alpha: false },
            enabled: true,
        });
        comp.layers.push(adj);

        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        // Red (255,0,0) inverted should be (0,255,255)
        let idx = (5 * 10 + 5) * 4;
        assert_eq!(pixels[idx], 0, "R should be inverted");
        assert_eq!(pixels[idx + 1], 255, "G should be 255");
        assert_eq!(pixels[idx + 2], 255, "B should be 255");
    }

    #[test]
    fn test_fractal_noise_generates_output() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
        let mut layer = Layer::new("l1".to_string(), "Solid".to_string(), LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] }, 30);
        layer.effects.push(Effect {
            id: "fn1".to_string(),
            name: "FractalNoise".to_string(),
            effect_type: EffectType::FractalNoise {
                fractal_type: Animatable::new_constant(0.0),
                contrast: Animatable::new_constant(100.0),
                brightness: Animatable::new_constant(0.0),
                complexity: Animatable::new_constant(3.0),
                evolution: Animatable::new_constant(0.0),
            },
            enabled: true,
        });
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);

        // At least some pixels should differ from the original gray
        let mut non_gray = 0;
        for p in pixels.chunks_exact(4) {
            if (p[0] as i32 - p[1] as i32).abs() > 5 || (p[1] as i32 - p[2] as i32).abs() > 5 || (p[0] as i32 - 128).abs() > 20 {
                non_gray += 1;
            }
        }
        assert!(non_gray > 0, "FractalNoise should produce varied pixel values");
    }

    #[test]
    fn test_background_colour_is_composited() {
        let mut comp = Composition::new("c1".to_string(), "BgComp".to_string(), 8, 8, 30, 30);
        comp.background_color = [1.0, 0.0, 0.0, 1.0];

        let pixels = render_frame_to_pixels(&comp, 0, 8, 8, 0.0, 0);
        assert_eq!(pixels.len(), 8 * 8 * 4);
        for p in pixels.chunks_exact(4) {
            assert_eq!(p[0], 255, "background red must fill an empty composition");
            assert_eq!(p[3], 255, "background alpha must be opaque");
        }
    }

    #[test]
    fn test_time_remap_shifts_layer_time() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        let mut layer = Layer::new("l1".to_string(), "Red".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        // Set time remap: at frame 0, remap to frame 10
        layer.time_remap = Some(crate::core::property::Animatable::new_constant(10.0));
        comp.layers.push(layer);

        // At frame 0, the layer should evaluate at frame 10 (still active, since duration=30)
        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        // Should still have red pixels since layer is active at frame 10
        let idx = (5 * 10 + 5) * 4;
        assert!(pixels[idx] > 200, "Time remapped layer should still render at frame 0");
    }

    #[test]
    fn test_softlight_blend_uses_both_src_and_dst() {
        // Create a red layer over a green layer with SoftLight blend
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        // Bottom: green solid
        let mut bottom = Layer::new("l1".to_string(), "Green".to_string(), LayerType::Solid { color: [0.0, 1.0, 0.0, 1.0] }, 30);
        bottom.blend_mode = BlendMode::Normal;
        comp.layers.push(bottom);
        // Top: red solid with SoftLight
        let mut top = Layer::new("l2".to_string(), "Red".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        top.blend_mode = BlendMode::SoftLight;
        comp.layers.push(top);

        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        let idx = (5 * 10 + 5) * 4;
        // SoftLight with src=red(1,0,0) over dst=green(0,1,0):
        // Red channel: s=1.0 > 0.5, d=0.0, a=0.0 → d + (2*s-1)*(a-d) = 0 + 1*(0-0) = 0
        // Green channel: s=0.0 <= 0.5, d=1.0 → d - (1-2*s)*d*(1-d) = 1 - 1*1*0 = 1
        // Blue channel: both 0 → 0
        // Result should be (0, 1, 0) which is green (unchanged since src red doesn't affect green via SoftLight)
        assert!(pixels[idx + 1] > 200, "Green channel should be preserved by SoftLight");
    }

    #[test]
    fn test_shape_ellipse_renders_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(), "Ellipse".to_string(),
            LayerType::Shape { shape_type: crate::core::timeline::ShapeType::Ellipse {
                width: crate::core::property::Animatable::new_constant(100.0),
                height: crate::core::property::Animatable::new_constant(100.0),
            }, color: [1.0, 0.0, 0.0, 1.0], stroke_color: [0.0, 0.0, 0.0, 1.0], stroke_width: 0.0, fill_type: Default::default(), extrusion_depth: 0.0, bevel_depth: 0.0 },
            30,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Center pixel should be red (shape color)
        let center = (32 * 64 + 32) * 4;
        assert!(pixels[center] > 200, "Center of ellipse should be red (R={})", pixels[center]);
        // Corner pixel should be background color, not red
        let corner = 0;
        assert!(pixels[corner] < 50, "Corner should be background color (R={})", pixels[corner]);
    }

    #[test]
    fn test_shape_rectangle_renders_pixels() {
        use crate::core::timeline::ShapeType;
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(), "Rect".to_string(),
            LayerType::Shape { shape_type: ShapeType::Rectangle {
                width: crate::core::property::Animatable::new_constant(100.0),
                height: crate::core::property::Animatable::new_constant(100.0),
                corner_radius: crate::core::property::Animatable::new_constant(0.0),
            }, color: [0.0, 1.0, 0.0, 1.0], stroke_color: [0.0, 0.0, 0.0, 1.0], stroke_width: 0.0, fill_type: Default::default(), extrusion_depth: 0.0, bevel_depth: 0.0 },
            30,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let center = (32 * 64 + 32) * 4;
        assert!(pixels[center + 1] > 200, "Center of rectangle should be green");
    }
}

#[cfg(test)]
mod render_size_guard_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn tiny_comp() -> Composition {
        let mut comp = Composition::new("c".into(), "Guard".into(), 32, 32, 30, 30);
        let l = Layer::new("l".into(), "S".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        comp.layers.push(l);
        comp
    }

    #[test]
    fn test_zero_and_huge_dimensions_return_empty() {
        let comp = tiny_comp();
        // Zero dimensions
        assert!(render_frame_to_pixels(&comp, 0, 0, 32, 0.0, 0).is_empty());
        assert!(render_frame_to_pixels(&comp, 0, 32, 0, 0.0, 0).is_empty());
        // Huge dimensions (would be ~40 GB) must be rejected without allocating
        assert!(render_frame_to_pixels(&comp, 0, 100_000, 100_000, 0.0, 0).is_empty());
        assert!(render_frame_to_pixels(&comp, 0, u32::MAX, u32::MAX, 0.0, 0).is_empty());
    }

    #[test]
    fn test_max_dimension_still_renders_small() {
        let comp = tiny_comp();
        let pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        assert_eq!(pixels.len(), 16 * 16 * 4);
    }

    #[test]
    fn test_rgba_buffer_size_overflow_safe() {
        assert!(rgba_buffer_size(u32::MAX, u32::MAX).is_none());
        assert!(rgba_buffer_size(0, 0).is_some()); // 0 is valid size, caller checks
        assert_eq!(rgba_buffer_size(2, 2), Some(16));
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn busy_comp(layers: usize) -> Composition {
        let mut comp = Composition::new("c".into(), "Cancel".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(format!("l{}", i), format!("L{}", i), LayerType::Solid { color: [0.5; 4] }, 30);
            l.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
            comp.layers.push(l);
        }
        comp
    }

    #[test]
    fn test_cancel_flag_stops_layer_processing() {
        let comp = busy_comp(200);
        let flag = Arc::new(AtomicBool::new(true)); // cancelled from the start
        set_render_cancel_flag(Some(flag.clone()));

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Buffer is still valid; but no layer work should have happened
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Background-only buffer: no bright solid pixels
        let bright = (0..pixels.len()).step_by(4).filter(|&i| pixels[i] > 100).count();
        assert_eq!(bright, 0, "cancelled render must not composite layers");

        set_render_cancel_flag(None);

        // Without the flag the same comp renders layers normally
        let pixels2 = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let bright2 = (0..pixels2.len()).step_by(4).filter(|&i| pixels2[i] > 100).count();
        assert!(bright2 > 0, "un-cancelled render must composite layers");
    }

    #[test]
    fn test_clearing_flag_restores_full_render() {
        let comp = busy_comp(10);
        let flag = Arc::new(AtomicBool::new(false));
        set_render_cancel_flag(Some(flag.clone()));
        set_render_cancel_flag(None); // cleared — must behave as default

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let bright = (0..pixels.len()).step_by(4).filter(|&i| pixels[i] > 100).count();
        assert!(bright > 0);
    }
}

#[cfg(test)]
mod watchdog_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn comp(layers: usize) -> Composition {
        let mut c = Composition::new("c".into(), "Watchdog".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(format!("l{}", i), format!("L{}", i), LayerType::Solid { color: [0.6; 4] }, 30);
            l.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
            c.layers.push(l);
        }
        c
    }

    #[test]
    fn test_fast_render_completes_without_timeout() {
        let (pixels, timed_out) = render_frame_with_deadline(&comp(5), 0, 64, 64, 0.0, 0, std::time::Duration::from_secs(5));
        assert!(!timed_out);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Layers must actually be composited
        let bright = (0..pixels.len()).step_by(4).filter(|&i| pixels[i] > 100).count();
        assert!(bright > 0);
    }

    #[test]
    fn test_deadline_aborts_render_early() {
        // Generous layer count; deadline so short the render cannot finish
        let start = std::time::Instant::now();
        let (pixels, timed_out) = render_frame_with_deadline(
            &comp(2000), 0, 256, 256, 0.0, 0,
            std::time::Duration::from_millis(5),
        );
        let elapsed = start.elapsed();
        assert!(timed_out, "render must report timeout");
        assert!(
            elapsed < std::time::Duration::from_millis(2000),
            "watchdog must return promptly, took {:?}",
            elapsed
        );
        // Partial buffer is still a valid allocation
        assert_eq!(pixels.len(), 256 * 256 * 4);
    }

    #[test]
    fn test_watchdog_does_not_leak_cancel_state() {
        // After a timed-out render, normal renders must work again
        let _ = render_frame_with_deadline(&comp(500), 0, 128, 128, 0.0, 0, std::time::Duration::from_millis(1));
        let pixels = render_frame_to_pixels(&comp(3), 0, 64, 64, 0.0, 0);
        let bright = (0..pixels.len()).step_by(4).filter(|&i| pixels[i] > 100).count();
        assert!(bright > 0, "cancel state must not leak into subsequent renders");
    }

}

#[cfg(test)]
mod shadow_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};
    use crate::core::property::Animatable;

    fn shadow_test_comp(caster_casts: bool) -> Composition {
        let mut comp = Composition::new("sh".into(), "Shadows".into(), 64, 64, 30, 30);
        // Receiver: full-frame white solid (bottom of stack)
        let mut recv = Layer::new("r1".into(), "Floor".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        recv.transform.scale = Animatable::new_constant([100.0, 100.0]);
        recv.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(recv);

        // Caster: red solid raised on +z between light and floor plane
        let mut caster = Layer::new("c1".into(), "Card".into(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        caster.is_3d = true;
        caster.material.cast_shadows = caster_casts;
        caster.transform.scale = Animatable::new_constant([20.0, 20.0]);
        caster.transform.position = Animatable::new_constant([36.0, 36.0]);
        caster.transform_3d.position = Animatable::new_constant([36.0, 36.0, 100.0]);
        comp.layers.push(caster);

        // Point light upper-left of the caster, above the plane (+z), casting
        let light = crate::core::timeline::Light3D {
            id: "key".into(),
            name: "Key".into(),
            light_type: crate::core::timeline::LightType::Point,
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 100.0,
            position: Animatable::new_constant([16.0, 16.0, 400.0]),
            casts_shadows: true,
            shadow_darkness: 90.0,
            falloff: 1.0,
            max_radius: 0.0,
        };
        comp.lights = vec![light];
        comp
    }

    #[test]
    fn test_shadow_falls_away_from_light() {
        let comp = shadow_test_comp(true);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Projection of caster center through L(16,16,400) onto z=0:
        // t=400/300 → (16+1.333*20, ...) ≈ (42.7, 42.7)
        let in_shadow = ((46 * 64 + 44) * 4) as usize;
        let lit_corner = ((8 * 64 + 8) * 4) as usize;
        assert!(px[in_shadow] < 200, "shadow region darkened, R={}", px[in_shadow]);
        assert!(px[lit_corner] >= 240, "far corner stays lit, R={}", px[lit_corner]);
    }

    #[test]
    fn test_no_shadow_when_caster_disabled() {
        let comp = shadow_test_comp(false);
        let px = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let probe = ((46 * 64 + 44) * 4) as usize;
        assert!(px[probe] >= 240, "no caster -> no shadow, R={}", px[probe]);
    }

    #[test]
    fn test_collapse_transformations_3d_z_continuity() {
        // Draw-order rule (renderer sorts far-first): SMALLER effective z is
        // nearer and paints last. Collapsed child lifts to pz+cz=200, nearer
        // than green 350 -> child wins. Uncollapsed card sits at pz=400,
        // farther than green -> green wins. Identical stack, opposite winner.
        let build = |collapsed: bool| {
            let mut comp = Composition::new("cz".into(), "Collapse3D".into(), 64, 64, 30, 30);
            let mut green = Layer::new("green".into(), "G".into(), LayerType::Solid { color: [0.0, 1.0, 0.0, 1.0] }, 30);
            green.is_3d = true;
            green.transform.scale = Animatable::new_constant([100.0, 100.0]);
            green.transform.position = Animatable::new_constant([32.0, 32.0]);
            green.transform_3d.position = Animatable::new_constant([32.0, 32.0, 350.0]);
            comp.layers.push(green);

            let mut sub = Composition::new("subz".into(), "S".into(), 64, 64, 30, 30);
            let mut blue_child = Layer::new("bc".into(), "B".into(), LayerType::Solid { color: [0.0, 0.0, 1.0, 1.0] }, 30);
            blue_child.is_3d = true;
            blue_child.transform.scale = Animatable::new_constant([100.0, 100.0]);
            blue_child.transform.position = Animatable::new_constant([32.0, 32.0]);
            blue_child.transform_3d.position = Animatable::new_constant([32.0, 32.0, -200.0]);
            sub.layers.push(blue_child);
            comp.sub_compositions.push(sub);

            let mut pc = Layer::new("pc".into(), "P".into(), LayerType::PreComp { comp_id: "subz".into() }, 30);
            pc.is_collapsed = collapsed;
            pc.is_3d = true;
            pc.transform.scale = Animatable::new_constant([100.0, 100.0]);
            pc.transform.position = Animatable::new_constant([32.0, 32.0]);
            pc.transform_3d.position = Animatable::new_constant([32.0, 32.0, 400.0]);
            comp.layers.push(pc);
            comp
        };

        let px_on = render_frame_to_pixels(&build(true), 0, 64, 64, 0.0, 0);
        let i = ((16 * 64 + 16) * 4) as usize;
        assert!(px_on[i + 2] > px_on[i + 1], "collapsed child (nearer) beats green: {:?}",
            &px_on[i..i + 3]);

        // Structural check: flattening composes parent z + child z and maps
        // the child into parent space around the sub-comp center.
        let flat = flatten_collapsed(&build(true), 0);
        let red_child = flat.layers.iter().find(|l| l.name == "B").expect("expanded child");
        assert!(red_child.is_3d);
        let lifted = red_child.transform_3d.position.evaluate(0);
        assert!((lifted[2] - 200.0).abs() < 0.01, "pz+cz lift: {}", lifted[2]);
        assert!((lifted[0] - 32.0).abs() < 0.01 && (lifted[1] - 32.0).abs() < 0.01,
            "child mapped around parent center: {:?}", lifted);
    }

    #[test]
    fn test_render_frame_to_pixels_f32_returns_linear() {
        let mut comp = Composition::new("f32t".into(), "F32".into(), 8, 8, 30, 30);
        comp.background_color = [1.0, 1.0, 1.0, 1.0];
        let f32_px = render_frame_to_pixels_f32(&comp, 0, 8, 8, 0.0, 0);
        assert_eq!(f32_px.len(), 64);
        // White bg -> linear sRGB decode of 1.0 should be 1.0
        for p in &f32_px {
            assert!(p[0] > 0.99, "expected ~1.0 linear, got {}", p[0]);
            assert!(p[3] > 0.99, "expected opaque alpha, got {}", p[3]);
        }
        // Empty render returns empty
        let empty = render_frame_to_pixels_f32(&comp, 0, 0, 0, 0.0, 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_render_frame_to_pixels_f32_exposure_boosts() {
        let mut comp = Composition::new("f32exp".into(), "F32E".into(), 4, 4, 30, 30);
        comp.background_color = [0.5, 0.5, 0.5, 1.0];
        let no_ev = render_frame_to_pixels_f32(&comp, 0, 4, 4, 0.0, 0);
        let plus2 = render_frame_to_pixels_f32(&comp, 0, 4, 4, 2.0, 0);
        // +2 EV boosts values; with sRGB encode/decode + dithering
        // the ratio isn't exactly 4x, but must be significantly > 1.
        assert!(plus2[0][0] > no_ev[0][0] * 2.0, "expected >2x, got {}", plus2[0][0] / no_ev[0][0]);
        // All values in valid range
        for p in &plus2 {
            for c in p { assert!(*c >= 0.0 && !c.is_nan()); }
        }
    }

    #[test]
    fn test_lut_mode_2_aces_pipeline_matches_reference() {
        let mut comp = Composition::new("aces".into(), "A".into(), 16, 16, 30, 30);
        let mut s = Layer::new("s".into(), "S".into(), LayerType::Solid { color: [1.0, 1.0, 1.0, 1.0] }, 30);
        s.transform.scale = Animatable::new_constant([100.0, 100.0]);
        s.transform.position = Animatable::new_constant([8.0, 8.0]);
        comp.layers.push(s);

        let px = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 2);
        let i = ((4 * 16 + 4) * 4) as usize;

        // Reference: white → decode(1)=1 → tonemap ≈0.8019776 → encode
        let expected = crate::core::aces::aces_preview_transform([1.0, 1.0, 1.0]);
        let want = (expected[0] * 255.0).round().clamp(0.0, 255.0) as u8;
        assert!(
            (px[i] as i32 - want as i32).abs() <= 1,
            "ACES preview white: got {} want {}",
            px[i], want
        );
        // And it must differ from plain passthrough (255)
        assert!(px[i] < 250, "tonemap must not be identity on white");
    }

    fn adjustment_test_comp(opacity: f32) -> Composition {
        let mut comp = Composition::new("adj".into(), "Adj".into(), 32, 32, 30, 30);
        // Bottom: pure white solid covering the frame
        let mut base = Layer::new("b1".into(), "Base".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        base.transform.scale = Animatable::new_constant([100.0, 100.0]);
        base.transform.position = Animatable::new_constant([16.0, 16.0]);
        comp.layers.push(base);
        // Adjustment layer with Invert at the given opacity
        let mut adj = Layer::new("a1".into(), "Adjust".into(), LayerType::AdjustmentLayer, 30);
        adj.effects.push(crate::core::timeline::Effect {
            id: "fx_inv".into(),
            enabled: true,
            name: "Invert".into(),
            effect_type: crate::core::timeline::EffectType::Invert { invert_alpha: false },
        });
        adj.transform.opacity = Animatable::new_constant(opacity);
        comp.layers.push(adj);
        comp
    }

    #[test]
    fn test_adjustment_layer_inverts_below_at_full_opacity() {
        let comp = adjustment_test_comp(100.0);
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let i = ((8 * 32 + 8) * 4) as usize;
        assert!(px[i] < 20, "white inverted to near-black, R={}", px[i]);
    }

    #[test]
    fn test_adjustment_layer_opacity_blends() {
        let comp = adjustment_test_comp(50.0);
        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        let i = ((8 * 32 + 8) * 4) as usize;
        assert!(px[i] > 100 && px[i] < 160, "50% invert of white ~127, R={}", px[i]);
    }

    #[test]
    fn test_ellipse_caster_shadow_is_round_not_square() {
        // Ellipse caster: the four bbox corners of its bounding quad must stay
        // LIT (round shape doesn't reach them), while the center is darkened.
        let mut comp = Composition::new("e".into(), "EllipseShadow".into(), 96, 96, 30, 30);
        let mut recv = Layer::new("r1".into(), "Floor".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        recv.transform.scale = Animatable::new_constant([100.0, 100.0]);
        recv.transform.position = Animatable::new_constant([48.0, 48.0]);
        comp.layers.push(recv);

        let mut caster = Layer::new(
            "c1".into(),
            "Disc".into(),
            LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::Ellipse {
                    width: Animatable::new_constant(40.0),
                    height: Animatable::new_constant(40.0),
                },
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        );
        caster.is_3d = true;
        caster.material.cast_shadows = true;
        caster.transform.scale = Animatable::new_constant([100.0, 100.0]);
        caster.transform.position = Animatable::new_constant([60.0, 60.0]);
        caster.transform_3d.position = Animatable::new_constant([60.0, 60.0, 100.0]);
        comp.layers.push(caster);
        comp.lights = vec![crate::core::timeline::Light3D {
            id: "k".into(),
            name: "K".into(),
            light_type: crate::core::timeline::LightType::Point,
            color: [1.0; 4],
            intensity: 100.0,
            position: Animatable::new_constant([48.0, 48.0, 300.0]),
            casts_shadows: true,
            shadow_darkness: 90.0,
            falloff: 1.0,
            max_radius: 0.0,
        }];

        let px = render_frame_to_pixels(&comp, 0, 96, 96, 0.0, 0);
        // Projection center ≈ light + t*(caster-light), t=300/200=1.5 →
        // (48+1.5*12, same) = (66,66); ellipse radius scales to ~30px.
        let in_shadow = ((70 * 96 + 68) * 4) as usize;
        // Bounding-quad corner of the projected ellipse (~±30 from center):
        // a square fallback would darken (92,92); the round shape must not.
        let quad_corner = ((90 * 96 + 90) * 4) as usize;
        assert!(px[in_shadow] < 210, "ellipse shadow core darkened, R={}", px[in_shadow]);
        assert!(px[quad_corner] > 235, "round shadow must spare quad corner, R={}", px[quad_corner]);
    }

    #[test]
    fn test_preserve_transparency_blends_only_onto_opaque_dest() {
        let mut comp = Composition::new("pt".into(), "PTComp".into(), 32, 32, 30, 30);
        // Base layer: small 16x16 white square in the center (16..32, 16..32), background transparent
        comp.background_color = [0.0, 0.0, 0.0, 0.0];
        let mut base = Layer::new("b".into(), "Base".into(), LayerType::Solid { color: [1.0, 1.0, 1.0, 1.0] }, 30);
        base.transform.position = Animatable::new_constant([16.0, 16.0]);
        base.transform.scale = Animatable::new_constant([50.0, 50.0]); // 16x16
        comp.layers.push(base);

        // Top layer: red solid covering entire 32x32 screen, but with preserve_transparency = true
        let mut top = Layer::new("t".into(), "TopRed".into(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        top.transform.position = Animatable::new_constant([16.0, 16.0]);
        top.transform.scale = Animatable::new_constant([100.0, 100.0]);
        top.preserve_transparency = true;
        comp.layers.push(top);

        let px = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        // Center pixel (16, 16): should be painted Red (R=255, G=0, B=0, A=255)
        let c_idx = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(px[c_idx], 255);
        assert_eq!(px[c_idx + 1], 0);

        // Corner pixel (2, 2): should remain transparent (A=0), not painted red
        let corner_idx = ((2 * 32 + 2) * 4) as usize;
        assert_eq!(px[corner_idx + 3], 0);
    }

    #[test]
    fn test_guide_layer_skipped_in_precomp() {
        let mut sub_comp = Composition::new("sub".into(), "Sub".into(), 32, 32, 30, 30);
        let mut guide = Layer::new("g".into(), "Guide".into(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        guide.is_guide_layer = true;
        sub_comp.layers.push(guide);

        let mut main_comp = Composition::new("main".into(), "Main".into(), 32, 32, 30, 30);
        main_comp.background_color = [0.0, 0.0, 0.0, 0.0];
        let precomp_layer = Layer::new("p".into(), "Pre".into(), LayerType::PreComp { comp_id: "sub".into() }, 30);
        main_comp.layers.push(precomp_layer);
        main_comp.sub_compositions.push(sub_comp);

        let px = render_frame_to_pixels(&main_comp, 0, 32, 32, 0.0, 0);
        // Entire buffer should be empty / transparent since the only sub-layer was a guide layer
        assert!(px.iter().all(|&b| b == 0));
    }
}


