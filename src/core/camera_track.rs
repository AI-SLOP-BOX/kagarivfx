//! Blender camera-track interchange: load keyframed camera solves exported
//! by `tools/blender_camera_export.py` (pure-OSS workflow — no addons, just
//! run the script inside Blender's text editor) and bake them onto the
//! active composition camera as position/rotation keyframes.
//!
//! JSON schema:
//! {
//!   "fps": 30,
//!   "up_axis": "Z",            // "Z" (Blender default) or "Y"
//!   "scale": 1.0,              // multiply positions (e.g. 0.01 cm→m)
//!   "frames": [
//!     {"frame": 0, "pos": [x,y,z], "rot_deg": [rx,ry,rz], "fov": 42.0}
//!   ]
//! }
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Camera3D, Composition};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BlenderCamTrack {
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default)]
    pub up_axis: String,
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Optional horizontal FOV exported by the Blender script.
    #[serde(default)]
    pub fov: Option<f64>,
    pub frames: Vec<TrackFrame>,
}

fn default_fps() -> u32 { 30 }
fn default_scale() -> f32 { 1.0 }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TrackFrame {
    pub frame: u32,
    pub pos: [f64; 3],
    #[serde(default)]
    pub rot_deg: [f64; 3],
    #[serde(default)]
    pub fov: Option<f64>,
}

impl BlenderCamTrack {
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Bad camera track JSON: {}", e))
    }

    /// Bake onto `comp.active_camera`. Returns number of baked keyframes.
    /// When `match_comp_fps` the track's own timeline is rescaled from its
    /// source fps to the composition fps so playback stays in sync.
    pub fn apply_to_comp(&self, comp: &mut Composition, match_comp_fps: bool) -> usize {
        let rate = if match_comp_fps && self.fps > 0 {
            comp.fps.max(1) as f32 / self.fps as f32
        } else {
            1.0
        };
        let z_up = self.up_axis.eq_ignore_ascii_case("Z");

        let mut pos_kfs: Vec<Keyframe<[f32; 3]>> = Vec::with_capacity(self.frames.len());
        let mut rot_kfs: Vec<Keyframe<[f32; 3]>> = Vec::with_capacity(self.frames.len());
        let mut last_fov: Option<f32> = None;

        for tf in &self.frames {
            let f = ((tf.frame as f32 * rate).round() as u32).min(comp.duration_frames.saturating_sub(1));
            let mut p = [tf.pos[0] as f32 * self.scale,
                         tf.pos[1] as f32 * self.scale,
                         tf.pos[2] as f32 * self.scale];
            let mut r = [tf.rot_deg[0] as f32, tf.rot_deg[1] as f32, tf.rot_deg[2] as f32];
            // Blender Z-up -> app Y-up: (x, y, z) -> (x, -z, y); same axis
            // permutation applied to euler components (documented v1 limit).
            if z_up {
                p = [p[0], -p[2], p[1]];
                r = [r[0], -r[2], r[1]];
            }
            pos_kfs.push(Keyframe::new(f, p, InterpolationType::Linear));
            rot_kfs.push(Keyframe::new(f, r, InterpolationType::Linear));
            if let Some(fov) = tf.fov {
                last_fov = Some(fov as f32);
            }
        }
        pos_kfs.sort_by_key(|k| k.frame);
        rot_kfs.sort_by_key(|k| k.frame);
        pos_kfs.dedup_by(|a, b| a.frame == b.frame);
        rot_kfs.dedup_by(|a, b| a.frame == b.frame);

        let cam: &mut Camera3D = &mut comp.active_camera;
        cam.active = true;
        if !pos_kfs.is_empty() {
            cam.transform.position = Animatable::new_animated(pos_kfs);
        }
        let baked = rot_kfs.len();
        if !rot_kfs.is_empty() {
            cam.transform.rotation = Animatable::new_animated(rot_kfs);
        }
        if let Some(fov) = last_fov {
            cam.fov_degrees = fov.clamp(5.0, 170.0);
        }
        baked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "fps": 24,
        "up_axis": "Z",
        "scale": 0.01,
        "frames": [
            {"frame": 0,   "pos": [0, 0, 500],  "rot_deg": [0, 0, 0]},
            {"frame": 10,  "pos": [100, 50, 450], "rot_deg": [5, -10, 0]}
        ]
    }"#;

    #[test]
    fn test_parse_and_count() {
        let t = BlenderCamTrack::parse(SAMPLE).unwrap();
        assert_eq!(t.frames.len(), 2);
        assert_eq!(t.fps, 24);
        assert!(t.up_axis.eq_ignore_ascii_case("z"));
    }

    #[test]
    fn test_apply_bakes_axis_swap_and_scale() {
        let t = BlenderCamTrack::parse(SAMPLE).unwrap();
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 24, 100);
        let n = t.apply_to_comp(&mut comp, false);
        assert_eq!(n, 2);
        let pos0 = comp.active_camera.transform.position.evaluate(0);
        // Z-up swap: [0,0,500]*0.01 -> [0, -5.0, 0]
        assert!((pos0[0]).abs() < 1e-4);
        assert!((pos0[1] + 5.0).abs() < 1e-4);
        assert!((pos0[2]).abs() < 1e-4);
        let rot1 = comp.active_camera.transform.rotation.evaluate(10);
        // swap: [rx,-rz,ry] -> y'=0, z'=orig ry=-10
        assert!(rot1[1].abs() < 1e-4);
        assert!((rot1[2] + 10.0).abs() < 1e-4, "z'=-? mapping check");
    }

    #[test]
    fn test_fps_rescale_matches_comp() {
        let t = BlenderCamTrack::parse(SAMPLE).unwrap(); // src 24fps
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 48, 100); // dst 48fps
        t.apply_to_comp(&mut comp, true);
        let kfs = comp.active_camera.transform.rotation.keyframes().unwrap();
        assert_eq!(kfs.last().unwrap().frame, 20, "frame 10 @24fps -> 20 @48fps");
    }
}
