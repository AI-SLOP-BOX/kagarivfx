//! Vector Graphics (SVG / Illustrator) Path & Anchor Point Importer.
//!
//! Parses vector commands (MoveTo, LineTo, Cubic Bezier, ClosePath) into fully animatable
//! AE MaskVertex and MaskPath structures.

#![allow(dead_code)]

use crate::core::mask::{MaskPath, MaskVertex};

/// Imported vector path with style information.
#[derive(Debug, Clone)]
pub struct SvgVectorPath {
    pub name: String,
    pub vertices: Vec<MaskVertex>,
    pub is_closed: bool,
    pub fill_color: Option<[f32; 4]>,
    pub stroke_color: Option<[f32; 4]>,
    pub stroke_width: f32,
}

impl SvgVectorPath {
    /// Converts imported vector path into an animatable AE MaskPath.
    pub fn to_mask_path(&self) -> MaskPath {
        let mut path = MaskPath::new_closed(self.vertices.iter().map(|v| v.position).collect());
        let tangents = self
            .vertices
            .iter()
            .map(|v| (v.tangent_in, v.tangent_out))
            .collect();
        path.tangents = Some(tangents);
        path.is_closed = self.is_closed;
        path
    }
}

/// Parses an SVG path definition string (`d="..."`) into a list of `MaskVertex` points.
pub fn parse_svg_path_data(d: &str) -> Result<Vec<MaskVertex>, String> {
    let mut vertices: Vec<MaskVertex> = Vec::new();
    let mut curr_pos = [0.0f32, 0.0f32];

    // Tokenize command letters and numbers
    let mut tokens = Vec::new();
    let mut curr_token = String::new();

    for c in d.chars() {
        if c.is_alphabetic() {
            if !curr_token.trim().is_empty() {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
            tokens.push(c.to_string());
        } else if c.is_whitespace() || c == ',' {
            if !curr_token.trim().is_empty() {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
        } else {
            curr_token.push(c);
        }
    }
    if !curr_token.trim().is_empty() {
        tokens.push(curr_token.trim().to_string());
    }

    let mut i = 0;
    while i < tokens.len() {
        let cmd = &tokens[i];
        match cmd.as_str() {
            "M" | "m" => {
                let is_rel = cmd == "m";
                if i + 2 < tokens.len() {
                    let x: f32 = tokens[i + 1].parse().map_err(|e| format!("M.x: {e}"))?;
                    let y: f32 = tokens[i + 2].parse().map_err(|e| format!("M.y: {e}"))?;
                    curr_pos = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };
                    vertices.push(MaskVertex::new(curr_pos[0], curr_pos[1]));
                    i += 3;
                } else {
                    break;
                }
            }
            "L" | "l" => {
                let is_rel = cmd == "l";
                if i + 2 < tokens.len() {
                    let x: f32 = tokens[i + 1].parse().map_err(|e| format!("L.x: {e}"))?;
                    let y: f32 = tokens[i + 2].parse().map_err(|e| format!("L.y: {e}"))?;
                    curr_pos = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };
                    vertices.push(MaskVertex::new(curr_pos[0], curr_pos[1]));
                    i += 3;
                } else {
                    break;
                }
            }
            "C" | "c" => {
                let is_rel = cmd == "c";
                if i + 6 < tokens.len() {
                    let x1: f32 = tokens[i + 1].parse().map_err(|e| format!("C.x1: {e}"))?;
                    let y1: f32 = tokens[i + 2].parse().map_err(|e| format!("C.y1: {e}"))?;
                    let x2: f32 = tokens[i + 3].parse().map_err(|e| format!("C.x2: {e}"))?;
                    let y2: f32 = tokens[i + 4].parse().map_err(|e| format!("C.y2: {e}"))?;
                    let x: f32 = tokens[i + 5].parse().map_err(|e| format!("C.x: {e}"))?;
                    let y: f32 = tokens[i + 6].parse().map_err(|e| format!("C.y: {e}"))?;

                    let c0 = if is_rel {
                        [curr_pos[0] + x1, curr_pos[1] + y1]
                    } else {
                        [x1, y1]
                    };
                    let c1 = if is_rel {
                        [curr_pos[0] + x2, curr_pos[1] + y2]
                    } else {
                        [x2, y2]
                    };
                    let dest = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };

                    // Update tangent_out of previous vertex
                    if let Some(prev) = vertices.last_mut() {
                        prev.tangent_out = [c0[0] - prev.position[0], c0[1] - prev.position[1]];
                    }

                    // Create destination vertex with tangent_in
                    let mut dest_vert = MaskVertex::new(dest[0], dest[1]);
                    dest_vert.tangent_in = [c1[0] - dest[0], c1[1] - dest[1]];
                    vertices.push(dest_vert);

                    curr_pos = dest;
                    i += 7;
                } else {
                    break;
                }
            }
            "Z" | "z" => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(vertices)
}

/// Extracts all vector shapes from an SVG file string.
pub fn parse_svg_document(svg_text: &str) -> Vec<SvgVectorPath> {
    let mut paths = Vec::new();

    for line in svg_text.lines() {
        if line.contains("<path") {
            if let Some(d_start) = line.find("d=\"") {
                let rest = &line[d_start + 3..];
                if let Some(d_end) = rest.find('"') {
                    let d = &rest[..d_end];
                    if let Ok(verts) = parse_svg_path_data(d) {
                        if !verts.is_empty() {
                            paths.push(SvgVectorPath {
                                name: format!("Path {}", paths.len() + 1),
                                vertices: verts,
                                is_closed: line.contains('Z') || line.contains('z'),
                                fill_color: Some([1.0, 1.0, 1.0, 1.0]),
                                stroke_color: None,
                                stroke_width: 1.0,
                            });
                        }
                    }
                }
            }
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svg_cubic_bezier_path() {
        let svg_d = "M 10 20 C 30 40 50 60 70 80 Z";
        let verts = parse_svg_path_data(svg_d).expect("SVG path parsing succeeds");
        assert_eq!(verts.len(), 2);
        assert_eq!(verts[0].position, [10.0, 20.0]);
        assert_eq!(verts[0].tangent_out, [20.0, 20.0]); // (30 - 10, 40 - 20)
        assert_eq!(verts[1].position, [70.0, 80.0]);
        assert_eq!(verts[1].tangent_in, [-20.0, -20.0]); // (50 - 70, 60 - 80)
    }
}
