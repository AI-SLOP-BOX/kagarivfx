use crate::core::timeline::{ShapeFillType, ShapeType, TrimPaths};

pub fn tessellate_bezier_path(
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

        let out_tan = if i < tangents.len() {
            tangents[i].1
        } else {
            p0
        };
        let in_tan = if (i + 1) % n < tangents.len() {
            tangents[(i + 1) % n].0
        } else {
            p1
        };

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

                let x = mt3 * p0[0]
                    + 3.0 * mt2 * t * out_tan[0]
                    + 3.0 * mt * t2 * in_tan[0]
                    + t3 * p1[0];
                let y = mt3 * p0[1]
                    + 3.0 * mt2 * t * out_tan[1]
                    + 3.0 * mt * t2 * in_tan[1]
                    + t3 * p1[1];
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

// ─── SDF Primitives ──────────────────────────────────────────────────────

/// SDF for axis-aligned rectangle centered at origin with half-extents (hx, hy).
pub fn sdf_rectangle(x: f32, y: f32, hx: f32, hy: f32) -> f32 {
    let dx = x.abs() - hx;
    let dy = y.abs() - hy;
    let outside = (dx.max(0.0), dy.max(0.0));
    let inside = dx.min(0.0).max(dy.min(0.0));
    (outside.0 * outside.0 + outside.1 * outside.1).sqrt() + inside
}

/// SDF for axis-aligned ellipse centered at origin with radii (rx, ry).
/// Returns negative inside, positive outside.
pub fn sdf_ellipse(x: f32, y: f32, rx: f32, ry: f32) -> f32 {
    let nx = x / rx;
    let ny = y / ry;
    let dist = (nx * nx + ny * ny).sqrt() - 1.0;
    dist * rx.min(ry)
}

/// SDF for a star with n points, outer radius, and inner radius.
/// Returns negative inside, positive outside.
pub fn sdf_star(x: f32, y: f32, points: u32, outer_r: f32, inner_r: f32) -> f32 {
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
pub fn sdf_polygon(x: f32, y: f32, sides: u32, radius: f32) -> f32 {
    let angle = y.atan2(x);
    let radius_point = (x * x + y * y).sqrt();
    let segment = 2.0 * std::f32::consts::PI / (sides as f32);
    let a = (angle % segment + segment) % segment - segment * 0.5;
    let s = radius * (a.cos() / (std::f32::consts::PI / sides as f32).cos());
    radius_point - s
}

/// SDF for an arbitrary polygon defined by vertices.
pub fn sdf_polygon_points(x: f32, y: f32, points: &[(f32, f32)]) -> f32 {
    if points.len() < 3 {
        return 1.0;
    }
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
    if wn != 0 {
        -dist
    } else {
        dist
    }
}

// ─── Boolean SDF Operations ──────────────────────────────────────────────

pub fn sdf_boolean_union(d1: f32, d2: f32) -> f32 {
    d1.min(d2)
}
pub fn sdf_boolean_subtract(d1: f32, d2: f32) -> f32 {
    d1.max(-d2)
}
pub fn sdf_boolean_intersect(d1: f32, d2: f32) -> f32 {
    d1.max(d2)
}
pub fn sdf_boolean_exclude(d1: f32, d2: f32) -> f32 {
    d1.abs().min(d2.abs()).copysign(d1)
}

pub fn sdf_boolean_op(op: u32, d1: f32, d2: f32) -> f32 {
    match op {
        0 => sdf_boolean_union(d1, d2),
        1 => sdf_boolean_subtract(d1, d2),
        2 => sdf_boolean_intersect(d1, d2),
        3 => sdf_boolean_exclude(d1, d2),
        _ => sdf_boolean_union(d1, d2),
    }
}

// ─── Color Utilities ─────────────────────────────────────────────────────

/// Convert RGB (each 0..1) to HSB (H: 0..360, S: 0..1, B: 0..1).
pub fn rgb_to_hsb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
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
pub fn hsb_to_rgb(h: f32, s: f32, b: f32) -> (f32, f32, f32) {
    let c = b * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = b - c;
    let (r, g, bl) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (r + m, g + m, bl + m)
}

// ─── Gradient Utilities ──────────────────────────────────────────────────

pub fn sample_gradient(colors: &[[f32; 4]], stops: &[f32], t: f32) -> [f32; 4] {
    if colors.is_empty() {
        return [0.5, 0.5, 0.5, 1.0];
    }
    if colors.len() == 1 || stops.len() <= 1 {
        return colors[0];
    }
    let t = t.clamp(0.0, 1.0);
    for i in 0..stops.len().saturating_sub(1) {
        if t >= stops[i] && t <= stops[i + 1] {
            let range = stops[i + 1] - stops[i];
            let local_t = if range.abs() < 0.001 {
                0.0
            } else {
                (t - stops[i]) / range
            };
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

pub fn resolve_fill_color(
    fill: &ShapeFillType,
    fallback: [f32; 4],
    px: f32,
    py: f32,
    cx: f32,
    cy: f32,
) -> [f32; 4] {
    match fill {
        ShapeFillType::Solid => fallback,
        ShapeFillType::LinearGradient {
            start,
            end,
            colors,
            stops,
        } => {
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
        ShapeFillType::RadialGradient {
            center,
            radius,
            colors,
            stops,
        } => {
            let dx = px - cx - center[0];
            let dy = py - cy - center[1];
            let dist = (dx * dx + dy * dy).sqrt();
            let t = (dist / radius.max(0.001)).clamp(0.0, 1.0);
            sample_gradient(colors, stops, t)
        }
    }
}

// ─── Shape Rasterization ─────────────────────────────────────────────────

/// Rasterize a shape layer into the layer buffer using SDF.
/// Returns true if any pixels were written.
#[allow(clippy::too_many_arguments)]
pub fn rasterize_shape_sdf(
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
    fill_type: &ShapeFillType,
    stroke_color: [f32; 4],
    stroke_width: f32,
    l_opacity: f32,
    shape_type: &ShapeType,
    frame: u32,
    trim_paths: Option<&TrimPaths>,
) -> bool {
    rasterize_shape_sdf_with_rotation(
        layer_buf,
        bw,
        bh,
        min_x,
        min_y,
        cx,
        cy,
        bounds_x,
        bounds_y,
        base_color,
        fill_type,
        stroke_color,
        stroke_width,
        l_opacity,
        shape_type,
        frame,
        trim_paths,
        0.0,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn rasterize_shape_sdf_with_rotation(
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
    fill_type: &ShapeFillType,
    stroke_color: [f32; 4],
    stroke_width: f32,
    l_opacity: f32,
    shape_type: &ShapeType,
    frame: u32,
    trim_paths: Option<&TrimPaths>,
    rotation_deg: f32,
) -> bool {
    let rotation = (-rotation_deg).to_radians();
    let (sin_rotation, cos_rotation) = rotation.sin_cos();
    let mut any_written = false;
    for py in 0..bh {
        for px in 0..bw {
            let world_x = min_x + px;
            let world_y = min_y + py;
            let raw_x = world_x as f32 - cx;
            let raw_y = world_y as f32 - cy;
            let local_x = raw_x * cos_rotation - raw_y * sin_rotation;
            let local_y = raw_x * sin_rotation + raw_y * cos_rotation;
            let nx = local_x / bounds_x;
            let ny = local_y / bounds_y;

            let dist = match shape_type {
                ShapeType::Rectangle {
                    width,
                    height,
                    corner_radius,
                } => {
                    let w = width.evaluate(frame) / 100.0;
                    let h = height.evaluate(frame) / 100.0;
                    let cr = corner_radius.evaluate(frame) / 100.0;
                    let hx = w * 0.5;
                    let hy = h * 0.5;
                    if cr > 0.01 {
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
                ShapeType::Star {
                    points,
                    inner_radius,
                    outer_radius,
                } => {
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
                ShapeType::FreeformBezier {
                    points,
                    tangents,
                    closed,
                } => {
                    if points.len() < 3 {
                        1.0
                    } else {
                        let tessellated = tessellate_bezier_path(points, tangents, *closed, 8);
                        let scale = 100.0;
                        let pts: Vec<(f32, f32)> = tessellated
                            .iter()
                            .map(|p| (p[0] / scale, p[1] / scale))
                            .collect();
                        sdf_polygon_points(nx, ny, &pts)
                    }
                }
            };

            let pixel_width = 4.0 / bounds_x;
            let mut alpha = (1.0 - (dist / pixel_width).clamp(0.0, 1.0)) * l_opacity;

            if alpha > 0.001 {
                if let Some(tp) = trim_paths {
                    let angle = ny.atan2(nx);
                    let angle_norm = (angle / (2.0 * std::f32::consts::PI) + 1.0).fract();
                    let start_pct = tp.start.evaluate(frame).clamp(0.0, 100.0) / 100.0;
                    let end_pct = tp.end.evaluate(frame).clamp(0.0, 100.0) / 100.0;
                    let offset_pct = (tp.offset.evaluate(frame) / 360.0).fract();
                    let s = (start_pct + offset_pct).fract();
                    let e = (end_pct + offset_pct).fract();
                    let in_trim = if (s - e).abs() < f32::EPSILON {
                        false
                    } else if s < e {
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
                    let (r, g, b, a) = if stroke_width > 0.5 && dist.abs() < stroke_width / bounds_x
                    {
                        (
                            stroke_color[0],
                            stroke_color[1],
                            stroke_color[2],
                            stroke_color[3] * alpha,
                        )
                    } else {
                        let fc = resolve_fill_color(
                            fill_type,
                            base_color,
                            world_x as f32,
                            world_y as f32,
                            cx,
                            cy,
                        );
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
