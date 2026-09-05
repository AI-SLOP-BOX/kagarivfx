//! External Camera Calibration & 3D Tracking Data Importer (.chan / Nuke / 3D Trackers).
//!
//! Parses ASCII .chan motion tracking data and bakes position, rotation, and focal length
//! keyframes directly into animatable Camera3D layers.

#![allow(dead_code)]

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::timeline::Camera3D;

/// Single frame entry from an external .chan camera track.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanCameraFrame {
    pub frame: u32,
    pub tx: f32,
    pub ty: f32,
    pub tz: f32,
    pub rx: f32,
    pub ry: f32,
    pub rz: f32,
    pub fov_deg: f32,
}

/// Parses ASCII `.chan` format lines (frame tx ty tz rx ry rz fov/focal).
pub fn parse_chan_data(chan_text: &str) -> Result<Vec<ChanCameraFrame>, String> {
    let mut frames = Vec::new();

    for line in chan_text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 7 {
            let frame: f32 = parts[0]
                .parse()
                .map_err(|e| format!("Invalid frame: {e}"))?;
            let tx: f32 = parts[1].parse().map_err(|e| format!("Invalid tx: {e}"))?;
            let ty: f32 = parts[2].parse().map_err(|e| format!("Invalid ty: {e}"))?;
            let tz: f32 = parts[3].parse().map_err(|e| format!("Invalid tz: {e}"))?;
            let rx: f32 = parts[4].parse().map_err(|e| format!("Invalid rx: {e}"))?;
            let ry: f32 = parts[5].parse().map_err(|e| format!("Invalid ry: {e}"))?;
            let rz: f32 = parts[6].parse().map_err(|e| format!("Invalid rz: {e}"))?;

            let fov_deg = if parts.len() >= 8 {
                parts[7].parse().unwrap_or(50.0)
            } else {
                50.0
            };

            frames.push(ChanCameraFrame {
                frame: frame.round().max(0.0) as u32,
                tx,
                ty,
                tz,
                rx,
                ry,
                rz,
                fov_deg,
            });
        }
    }

    Ok(frames)
}

/// Bakes parsed .chan camera track frames into an existing Camera3D structure.
pub fn bake_chan_to_camera(frames: &[ChanCameraFrame], camera: &mut Camera3D) {
    if frames.is_empty() {
        return;
    }

    let mut pos_kfs = Vec::new();
    let mut fov_kfs = Vec::new();

    for f in frames {
        pos_kfs.push(Keyframe::new(
            f.frame,
            [f.tx, f.ty, f.tz],
            InterpolationType::Linear,
        ));
        fov_kfs.push(Keyframe::new(f.frame, f.fov_deg, InterpolationType::Linear));
    }

    camera.transform.position = crate::core::property::Animatable::new_animated(pos_kfs);
    camera.fov_animation = Some(crate::core::property::Animatable::new_animated(fov_kfs));
    if let Some(first) = frames.first() {
        camera.fov_degrees = first.fov_deg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_bake_chan_camera_track() {
        let chan_data = "\
1 0.0 100.0 500.0 0.0 0.0 0.0 45.0
2 10.0 105.0 490.0 1.0 0.0 0.0 45.0
";
        let frames = parse_chan_data(chan_data).expect("Parsing .chan succeeds");
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].tx, 10.0);

        let mut camera = Camera3D::default();
        bake_chan_to_camera(&frames, &mut camera);
        assert_eq!(camera.transform.position.evaluate(1), [0.0, 100.0, 500.0]);
        assert_eq!(camera.transform.position.evaluate(2), [10.0, 105.0, 490.0]);
        assert_eq!(camera.fov_animation.as_ref().unwrap().evaluate(1), 45.0);
    }
}
