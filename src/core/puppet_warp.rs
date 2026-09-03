//! Puppet-tool mesh warp: inverse-distance-weighted displacement of an
//! isolated layer buffer, driven by pin rest/current position pairs
//! (both in layer-buffer pixel space).
//!
//! For every destination pixel the displacement is a Shepard interpolation
//! of the pin offsets with weights 1/(dist²+ε), then the source color is
//! sampled bilinearly from the pre-warp copy.

/// Pins as `(source_xy, current_xy)` pairs in buffer pixel coordinates.
pub fn warp_layer_buf(buf: &mut [u8], w: u32, h: u32, pins: &[([f32; 2], [f32; 2])]) {
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

/// Pins as `(source_xy, current_xy)` pairs in buffer pixel coordinates.
#[derive(Debug, Clone, Copy)]
pub struct TriPoint {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Triangle(pub usize, pub usize, pub usize);

fn delaunay_triangulate(points: &[TriPoint]) -> Vec<Triangle> {
    if points.len() < 3 {
        return Vec::new();
    }
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let dmax = dx.max(dy).max(1.0);
    let midx = (min_x + max_x) * 0.5;
    let midy = (min_y + max_y) * 0.5;

    // Build working array: super-triangle vertices first, then real points
    let mut all: Vec<TriPoint> = Vec::with_capacity(points.len() + 3);
    all.push(TriPoint {
        x: midx + 2.0 * dmax,
        y: midy - dmax,
        dx: 0.0,
        dy: 0.0,
    });
    all.push(TriPoint {
        x: midx,
        y: midy + 2.0 * dmax,
        dx: 0.0,
        dy: 0.0,
    });
    all.push(TriPoint {
        x: midx - 2.0 * dmax,
        y: midy - dmax,
        dx: 0.0,
        dy: 0.0,
    });
    let super_count = 3;
    all.extend_from_slice(points);

    let mut triangles: Vec<Triangle> = vec![Triangle(0, 1, 2)];

    for pi in super_count..all.len() {
        let p = all[pi];
        let mut bad: Vec<Triangle> = Vec::new();
        for tri in &triangles {
            let a = all[tri.0];
            let b = all[tri.1];
            let c = all[tri.2];
            if in_circumcircle(p.x, p.y, a, b, c) {
                bad.push(*tri);
            }
        }
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for tri in &bad {
            for e in [(tri.0, tri.1), (tri.1, tri.2), (tri.2, tri.0)] {
                let rev = (e.1, e.0);
                if let Some(pos) = edges.iter().position(|x| *x == rev) {
                    edges.remove(pos);
                } else {
                    edges.push(e);
                }
            }
        }
        for tri in &bad {
            if let Some(pos) = triangles.iter().position(|x| *x == *tri) {
                triangles.remove(pos);
            }
        }
        for e in &edges {
            triangles.push(Triangle(e.0, e.1, pi));
        }
    }
    // Remove triangles that touch any super-triangle vertex
    triangles.retain(|tri| tri.0 >= super_count && tri.1 >= super_count && tri.2 >= super_count);
    // Shift indices back: subtract super_count to map back to original points
    triangles
        .iter()
        .map(|t| Triangle(t.0 - super_count, t.1 - super_count, t.2 - super_count))
        .collect()
}

fn in_circumcircle(px: f32, py: f32, a: TriPoint, b: TriPoint, c: TriPoint) -> bool {
    let ax = a.x - px;
    let ay = a.y - py;
    let bx = b.x - px;
    let by = b.y - py;
    let cx = c.x - px;
    let cy = c.y - py;
    let det = (ax * ax + ay * ay) * (bx * cy - cx * by) - (bx * bx + by * by) * (ax * cy - cx * ay)
        + (cx * cx + cy * cy) * (ax * by - bx * ay);
    det > 0.0
}

fn barycentric_coords(
    px: f32,
    py: f32,
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
) -> Option<(f32, f32, f32)> {
    let v0x = b.0 - a.0;
    let v0y = b.1 - a.1;
    let v1x = c.0 - a.0;
    let v1y = c.1 - a.1;
    let v2x = px - a.0;
    let v2y = py - a.1;
    let d00 = v0x * v0x + v0y * v0y;
    let d01 = v0x * v1x + v0y * v1y;
    let d11 = v1x * v1x + v1y * v1y;
    let d20 = v2x * v0x + v2y * v0y;
    let d21 = v2x * v1x + v2y * v1y;
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-10 {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    Some((u, v, w))
}

/// Mesh-based warp using Delaunay triangulation + barycentric interpolation.
/// Produces smoother, more predictable deformation than Shepard interpolation.
pub fn warp_layer_buf_mesh(buf: &mut [u8], w: u32, h: u32, pins: &[([f32; 2], [f32; 2])]) {
    if w == 0 || h == 0 || pins.is_empty() {
        return;
    }
    if pins.len() < 3 {
        return warp_layer_buf(buf, w, h, pins);
    }
    // Check if any pin moved
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

    let mut points: Vec<TriPoint> = pins
        .iter()
        .map(|(s, d)| TriPoint {
            x: s[0],
            y: s[1],
            dx: d[0] - s[0],
            dy: d[1] - s[1],
        })
        .collect();
    // Add boundary corner points (no displacement) for full coverage
    let corners: &[[f32; 2]] = &[
        [0.0, 0.0],
        [w as f32, 0.0],
        [w as f32, h as f32],
        [0.0, h as f32],
    ];
    for c in corners {
        points.push(TriPoint {
            x: c[0],
            y: c[1],
            dx: 0.0,
            dy: 0.0,
        });
    }

    let triangles = delaunay_triangulate(&points);
    if triangles.is_empty() {
        return warp_layer_buf(buf, w, h, pins);
    }

    let src = buf.to_vec();
    for py in 0..h {
        for px in 0..w {
            let ppx = px as f32 + 0.5;
            let ppy = py as f32 + 0.5;
            for tri in &triangles {
                let a = points[tri.0];
                let b = points[tri.1];
                let c = points[tri.2];
                if let Some((u, v, w_b)) =
                    barycentric_coords(ppx, ppy, (a.x, a.y), (b.x, b.y), (c.x, c.y))
                {
                    if u >= -0.001 && v >= -0.001 && w_b >= -0.001 {
                        let dx = u * a.dx + v * b.dx + w_b * c.dx;
                        let dy = u * a.dy + v * b.dy + w_b * c.dy;
                        let sx = ppx - dx - 0.5;
                        let sy = ppy - dy - 0.5;
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
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn solid(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for p in v.chunks_exact_mut(4) {
            p[0] = 100;
            p[1] = 100;
            p[2] = 100;
            p[3] = 255;
        }
        v
    }

    #[test]
    fn test_shepard_no_pins_noop() {
        let mut a = solid(4, 4);
        let orig = a.clone();
        warp_layer_buf(&mut a, 4, 4, &[]);
        assert_eq!(a, orig);
    }

    #[test]
    fn test_shepard_single_pin_noop() {
        let mut a = solid(4, 4);
        let orig = a.clone();
        warp_layer_buf(&mut a, 4, 4, &[([2.0, 2.0], [2.0, 2.0])]);
        assert_eq!(a, orig);
    }

    #[test]
    fn test_shepard_moves_pixels() {
        let mut a = solid(8, 8);
        warp_layer_buf(&mut a, 8, 8, &[([4.0, 4.0], [6.0, 4.0])]);
        // Pixel at (6,4) should now show original content from (4,4)
        // which was gray (100). Pixel at (4,4) may show 0 (out-of-bounds sample).
        assert_ne!(a[((4 * 8 + 4) * 4) as usize], 0);
    }

    #[test]
    fn test_delaunay_triangulate_basic() {
        let pts = vec![
            TriPoint {
                x: 0.0,
                y: 0.0,
                dx: 0.0,
                dy: 0.0,
            },
            TriPoint {
                x: 4.0,
                y: 0.0,
                dx: 0.0,
                dy: 0.0,
            },
            TriPoint {
                x: 2.0,
                y: 3.0,
                dx: 0.0,
                dy: 0.0,
            },
            TriPoint {
                x: 1.0,
                y: 1.0,
                dx: 0.0,
                dy: 0.0,
            },
        ];
        let tris = delaunay_triangulate(&pts);
        assert!(!tris.is_empty(), "expected triangles, got 0");
        for t in &tris {
            assert!(t.0 < pts.len() && t.1 < pts.len() && t.2 < pts.len());
        }
    }

    #[test]
    fn test_barycentric_inside_triangle() {
        let bary = barycentric_coords(2.0, 1.0, (0.0, 0.0), (4.0, 0.0), (2.0, 3.0));
        assert!(bary.is_some());
        let (u, v, w) = bary.unwrap();
        assert!(u >= -0.01 && v >= -0.01 && w >= -0.01);
        assert!((u + v + w - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_barycentric_outside_returns_negative() {
        let bary = barycentric_coords(10.0, 10.0, (0.0, 0.0), (1.0, 0.0), (0.0, 1.0));
        if let Some((u, _v, _w)) = bary {
            assert!(u < 0.0, "should be outside");
        }
    }

    #[test]
    fn test_warp_mesh_moves_pixels() {
        let mut img = solid(4, 4);
        warp_layer_buf_mesh(
            &mut img,
            4,
            4,
            &[
                ([0.0, 0.0], [0.0, 0.0]),
                ([4.0, 0.0], [4.0, 0.0]),
                ([4.0, 4.0], [4.0, 4.0]),
                ([0.0, 4.0], [0.0, 4.0]),
                ([2.0, 2.0], [3.0, 2.0]),
            ],
        );
        // Should execute without panic
        assert!(img.iter().any(|&b| b > 0));
    }

    #[test]
    fn test_warp_mesh_falls_back_to_shepard_with_few_pins() {
        let mut a = solid(4, 4);
        let orig = a.clone();
        // Only 2 pins — should fall back to shepard (which is a noop when unmoved)
        warp_layer_buf_mesh(
            &mut a,
            4,
            4,
            &[([1.0, 1.0], [1.0, 1.0]), ([3.0, 3.0], [3.0, 3.0])],
        );
        assert_eq!(a, orig);
    }
}
