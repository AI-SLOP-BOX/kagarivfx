use crate::core::property::Animatable;
use crate::core::expression_engine;
use crate::core::text_animator_advanced::AnimatorStack;
use serde::{Deserialize, Serialize};

/// Bounding box: (min_xy, max_xy, width, height)
type BoundingBox = ([f32; 2], [f32; 2], f32, f32);

// Thread-local cached Rhai engine — building it is expensive (~1ms), reusing is free.
thread_local! {
    static RHAI_ENGINE: rhai::Engine = expression_engine::build_engine();
}

// ─── Tracker Point ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerPoint {
    pub id: String,
    pub name: String,
    pub position: Animatable<[f32; 2]>,
    pub search_size: f32,
    pub feature_size: f32,
    pub reference_pattern: Option<Vec<f32>>,
}

impl TrackerPoint {
    pub fn new(id: String, name: String, initial_pos: [f32; 2]) -> Self {
        Self {
            id,
            name,
            position: Animatable::new_constant(initial_pos),
            search_size: 25.0,
            feature_size: 10.0,
            reference_pattern: None,
        }
    }
}

// ─── Serde default helpers for LayerType::Text new fields ────────────────

fn default_font_family() -> String { "Inter".to_string() }
fn default_leading() -> f32 { 1.2 }
fn default_stroke_color() -> [f32; 4] { [0.0, 0.0, 0.0, 1.0] }
fn default_video_speed() -> f32 { 1.0 }

// ─── Layer Type ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    Solid {
        color: [f32; 4],
    },
    Image {
        path: String,
    },
    /// Video layer: plays a pre-extracted frame sequence (see video_import).
    /// `frames_dir` holds frame_%05d.png files decoded at import time; rendering
    /// samples the sequence by the layer's effective frame.
    Video {
        source: String,
        frames_dir: String,
        frame_count: u32,
        #[serde(default)]
        audio_wav: Option<String>,
        /// Playback speed multiplier: 1.0 = normal, 2.0 = double speed
        #[serde(default = "default_video_speed")]
        speed: f32,
    },
    Text {
        text: String,
        font_size: u32,
        color: [f32; 4],
        /// Font family name, e.g. "Inter"
        #[serde(default = "default_font_family")]
        font_family: String,
        /// Tracking (letter-spacing) in virtual Adobe units
        #[serde(default)]
        tracking: f32,
        /// Leading (line-height) multiplier
        #[serde(default = "default_leading")]
        leading: f32,
        /// Paragraph alignment: 0=Left, 1=Center, 2=Right
        #[serde(default)]
        align: usize,
        /// Stroke color [r,g,b,a]
        #[serde(default = "default_stroke_color")]
        stroke_color: [f32; 4],
        /// Stroke width in pixels (0 = no stroke)
        #[serde(default)]
        stroke_width: f32,
        /// Text on Path: use first mask as text path
        #[serde(default)]
        text_on_path: bool,
    },
    Shape {
        shape_type: ShapeType,
        color: [f32; 4],
        stroke_color: [f32; 4],
        stroke_width: f32,
        #[serde(default)]
        fill_type: ShapeFillType,
    },
    Null,
    PreComp {
        comp_id: String,
    },
    Audio {
        path: String,
        volume: Animatable<f32>,
    },
    /// Adjustment Layer: applies layer effects to the composited buffer of lower layers
    AdjustmentLayer,
    /// Particle emitter layer: procedural particle simulation
    Particle {
        #[serde(default)]
        emitter: crate::core::particle_system::ParticleEmitter,
    },
}

// ─── Trim Paths Animator ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimPaths {
    /// Start percentage (0.0 .. 100.0)
    pub start: Animatable<f32>,
    /// End percentage (0.0 .. 100.0)
    pub end: Animatable<f32>,
    /// Offset degrees (0.0 .. 360.0)
    pub offset: Animatable<f32>,
}

impl Default for TrimPaths {
    fn default() -> Self {
        Self {
            start: Animatable::new_constant(0.0),
            end: Animatable::new_constant(100.0),
            offset: Animatable::new_constant(0.0),
        }
    }
}

impl TrimPaths {
    /// Trims a list of 2D polygon points based on start/end/offset percentages.
    pub fn trim_polygon(&self, points: &[[f32; 2]], frame: u32) -> Vec<[f32; 2]> {
        if points.len() < 2 {
            return points.to_vec();
        }

        let mut start_pct = self.start.evaluate(frame).clamp(0.0, 100.0) / 100.0;
        let mut end_pct = self.end.evaluate(frame).clamp(0.0, 100.0) / 100.0;
        let offset_norm = (self.offset.evaluate(frame) / 360.0).fract();

        start_pct = (start_pct + offset_norm).fract();
        end_pct = (end_pct + offset_norm).fract();
        if start_pct < 0.0 { start_pct += 1.0; }
        if end_pct < 0.0 { end_pct += 1.0; }

        if (start_pct - end_pct).abs() < 0.001 {
            return Vec::new();
        }

        let mut segment_lengths = Vec::with_capacity(points.len() - 1);
        let mut total_len = 0.0f32;

        for w in points.windows(2) {
            let dx = w[1][0] - w[0][0];
            let dy = w[1][1] - w[0][1];
            let len = (dx * dx + dy * dy).sqrt();
            segment_lengths.push(len);
            total_len += len;
        }

        if total_len <= 0.0001 {
            return points.to_vec();
        }

        let target_start = start_pct.min(end_pct) * total_len;
        let target_end = start_pct.max(end_pct) * total_len;

        let mut current_accum = 0.0f32;
        let mut trimmed = Vec::new();

        for i in 0..segment_lengths.len() {
            let seg_len = segment_lengths[i];
            let seg_start = current_accum;
            let seg_end = current_accum + seg_len;

            if seg_end >= target_start && seg_start <= target_end {
                let p0 = points[i];
                let p1 = points[i + 1];

                // Interpolate start point
                let t_start = if target_start > seg_start {
                    ((target_start - seg_start) / seg_len).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                // Interpolate end point
                let t_end = if target_end < seg_end {
                    ((target_end - seg_start) / seg_len).clamp(0.0, 1.0)
                } else {
                    1.0
                };

                let sub_p0 = [p0[0] + (p1[0] - p0[0]) * t_start, p0[1] + (p1[1] - p0[1]) * t_start];
                let sub_p1 = [p0[0] + (p1[0] - p0[0]) * t_end, p0[1] + (p1[1] - p0[1]) * t_end];

                if trimmed.is_empty() {
                    trimmed.push(sub_p0);
                }
                trimmed.push(sub_p1);
            }

            current_accum = seg_end;
        }

        trimmed
    }
}


impl LayerType {
    /// Convenience constructor for a new text layer with sensible defaults.
    pub fn new_text(text: impl Into<String>, font_size: u32, color: [f32; 4]) -> Self {
        LayerType::Text {
            text: text.into(),
            font_size,
            color,
            font_family: "Inter".to_string(),
            tracking: 0.0,
            leading: 1.2,
            align: 0,
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
            text_on_path: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ShapeFillType {
    #[default]
    Solid,
    LinearGradient {
        start: [f32; 2],
        end: [f32; 2],
        colors: Vec<[f32; 4]>,
        stops: Vec<f32>,
    },
    RadialGradient {
        center: [f32; 2],
        radius: f32,
        colors: Vec<[f32; 4]>,
        stops: Vec<f32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeType {
    /// Freeform Bezier path with control points.
    FreeformBezier {
        /// Flat list of [x, y] coordinates (closed path if first == last).
        points: Vec<[f32; 2]>,
        /// Tangent handles: pairs of (in_tangent, out_tangent) per point.
        tangents: Vec<([f32; 2], [f32; 2])>,
        closed: bool,
    },
    Rectangle {
        width: Animatable<f32>,
        height: Animatable<f32>,
        corner_radius: Animatable<f32>,
    },
    Ellipse {
        width: Animatable<f32>,
        height: Animatable<f32>,
    },
    Star {
        points: Animatable<f32>,
        inner_radius: Animatable<f32>,
        outer_radius: Animatable<f32>,
    },
    Polygon {
        sides: Animatable<f32>,
        radius: Animatable<f32>,
    },
}

// ─── Track Matte ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TrackMatteMode {
    #[default]
    None,
    AlphaMatte,
    AlphaMatteInverted,
    LumaMatte,
    LumaMatteInverted,
}

// ─── AE Blend Modes ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Add,
    Darken,
    Lighten,
    SoftLight,
    HardLight,
    Difference,
    Exclusion,
    Divide,
    Subtract,
    ColorBurn,
    LinearBurn,
    VividLight,
    ColorDodge,
    LinearDodge,
    Color,
    Hue,
    Saturation,
    Luminosity,
    StencilAlpha,
    StencilLuma,
    SilhouetteAlpha,
    SilhouetteLuma,
    Behind,
    AlphaAdd,
    LinearLight,
}

// ─── Label Colors ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LabelColor {
    #[default]
    None,
    Red,
    Yellow,
    Aqua,
    Pink,
    Lavender,
    Peach,
    Sea,
    Blue,
    Purple,
}

impl LabelColor {
    pub fn to_rgb(self) -> [f32; 3] {
        match self {
            LabelColor::None      => [0.35, 0.35, 0.35],
            LabelColor::Red       => [0.90, 0.25, 0.25],
            LabelColor::Yellow    => [0.95, 0.85, 0.10],
            LabelColor::Aqua      => [0.20, 0.85, 0.80],
            LabelColor::Pink      => [0.95, 0.45, 0.70],
            LabelColor::Lavender  => [0.65, 0.55, 0.90],
            LabelColor::Peach     => [0.95, 0.70, 0.45],
            LabelColor::Sea       => [0.30, 0.70, 0.55],
            LabelColor::Blue      => [0.35, 0.55, 0.95],
            LabelColor::Purple    => [0.70, 0.35, 0.90],
        }
    }
}

// ─── Expression Engine ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Wiggle { frequency: f32, amplitude: f32 },
    LoopOut,
    PingPong,
    TimeDriver { multiplier: f32, offset: f32 },
    Raw(String),
}

impl Expression {
    pub fn evaluate_f32(&self, base: f32, frame: u32, fps: u32) -> f32 {
        let time = frame as f32 / fps.max(1) as f32;
        match self {
            Expression::Wiggle { frequency, amplitude } => {
                let seed = (frame as f32 * frequency * 1.618_034) % std::f32::consts::TAU;
                let noise = seed.sin() * 0.7 + (seed * 2.1).sin() * 0.2 + (seed * 5.3).sin() * 0.1;
                base + noise * amplitude
            }
            Expression::TimeDriver { multiplier, offset } => {
                time * multiplier + offset
            }
            // Raw Rhai script — evaluated by the Rhai expression engine.
            // Supports full AE-compatible API: time, frame, fps, value, wiggle(), etc.
            Expression::Raw(script) => {
                RHAI_ENGINE.with(|engine| {
                    expression_engine::eval_f32(engine, script, base, frame, fps)
                })
            }
            _ => base,
        }
    }

    pub fn evaluate_v2(&self, base: [f32; 2], frame: u32, fps: u32) -> [f32; 2] {
        let time = frame as f32 / fps.max(1) as f32;
        match self {
            Expression::Wiggle { frequency, amplitude } => {
                let t = frame as f32 * frequency * 1.618_034;
                let nx = t.sin() * 0.7 + (t * 2.1).sin() * 0.2 + (t * 5.3).sin() * 0.1;
                let ny = (t + 100.0).sin() * 0.7 + ((t + 100.0) * 2.1).sin() * 0.2;
                [base[0] + nx * amplitude, base[1] + ny * amplitude]
            }
            Expression::TimeDriver { multiplier, offset } => {
                [base[0] + time * multiplier + offset, base[1]]
            }
            // Raw Rhai script — evaluated by the Rhai expression engine.
            Expression::Raw(script) => {
                RHAI_ENGINE.with(|engine| {
                    expression_engine::eval_v2(engine, script, base, frame, fps)
                })
            }
            _ => base,
        }
    }
}

// ─── Transform ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform2D {
    pub anchor_point: Animatable<[f32; 2]>,
    pub position: Animatable<[f32; 2]>,
    pub scale: Animatable<[f32; 2]>,
    pub rotation: Animatable<f32>,
    pub opacity: Animatable<f32>,

    #[serde(default)]
    pub anchor_point_expression: Option<Expression>,
    pub position_expression: Option<Expression>,
    pub rotation_expression: Option<Expression>,
    pub scale_expression: Option<Expression>,
    pub opacity_expression: Option<Expression>,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            anchor_point: Animatable::new_constant([0.0, 0.0]),
            position: Animatable::new_constant([0.0, 0.0]),
            scale: Animatable::new_constant([100.0, 100.0]),
            rotation: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            anchor_point_expression: None,
            position_expression: None,
            rotation_expression: None,
            scale_expression: None,
            opacity_expression: None,
        }
    }
}

/// Helper to map an extended frame number according to keyframe loop settings (LoopOut / PingPong).
pub fn remap_frame_for_loop(frame: u32, first_kf: u32, last_kf: u32, is_pingpong: bool) -> u32 {
    if first_kf >= last_kf || frame <= last_kf {
        return frame;
    }
    let span = last_kf - first_kf;
    if span == 0 {
        return first_kf;
    }
    let offset = frame - first_kf;
    let cycle_idx = offset / span;
    let rem = offset % span;

    if is_pingpong && (cycle_idx % 2 == 1) {
        last_kf - rem
    } else {
        first_kf + rem
    }
}

#[derive(Clone, Copy)]
enum LoopProp {
    Position,
    Scale,
    Rotation,
    Opacity,
}

impl Transform2D {
    fn remap_loop_frame(frame: u32, expression: &Option<Expression>, first_frame: u32, last_frame: u32) -> u32 {
        match expression {
            Some(Expression::LoopOut) => remap_frame_for_loop(frame, first_frame, last_frame, false),
            Some(Expression::PingPong) => remap_frame_for_loop(frame, first_frame, last_frame, true),
            _ => frame,
        }
    }

    pub fn eval_position(&self, frame: u32, fps: u32) -> [f32; 2] {
        let eval_frame = if let Some(kfs) = self.position.keyframes() {
            match (kfs.first(), kfs.last()) {
                (Some(first), Some(last)) => {
                    Self::remap_loop_frame(frame, &self.position_expression, first.frame, last.frame)
                }
                _ => frame,
            }
        } else { frame };
        let base = self.position.evaluate(eval_frame);
        match &self.position_expression {
            Some(Expression::Raw(script)) if expression_engine::script_uses_loops(script) => {
                let loops = self.compute_loop_vals(frame, fps, LoopProp::Position);
                expression_engine::eval_v2_with_loops(script, base, frame, fps, loops)
            }
            Some(expr) => expr.evaluate_v2(base, eval_frame, fps),
            None => base,
        }
    }

    /// Computes loopOut/loopIn reference values from this property's keyframes for Raw scripts.
    fn compute_loop_vals(&self, frame: u32, _fps: u32, prop: LoopProp) -> expression_engine::LoopVals {
        use crate::core::expression_engine::LoopVals;
        let mut vals = LoopVals::default();
        // Sample the animatable at one cycle past the last keyframe for each loop mode
        let mut compute = |kfs: &[crate::core::keyframe::Keyframe<[f32; 2]>], sample: &dyn Fn(u32) -> [f32; 2]| {
            if let (Some(first), Some(last)) = (kfs.first(), kfs.last()) {
                let first = first.frame;
                let last = last.frame;
                let out_c = sample(remap_frame_for_loop(frame, first, last, false));
                let out_p = sample(remap_frame_for_loop(frame, first, last, true));
                vals.out_cycle = out_c[0];
                vals.out_pingpong = out_p[0];
                vals.in_cycle = out_c[1];
                vals.in_pingpong = out_p[1];
            }
        };
        match prop {
            LoopProp::Position => {
                if let Some(kfs) = self.position.keyframes() {
                    compute(kfs, &|f| self.position.evaluate(f));
                }
            }
            LoopProp::Scale => {
                if let Some(kfs) = self.scale.keyframes() {
                    compute(kfs, &|f| self.scale.evaluate(f));
                }
            }
            LoopProp::Rotation | LoopProp::Opacity => {
                // Scalar properties: expose the scalar in both X and Y slots
                type ScalarKfs<'a> = (Option<&'a [crate::core::keyframe::Keyframe<f32>]>, &'a dyn Fn(u32) -> f32);
                let (kfs, anim): ScalarKfs = match prop {
                    LoopProp::Rotation => (self.rotation.keyframes(), &|f| self.rotation.evaluate(f)),
                    _ => (self.opacity.keyframes(), &|f| self.opacity.evaluate(f)),
                };
                if let Some(kfs) = kfs {
                    if let (Some(first), Some(last)) = (kfs.first(), kfs.last()) {
                        let first = first.frame;
                        let last = last.frame;
                        let v2 = |f: u32| [anim(f), anim(f)];
                        vals.out_cycle = v2(remap_frame_for_loop(frame, first, last, false))[0];
                        vals.out_pingpong = v2(remap_frame_for_loop(frame, first, last, true))[0];
                        vals.in_cycle = vals.out_cycle;
                        vals.in_pingpong = vals.out_pingpong;
                    }
                }
            }
        }
        vals
    }

    pub fn eval_rotation(&self, frame: u32, fps: u32) -> f32 {
        let eval_frame = if let Some(kfs) = self.rotation.keyframes() {
            match (kfs.first(), kfs.last()) {
                (Some(first), Some(last)) => {
                    Self::remap_loop_frame(frame, &self.rotation_expression, first.frame, last.frame)
                }
                _ => frame,
            }
        } else { frame };
        let base = self.rotation.evaluate(eval_frame);
        match &self.rotation_expression {
            Some(Expression::Raw(script)) if expression_engine::script_uses_loops(script) => {
                let loops = self.compute_loop_vals(frame, fps, LoopProp::Rotation);
                expression_engine::eval_f32_with_loops(script, base, frame, fps, loops)
            }
            Some(expr) => expr.evaluate_f32(base, eval_frame, fps),
            None => base,
        }
    }

    pub fn eval_scale(&self, frame: u32, fps: u32) -> [f32; 2] {
        let eval_frame = if let Some(kfs) = self.scale.keyframes() {
            match (kfs.first(), kfs.last()) {
                (Some(first), Some(last)) => {
                    Self::remap_loop_frame(frame, &self.scale_expression, first.frame, last.frame)
                }
                _ => frame,
            }
        } else { frame };
        let base = self.scale.evaluate(eval_frame);
        match &self.scale_expression {
            Some(Expression::Raw(script)) if expression_engine::script_uses_loops(script) => {
                let loops = self.compute_loop_vals(frame, fps, LoopProp::Scale);
                expression_engine::eval_v2_with_loops(script, base, frame, fps, loops)
            }
            Some(expr) => expr.evaluate_v2(base, eval_frame, fps),
            None => base,
        }
    }

    pub fn eval_opacity(&self, frame: u32, fps: u32) -> f32 {
        let eval_frame = if let Some(kfs) = self.opacity.keyframes() {
            match (kfs.first(), kfs.last()) {
                (Some(first), Some(last)) => {
                    Self::remap_loop_frame(frame, &self.opacity_expression, first.frame, last.frame)
                }
                _ => frame,
            }
        } else { frame };
        let base = self.opacity.evaluate(eval_frame);
        match &self.opacity_expression {
            Some(Expression::Raw(script)) if expression_engine::script_uses_loops(script) => {
                let loops = self.compute_loop_vals(frame, fps, LoopProp::Opacity);
                expression_engine::eval_f32_with_loops(script, base, frame, fps, loops)
            }
            Some(expr) => expr.evaluate_f32(base, eval_frame, fps),
            None => base,
        }
    }
}

// ─── 3D Space Extension (Camera Suite) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform3D {
    pub position: Animatable<[f32; 3]>,
    pub rotation: Animatable<[f32; 3]>,
    pub scale: Animatable<[f32; 3]>,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            position: Animatable::new_constant([0.0, 0.0, 0.0]),
            rotation: Animatable::new_constant([0.0, 0.0, 0.0]),
            scale: Animatable::new_constant([100.0, 100.0, 100.0]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera3D {
    pub name: String,
    pub active: bool,
    pub fov_degrees: f32,
    pub focus_distance: f32,
    pub aperture: f32,
    pub transform: Transform3D,
    /// Depth of Field toggle
    #[serde(default)]
    pub dof_enabled: bool,
    /// Maximum blur radius in pixels for out-of-focus areas (1–64)
    #[serde(default = "default_dof_max_blur")]
    pub dof_max_blur: f32,
    /// Iris shape: 0=circle, 3=triangle, 5=pentagon, 6=hexagon, 8=octagon
    #[serde(default)]
    pub dof_iris_sides: u32,
}

fn default_dof_max_blur() -> f32 { 16.0 }

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            name: "Active Camera".to_string(),
            active: true,
            fov_degrees: 50.0,
            focus_distance: 1000.0,
            aperture: 2.8,
            transform: Transform3D::default(),
            dof_enabled: false,
            dof_max_blur: 16.0,
            dof_iris_sides: 0,
        }
    }
}

/// Project a world-space point through the composition camera to normalized
/// screen coordinates ([0..1], origin top-left). Returns None when the point is
/// behind the camera. Mirrors the simplified camera model (translate + Z-rotate)
/// used by the renderers' `perspective_project_layer`.
pub fn project_point_to_screen(
    cam: &Camera3D,
    point: [f32; 3],
    screen_width: f32,
    screen_height: f32,
) -> Option<[f32; 2]> {
    if screen_width <= 0.0 || screen_height <= 0.0 {
        return None;
    }
    let cam_pos = cam.transform.position.evaluate(0);
    let cam_rot = cam.transform.rotation.evaluate(0);
    let to_rad = |d: f32| d * std::f32::consts::PI / 180.0;
    let cam_zr = to_rad(cam_rot[2]);
    let (ccrz, ssrz) = (cam_zr.cos(), cam_zr.sin());

    let dx = point[0] - cam_pos[0];
    let dy = point[1] - cam_pos[1];
    let dz = point[2] - cam_pos[2];
    let cx = dx * ccrz - dy * ssrz;
    let cy = dx * ssrz + dy * ccrz;
    let cz = dz;
    if cz <= 0.1 {
        return None;
    }

    let fov_rad = cam.fov_degrees.max(1.0) * std::f32::consts::PI / 180.0;
    let focal = (screen_height * 0.5) / (fov_rad * 0.5).tan();
    let sx = screen_width * 0.5 + (cx * focal) / cz;
    let sy = screen_height * 0.5 - (cy * focal) / cz;
    Some([sx / screen_width, sy / screen_height])
}

// ─── 3D Material Options (AE-style material properties) ─────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialOptions {
    /// Ambient light contribution (0.0–1.0). Default 0.3.
    pub ambient: f32,
    /// Diffuse (Lambertian) reflection intensity (0.0–1.0). Default 0.8.
    pub diffuse: f32,
    /// Specular highlight intensity (0.0–1.0). Default 0.5.
    pub specular: f32,
    /// Specular highlight sharpness / exponent (1–256). Higher = tighter highlight.
    pub specular_exponent: f32,
    /// Emissive self-illumination (0.0–1.0). Default 0.0.
    pub emission: f32,
    /// Metalness for PBR-like shading (0.0 = dielectric, 1.0 = metal). Default 0.0.
    pub metalness: f32,
    /// Whether this layer casts shadows from shadow-casting lights (AE material option).
    #[serde(default = "default_true")]
    pub cast_shadows: bool,
}

fn default_true() -> bool { true }
fn default_shadow_darkness() -> f32 { 60.0 }

impl Default for MaterialOptions {
    fn default() -> Self {
        Self {
            ambient: 0.3,
            diffuse: 0.8,
            specular: 0.5,
            specular_exponent: 32.0,
            emission: 0.0,
            metalness: 0.0,
            cast_shadows: true,
        }
    }
}

// ─── Color Space Convert Modes ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorConversionMode {
    #[default]
    LogCToLinear,
    LinearToLogC,
    SLog3ToLinear,
    LinearToSLog3,
}

// ─── Effects ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectType {
    GaussianBlur {
        blur_radius: Animatable<f32>,
    },
    ColorTint {
        color: Animatable<[f32; 4]>,
        intensity: Animatable<f32>,
    },
    DropShadow {
        color: Animatable<[f32; 4]>,
        opacity: Animatable<f32>,
        direction: Animatable<f32>,
        distance: Animatable<f32>,
        softness: Animatable<f32>,
    },
    ChromaticAberration {
        shift_r: Animatable<f32>,
        shift_b: Animatable<f32>,
        edge_falloff: Animatable<f32>,
        /// When true, the aberration shift scales with the layer's DOF
        /// CoC radius and the number of iris blades for physically-plausible
        /// camera-lens color fringing.
        #[serde(default = "default_true")]
        iris_linked: bool,
    },
    Vignette {
        intensity: Animatable<f32>,
        roundness: Animatable<f32>,
        feather: Animatable<f32>,
        color: Animatable<[f32; 4]>,
    },
    Levels {
        input_black: Animatable<f32>,
        input_white: Animatable<f32>,
        gamma: Animatable<f32>,
        output_black: Animatable<f32>,
        output_white: Animatable<f32>,
    },
    HueSaturation {
        hue_shift: Animatable<f32>,
        saturation: Animatable<f32>,
        lightness: Animatable<f32>,
    },
    Glow {
        threshold: Animatable<f32>,
        radius: Animatable<f32>,
        intensity: Animatable<f32>,
        color: Animatable<[f32; 4]>,
    },
    /// GPU-first screen-space optical flare (core + rings + star streaks).
    /// When `link_to_light` names a comp light, the flare source tracks that
    /// light's projected position each frame (overrides position_x/y).
    LensFlare {
        enabled: Animatable<f32>,
        position_x: Animatable<f32>,
        position_y: Animatable<f32>,
        intensity: Animatable<f32>,
        threshold: Animatable<f32>,
        color: Animatable<[f32; 4]>,
        #[serde(default)]
        link_to_light: Option<String>,
    },
    MotionBlur {
        shutter_angle: Animatable<f32>,
        samples: u32,
    },
    MeshWarp {
        top_left: Animatable<[f32; 2]>,
        top_right: Animatable<[f32; 2]>,
        bottom_left: Animatable<[f32; 2]>,
        bottom_right: Animatable<[f32; 2]>,
    },
    /// AE Corner Pin: 8-DOF homography mapping the layer rectangle onto an
    /// arbitrary target quadrilateral (screen inserts, perspective billboards).
    CornerPin {
        top_left: Animatable<[f32; 2]>,
        top_right: Animatable<[f32; 2]>,
        bottom_right: Animatable<[f32; 2]>,
        bottom_left: Animatable<[f32; 2]>,
    },
    ColorGradeLUT {
        lut_path: String,
        intensity: Animatable<f32>,
    },
    ColorSpaceConvert {
        mode: ColorConversionMode,
    },
    FilmGrain {
        intensity: Animatable<f32>,
        grain_size: f32,
        color_film: bool,
    },

    // ── CPU pixel-effect kernels (wired to core::cpu_effects → ae_effects_pack) ──
    // These run on the software/CPU render path so effects are visible even
    // without the GPU pipeline, and reuse the orphaned ae_effects_pack kernels.
    Twirl {
        angle: Animatable<f32>,
        radius: Animatable<f32>,
    },
    Bulge {
        amount: Animatable<f32>,
        radius: Animatable<f32>,
    },
    Posterize {
        levels: Animatable<f32>,
    },
    Invert {
        invert_alpha: bool,
    },
    Offset {
        shift_x: Animatable<f32>,
        shift_y: Animatable<f32>,
    },
    DirectionalBlur {
        angle: Animatable<f32>,
        length: Animatable<f32>,
    },
    RadialBlur {
        amount: Animatable<f32>,
    },
    Sharpen {
        amount: Animatable<f32>,
    },
    Threshold {
        threshold: Animatable<f32>,
    },
    LinearWipe {
        completion: Animatable<f32>,
        angle: Animatable<f32>,
    },
    SimpleChoker {
        choke_amount: Animatable<f32>,
    },
    ChromaKey {
        screen_color: Animatable<[f32; 3]>,
        screen_gain: Animatable<f32>,
        clip_black: Animatable<f32>,
        clip_white: Animatable<f32>,
    },
    Spherize {
        radius: Animatable<f32>,
        refractive_index: Animatable<f32>,
    },
    TurbulentDisplace {
        amount: Animatable<f32>,
        size: Animatable<f32>,
        evolution: Animatable<f32>,
        complexity: Animatable<f32>,
    },
    Colorama {
        preset_index: Animatable<f32>,
        cycle_phase: Animatable<f32>,
    },
    // ── New AE-standard effects ──
    FractalNoise {
        fractal_type: Animatable<f32>,
        contrast: Animatable<f32>,
        brightness: Animatable<f32>,
        complexity: Animatable<f32>,
        evolution: Animatable<f32>,
    },
    Curves {
        channel: Animatable<f32>,
    },
    DisplacementMap {
        source_layer: Animatable<f32>,
        max_horizontal: Animatable<f32>,
        max_vertical: Animatable<f32>,
    },
    CompoundBlur {
        source_layer: Animatable<f32>,
        max_blur: Animatable<f32>,
    },
    Minimax {
        operation: Animatable<f32>,
        radius: Animatable<f32>,
    },
    ShiftChannels {
        take_red: Animatable<f32>,
        take_green: Animatable<f32>,
        take_blue: Animatable<f32>,
        take_alpha: Animatable<f32>,
    },
    // ── Effects migrated from ExtEffect (effect_registry_ext) ──
    WaveWarp {
        wave_height: Animatable<f32>,
        wave_width: Animatable<f32>,
        speed: Animatable<f32>,
        direction_deg: Animatable<f32>,
        wave_type: u8,
        pinning: u8,
    },
    CcLens {
        convergence: Animatable<f32>,
        zoom: Animatable<f32>,
    },
    PolarCoordinates {
        to_polar: bool,
        interpolation: Animatable<f32>,
    },
    OpticsCompensation {
        field_of_view_deg: Animatable<f32>,
        reverse: bool,
        zoom: Animatable<f32>,
    },
    ColorBalance {
        shadows: [f32; 3],
        midtones: [f32; 3],
        highlights: [f32; 3],
        preserve_luminosity: bool,
    },
    ChannelMixer {
        matrix: [[f32; 3]; 3],
        monochrome: bool,
    },
    LightSweep {
        direction_deg: Animatable<f32>,
        center: Animatable<f32>,
        width: Animatable<f32>,
        sweep_intensity: Animatable<f32>,
        edge_intensity: Animatable<f32>,
    },
    RadialFastBlur {
        amount: Animatable<f32>,
        samples: u32,
    },
    BendIt {
        top_offset: Animatable<f32>,
        bottom_offset: Animatable<f32>,
    },
    Tiler {
        scale_percent: Animatable<f32>,
        mirror: bool,
    },
    // ── Effects from ae_effects_pack not yet wired ──
    Tritone {
        shadow_color: Animatable<[f32; 3]>,
        mid_color: Animatable<[f32; 3]>,
        highlight_color: Animatable<[f32; 3]>,
    },
    MatteChoker {
        choke_amount: Animatable<f32>,
        gray_level: Animatable<f32>,
    },
    VenetianBlinds {
        completion: Animatable<f32>,
        width: Animatable<f32>,
    },
    // ── Lumetri Basic Correction (Vibrance / WB / HSL Secondary) ──
    Vibrance {
        amount: Animatable<f32>,
    },
    WhiteBalance {
        temperature: Animatable<f32>,
        tint: Animatable<f32>,
    },
    HslAdjust {
        hue_deg: Animatable<f32>,
        saturation: Animatable<f32>,
        lightness: Animatable<f32>,
    },
    /// Threshold bloom glow (AE Glow): bright pixels bleed outwards.
    GlowPro {
        threshold: Animatable<f32>,
        radius: Animatable<f32>,
        intensity: Animatable<f32>,
    },
    /// CRT scanline TV distortion.
    CrtScanlines {
        line_spacing: Animatable<f32>,
        intensity: Animatable<f32>,
    },
    /// Attenuated spiral vortex around the layer centre.
    Vortex {
        radius: Animatable<f32>,
        angle_deg: Animatable<f32>,
    },
    /// Rising thermal turbulence; time advances with `speed` per second.
    HeatDistortion {
        strength: Animatable<f32>,
        speed: Animatable<f32>,
    },
    /// Water drop / rain ripple ring displacement.
    RainRipples {
        drop_count: Animatable<f32>,
        wave_strength: Animatable<f32>,
    },
    /// Fisheye lens bulge distortion (−strength = pincushion).
    Fisheye {
        strength: Animatable<f32>,
    },
    /// Barrel (+k1) / pincushion (−k1) lens correction with k2 term.
    LensCorrection {
        k1: Animatable<f32>,
        k2: Animatable<f32>,
    },
    /// Deterministic digital block glitch displacement.
    GlitchDisplacement {
        seed: Animatable<f32>,
        amount: Animatable<f32>,
    },
    /// Morphological matte choke (erode) or spread (dilate).
    MatteChokeSpread {
        radius: Animatable<f32>,
        expand: bool,
    },
    /// Soft blur applied to the alpha edge only.
    AlphaFeather {
        radius: Animatable<f32>,
    },
    /// Replace alpha with luminance (optionally inverted).
    AlphaFromLuminance {
        invert: bool,
    },
    /// Phosphor-green night vision look with per-frame film noise.
    NightVision {
        amplification: Animatable<f32>,
    },
    /// Circular iris wipe transition (0 = fully covered).
    IrisWipe {
        completion: Animatable<f32>,
    },
    /// Sweeping radial wipe transition (0 = fully covered).
    RadialWipe {
        completion: Animatable<f32>,
    },
    /// ASC CDL film emulation: lift/gamma/gain grade + YCbCr hue rotation.
    FilmEmulation {
        lift: Animatable<f32>,
        gamma: Animatable<f32>,
        gain: Animatable<f32>,
        hue_shift_deg: Animatable<f32>,
    },
    /// Volumetric light scattering from a sun position (normalized 0..1).
    GodRays {
        sun_x: Animatable<f32>,
        sun_y: Animatable<f32>,
        samples: Animatable<f32>,
        decay: Animatable<f32>,
        weight: Animatable<f32>,
    },
    /// Audio Spectrum: paint a real-time frequency-bar overlay sourced from
    /// the current frame's analyzed audio bands (see
    /// expression_engine::set_audio_data).
    AudioSpectrum {
        enabled: Animatable<f32>,
        bands: Animatable<f32>,
        opacity: Animatable<f32>,
        color_start: [f32; 4],
        color_end: [f32; 4],
        // Normalized position (0..1) of the bar strip's lower-left.
        position_x: Animatable<f32>,
        position_y: Animatable<f32>,
        // Strip width/height as fraction of the layer quad.
        width: Animatable<f32>,
        height: Animatable<f32>,
    },
    /// Centre zoom motion blur.
    RadialBlurZoom {
        amount: Animatable<f32>,
    },
    /// Median filter — salt-and-pepper noise removal.
    MedianFilter {
        radius: Animatable<f32>,
    },
    /// Sobel edge outline.
    SobelEdges {
        invert: bool,
    },
    /// Block pixelation.
    Mosaic {
        block_w: Animatable<f32>,
        block_h: Animatable<f32>,
    },
    /// Physical Optical Lens Flare generator (AE Parity).
    OpticalFlares {
        position: Animatable<[f32; 2]>,
        brightness: Animatable<f32>,
        scale: Animatable<f32>,
    },
    /// Motion Tile & CC RepeTile (AE Parity).
    MotionTile {
        tile_center: Animatable<[f32; 2]>,
        tile_width: Animatable<f32>,
        tile_height: Animatable<f32>,
        output_width: Animatable<f32>,
        output_height: Animatable<f32>,
        mirror_edges: bool,
        phase: Animatable<f32>,
    },
    /// CC Page Turn 3D cylindrical paper curl & peel (AE Parity).
    PageTurn {
        fold_position: Animatable<[f32; 2]>,
        fold_radius: Animatable<f32>,
        fold_direction_deg: Animatable<f32>,
        light_direction_deg: Animatable<f32>,
        back_opacity: Animatable<f32>,
        back_color: Animatable<[f32; 4]>,
    },
    /// Tilt-shift miniature focus band.
    TiltShift {
        focus_y: Animatable<f32>,
        focus_height: Animatable<f32>,
        max_blur: Animatable<f32>,
    },
    /// Surface relief emboss.
    Emboss {
        angle_deg: Animatable<f32>,
        depth: Animatable<f32>,
    },
    /// Parallax star field generator.
    StarField {
        num_stars: Animatable<f32>,
        depth_speed: Animatable<f32>,
    },
    /// Procedural lightning bolt between two normalized points.
    LightningArc {
        start_x: Animatable<f32>,
        start_y: Animatable<f32>,
        end_x: Animatable<f32>,
        end_y: Animatable<f32>,
        seed: Animatable<f32>,
        glow: Animatable<f32>,
    },
    /// Cellular-automaton fire rising from the bottom edge.
    FireAutomaton {
        intensity: Animatable<f32>,
    },
    /// Luminance-range key — pixels outside the luma band become transparent.
    LumaKeyRange {
        low_threshold: Animatable<f32>,
        high_threshold: Animatable<f32>,
        invert: bool,
    },
    /// Halftone dot screen rasterization.
    Halftone {
        cell_size: Animatable<f32>,
    },
    /// Solar inversion of bright values above a threshold.
    Solarize {
        threshold: Animatable<f32>,
    },
    /// Column-wise pixel sorting glitch.
    PixelSort {
        threshold: Animatable<f32>,
    },
    /// Pinch (amount > 0) / punch (amount < 0) polar distortion around centre.
    PinchPunch {
        radius: Animatable<f32>,
        amount: Animatable<f32>,
    },
    /// Horizontal scanline jitter glitch.
    ScanlineGlitch {
        jitter_amount: Animatable<f32>,
        seed: Animatable<f32>,
    },
    /// Glass edge bevel with specular refraction on layer borders.
    GlassEdgeBevel {
        bevel_size: Animatable<f32>,
        refraction: Animatable<f32>,
    },
    /// Sharpen along an arbitrary direction.
    DirectionalSharpen {
        angle_deg: Animatable<f32>,
        strength: Animatable<f32>,
    },
    /// Spherical glass-ball refraction lens.
    RefractionLens {
        radius: Animatable<f32>,
        ior: Animatable<f32>,
    },
    /// 3-colour gradient map (shadow / mid / high ramp).
    GradientMap {
        low_color: Animatable<[f32; 3]>,
        mid_color: Animatable<[f32; 3]>,
        high_color: Animatable<[f32; 3]>,
    },
    /// Cinematic warm light leak from a normalized position.
    LightLeak {
        pos_x: Animatable<f32>,
        pos_y: Animatable<f32>,
        intensity: Animatable<f32>,
    },
    /// Alpha bevel with directional lighting.
    BevelAlpha {
        depth: Animatable<f32>,
        light_angle_deg: Animatable<f32>,
    },
    /// Ink cross-hatch sketch stylize.
    CrossHatch {
        line_gap: Animatable<f32>,
        threshold: Animatable<f32>,
    },
    /// CMYK four-plate halftone rasterization.
    CmykHalftone {
        dot_size: Animatable<f32>,
    },
    /// Mirror reflection below a horizon line with distance fade.
    ReflectionMap {
        reflect_y: Animatable<f32>,
        fade_dist: Animatable<f32>,
        opacity: Animatable<f32>,
    },
    /// Perlin flow vector noise field (animated).
    PerlinFlow {
        scale: Animatable<f32>,
    },
    /// Fractal Brownian motion turbulence.
    FbmTurbulence {
        octaves: Animatable<f32>,
        amplitude: Animatable<f32>,
    },
    // ── Expression Controls (non-rendering utility effects, AE parity) ──
    SliderControl {
        value: Animatable<f32>,
    },
    AngleControl {
        angle_degrees: Animatable<f32>,
    },
    PointControl {
        point: Animatable<[f32; 2]>,
    },
    ColorControl {
        color: Animatable<[f32; 4]>,
    },
    /// AE Checkbox Control: boolean toggle for expressions
    CheckboxControl {
        checked: bool,
    },
    /// AE Dropdown/Menu Control: integer selector for expressions
    DropdownControl {
        value: i32,
        options: Vec<String>,
    },
    /// AE 3D Point Control: XYZ position for expressions
    Point3DControl {
        point: Animatable<[f32; 3]>,
    },
    /// Cinematic letterbox bars (fraction of frame height, total both sides).
    Letterbox {
        frac: Animatable<f32>,
    },
    /// Custom WGSL shader effect (runtime compiled).
    CustomShader {
        /// WGSL source code for the fragment shader.
        wgsl_source: String,
        /// Uniform float values passed to the shader (up to 16).
        uniform_values: Vec<f32>,
    },
    /// Merge Paths boolean operation on shape sub-paths.
    MergePaths {
        /// Operation: 0=Add, 1=Subtract, 2=Intersect, 3=Exclude.
        operation: Animatable<f32>,
    },
    /// Offset Path: expand or contract a shape path along its normals.
    OffsetPath {
        /// Offset amount in pixels (positive = expand, negative = contract).
        amount: Animatable<f32>,
        /// Line join: 0=Miter, 1=Round, 2=Bevel.
        line_join: Animatable<f32>,
        /// Miter limit for miter joins.
        miter_limit: Animatable<f32>,
    },
    /// Two-band bass/treble equalizer.
    BassTreble {
        /// Bass gain in dB (-24..+24).
        bass_gain: Animatable<f32>,
        /// Treble gain in dB (-24..+24).
        treble_gain: Animatable<f32>,
        /// Crossover frequency in Hz (80..1200).
        crossover_freq: Animatable<f32>,
    },
    /// Flanger audio effect (modulated delay).
    Flanger {
        /// Maximum delay in ms (1..10).
        max_delay_ms: Animatable<f32>,
        /// LFO rate in Hz (0.1..10).
        lfo_rate: Animatable<f32>,
        /// Feedback amount (0.0..0.95).
        feedback: Animatable<f32>,
        /// Wet/dry mix (0..1).
        wet_dry: Animatable<f32>,
    },
    /// Chorus audio effect (multiple detuned delays).
    Chorus {
        /// Delay time in ms (1..30).
        delay_ms: Animatable<f32>,
        /// Modulation depth in ms (0.5..10).
        depth_ms: Animatable<f32>,
        /// LFO rate in Hz (0.1..6).
        rate_hz: Animatable<f32>,
        /// Number of voices (2..8).
        voices: Animatable<f32>,
        /// Feedback (0.0..0.9).
        feedback: Animatable<f32>,
    },
    /// Parametric equalizer (2-band bell filter).
    ParametricEQ {
        /// Center frequency in Hz (60..18000).
        freq_hz: Animatable<f32>,
        /// Gain in dB (-24..+24).
        gain_db: Animatable<f32>,
        /// Q factor (0.5..12).
        q_factor: Animatable<f32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub id: String,
    pub name: String,
    pub effect_type: EffectType,
    pub enabled: bool,
}

// ─── Layer Style ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DropShadowStyle {
    pub enabled: bool,
    pub blend_mode: BlendMode,
    pub opacity: f32,
    pub angle: f32,
    pub distance: f32,
    pub size: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OuterGlowStyle {
    pub enabled: bool,
    pub opacity: f32,
    pub spread: f32,
    pub size: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrokeStyle {
    pub enabled: bool,
    pub size: f32,
    pub position: u32,
    pub color: [f32; 4],
}

/// Gradient Overlay layer style: linear gradient blend across the layer
/// (angle in degrees, scale = gradient extent % of layer diagonal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientOverlayStyle {
    pub enabled: bool,
    pub opacity: f32,
    /// Gradient direction in degrees (0 = left→right, 90 = bottom→top)
    pub angle: f32,
    /// Gradient length as % of layer diagonal
    pub scale: f32,
    /// Gradient start / end colors
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
}

impl Default for GradientOverlayStyle {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity: 100.0,
            angle: 90.0,
            scale: 100.0,
            color_start: [1.0, 1.0, 1.0, 1.0],
            color_end: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Solid Color Overlay layer style.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorOverlayStyle {
    pub enabled: bool,
    pub opacity: f32,
    pub color: [f32; 4],
}

/// Inner Shadow layer style (compass angle matching DropShadowStyle).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InnerShadowStyle {
    pub enabled: bool,
    pub opacity: f32,
    pub angle: f32,
    pub distance: f32,
    pub size: f32,
    pub color: [f32; 4],
}

/// Inner Glow layer style.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InnerGlowStyle {
    pub enabled: bool,
    pub opacity: f32,
    pub size: f32,
    pub color: [f32; 4],
}

/// Satin layer style: soft interior sheen band along the shape's contour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatinStyle {
    pub enabled: bool,
    pub opacity: f32,
    /// Sheen direction (compass degrees, matching DropShadowStyle)
    pub angle: f32,
    /// Offset of the sheen source from the edge
    pub distance: f32,
    /// Softness of the band
    pub size: f32,
    pub color: [f32; 4],
}

impl Default for SatinStyle {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity: 50.0,
            angle: 90.0,
            distance: 12.0,
            size: 16.0,
            color: [0.2, 0.2, 0.35, 1.0],
        }
    }
}

/// Bevel/Emboss layer style: directional highlight + shadow along alpha edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BevelEmbossStyle {
    pub enabled: bool,
    /// Light direction (compass degrees matching DropShadowStyle)
    pub angle: f32,
    /// Edge detection offset (px)
    pub depth: f32,
    /// Softness of the bevel band (px)
    pub size: f32,
    /// Highlight strength (0..100)
    pub highlight: f32,
    /// Shadow strength (0..100)
    pub shadow: f32,
    /// Highlight color
    pub color_light: [f32; 4],
    /// Shadow color
    pub color_dark: [f32; 4],
}

impl Default for BevelEmbossStyle {
    fn default() -> Self {
        Self {
            enabled: false,
            angle: 135.0,
            depth: 3.0,
            size: 3.0,
            highlight: 70.0,
            shadow: 50.0,
            color_light: [1.0, 1.0, 1.0, 1.0],
            color_dark: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerStyle {
    pub drop_shadow: DropShadowStyle,
    pub outer_glow: OuterGlowStyle,
    pub stroke: StrokeStyle,
    #[serde(default)]
    pub gradient_overlay: GradientOverlayStyle,
    #[serde(default)]
    pub color_overlay: ColorOverlayStyle,
    #[serde(default)]
    pub inner_shadow: InnerShadowStyle,
    #[serde(default)]
    pub inner_glow: InnerGlowStyle,
    #[serde(default)]
    pub satin: SatinStyle,
    #[serde(default)]
    pub bevel_emboss: BevelEmbossStyle,
}

// ─── Text Formatting ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFormatting {
    pub font_family: String,
    pub tracking: f32,
    pub leading: f32,
    pub stroke_color: Option<[f32; 4]>,
    pub stroke_width: f32,
    /// Paragraph alignment: 0=Left, 1=Center, 2=Right, 3=Justify
    #[serde(default)]
    pub alignment: u32,
    /// Text box width for line wrapping (0 = no wrapping)
    #[serde(default)]
    pub box_width: f32,
    /// Text box height for vertical overflow (0 = no limit)
    #[serde(default)]
    pub box_height: f32,
}

impl Default for TextFormatting {
    fn default() -> Self {
        Self {
            font_family: "Inter".to_string(),
            tracking: 0.0,
            leading: 1.2,
            stroke_color: None,
            stroke_width: 1.0,
            alignment: 0,
            box_width: 0.0,
            box_height: 0.0,
        }
    }
}

// ─── Layer ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub layer_type: LayerType,
    pub in_frame: u32,
    pub out_frame: u32,
    pub transform: Transform2D,
    pub effects: Vec<Effect>,
    pub visible: bool,
    pub locked: bool,

    pub parent_id: Option<String>,
    pub track_matte: TrackMatteMode,
    pub solo: bool,
    pub motion_blur: bool,
    pub label: LabelColor,
    pub time_remap: Option<Animatable<f32>>,

    pub trackers: Vec<TrackerPoint>,

    pub is_3d: bool,
    pub transform_3d: Transform3D,
    
    #[serde(default)]
    pub material: MaterialOptions,

    // ── AE Blend Mode ──
    pub blend_mode: BlendMode,

    pub is_adjustment_layer: bool,
    pub is_guide_layer: bool,
    pub is_shy: bool,
    pub effects_enabled: bool,
    pub is_collapsed: bool,

    // ── AE Masking System ──
    pub masks: Vec<crate::core::mask::Mask>,
    #[serde(default)]
    pub puppet_pins: Vec<PuppetPin>,
    #[serde(default)]
    pub paint_strokes: Vec<PaintStroke>,

    // ── AE Layer Markers (per-layer comment flags) ──
    #[serde(default)]
    pub markers: Vec<TimelineMarker>,

    // ── AE Auto-Orient (rotation follows motion path) ──
    #[serde(default)]
    pub auto_orient: crate::core::auto_orient::AutoOrientMode,

    // ── Posterize Time (stop-motion frame quantization) ──
    #[serde(default)]
    pub posterize_time: Option<crate::core::posterize_time::PosterizeTimeSettings>,

    // ── AE Layer Style System ──
    pub style: LayerStyle,

    // ── Text Formatting System ──
    pub text_formatting: Option<TextFormatting>,

    // ── Text Animator (per-character animation) ──
    #[serde(default)]
    pub text_animator: Option<crate::core::text_animator::TextAnimatorSettings>,
    pub text_animator_stack: Option<AnimatorStack>,

    // ── AE Transparency System ──
    #[serde(default)]
    pub preserve_transparency: bool,

    // ── Trim Paths Animator (Shape path trimming) ──
    #[serde(default)]
    pub trim_paths: Option<TrimPaths>,

    // ── Shape Repeater (shape duplication) ──
    #[serde(default)]
    pub shape_repeater: Option<crate::core::shape_repeater::ShapeRepeaterOptions>,

    // ── Layer Constraints System (Pinning) ──
    #[serde(default)]
    pub constraints: crate::core::layer_constraints::LayerConstraints,

    // ── Essential Properties (Master Properties) ──
    #[serde(default)]
    pub essential_properties: Vec<crate::core::essential_properties::EssentialProperty>,

    // ── Proxy (low-res preview) ──
    #[serde(default)]
    pub proxy: crate::core::proxy::LayerProxy,
}

/// A brush stroke painted onto a layer. Points live in layer-local space
/// (origin = the layer's rest center), so strokes follow transforms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintStroke {
    pub color: [f32; 4],
    pub size: f32,
    pub points: Vec<[f32; 2]>,
    /// First frame the stroke is visible.
    #[serde(default)]
    pub start_frame: u32,
    /// Last frame it is visible; 0 = until the layer's out-point.
    #[serde(default)]
    pub end_frame: u32,
}

/// A puppet-tool deformation pin. `comp_source` is the rest position in
/// composition space where the pin was placed; `position` (animatable) is
/// its current comp-space location — the delta drives the mesh warp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PuppetPin {
    pub id: String,
    pub name: String,
    pub comp_source: [f32; 2],
    pub position: Animatable<[f32; 2]>,
}

impl PuppetPin {
    pub fn new(id: String, name: String, source: [f32; 2]) -> Self {
        Self { id, name, comp_source: source, position: Animatable::new_constant(source) }
    }
}



impl Layer {
    pub fn new(id: String, name: String, layer_type: LayerType, duration_frames: u32) -> Self {
        let is_adj = matches!(layer_type, LayerType::AdjustmentLayer);
        Self {
            id,
            name,
            layer_type,
            in_frame: 0,
            out_frame: duration_frames,
            transform: Transform2D::default(),
            effects: Vec::new(),
            visible: true,
            locked: false,
            parent_id: None,
            track_matte: TrackMatteMode::None,
            solo: false,
            motion_blur: false,
            label: LabelColor::Red,
            time_remap: None,
            trackers: Vec::new(),
            is_3d: false,
            transform_3d: Transform3D::default(),
            material: MaterialOptions::default(),
            blend_mode: BlendMode::Normal,
            is_adjustment_layer: is_adj,
            is_guide_layer: false,
            is_shy: false,
            effects_enabled: true,
            is_collapsed: false,
            masks: Vec::new(),
            puppet_pins: Vec::new(),
            paint_strokes: Vec::new(),
            markers: Vec::new(),
            auto_orient: crate::core::auto_orient::AutoOrientMode::Off,
            posterize_time: None,
            style: LayerStyle::default(),
            text_formatting: None,
            text_animator: None,
            text_animator_stack: None,
            preserve_transparency: false,
            trim_paths: None,
            shape_repeater: None,
            constraints: crate::core::layer_constraints::LayerConstraints::default(),
            essential_properties: vec![],
            proxy: crate::core::proxy::LayerProxy::default(),
        }
    }

    pub fn new_null(id: String, name: String, duration_frames: u32) -> Self {
        let mut l = Self::new(id, name, LayerType::Null, duration_frames);
        l.label = LabelColor::Red;
        l
    }


    pub fn new_adjustment(id: String, name: String, duration_frames: u32) -> Self {
        let mut l = Self::new(id, name, LayerType::AdjustmentLayer, duration_frames);
        l.label = LabelColor::Lavender;
        l
    }


    /// Computes the accurate, unscaled bounding box dimensions (width, height) of the layer based on its LayerType.
    pub fn bounding_size(&self) -> [f32; 2] {
        match &self.layer_type {
            LayerType::Solid { color: _ } => [1920.0, 1080.0],
            LayerType::Text { text, font_size, tracking, .. } => {
                let char_count = text.chars().count().max(1) as f32;
                let fs = *font_size as f32;
                let approx_w = char_count * fs * 0.6 + (char_count - 1.0) * tracking;
                let approx_h = fs * 1.2;
                [approx_w.max(10.0), approx_h.max(10.0)]
            }
            _ => [100.0, 100.0],
        }
    }




    pub fn is_active(&self, frame: u32) -> bool {
        self.visible && frame >= self.in_frame && frame <= self.out_frame
    }

    pub fn duration_frames(&self) -> u32 {
        self.out_frame.saturating_sub(self.in_frame)
    }

    pub fn remap_frame(&self, frame: u32) -> u32 {
        match &self.time_remap {
            Some(anim) => anim.evaluate(frame) as u32,
            None => frame,
        }
    }

    /// AE "Enable Time Remapping": identity map over the layer span, giving the
    /// user editable keyframes. No-op if remapping is already enabled.
    pub fn enable_time_remapping(&mut self) {
        if self.time_remap.is_some() {
            return;
        }
        let in_f = self.in_frame;
        let out_f = self.out_frame;
        let linear = crate::core::keyframe::InterpolationType::Linear;
        self.time_remap = Some(Animatable::Animated(vec![
            crate::core::keyframe::Keyframe::new(in_f, in_f as f32, linear),
            crate::core::keyframe::Keyframe::new(out_f, out_f as f32, linear),
        ]));
    }

    /// AE "Time Stretch": scale the layer span by `factor` and keep source
    /// playback consistent via a linear remap (source advances at `factor`).
    /// factor > 1 slows the source down; factor < 1 speeds it up.
    pub fn time_stretch(&mut self, factor: f32) {
        if factor <= 0.001 {
            return;
        }
        let in_f = self.in_frame;
        let duration = self.duration_frames() as f32;
        let new_duration = ((duration * factor).round() as u32).max(1);
        self.out_frame = in_f.saturating_add(new_duration);

        let end_source = in_f as f32 + new_duration as f32 / factor;
        let linear = crate::core::keyframe::InterpolationType::Linear;
        self.time_remap = Some(Animatable::Animated(vec![
            crate::core::keyframe::Keyframe::new(in_f, in_f as f32, linear),
            crate::core::keyframe::Keyframe::new(
                in_f.saturating_add(new_duration),
                end_source,
                linear,
            ),
        ]));
    }

    /// AE "Time-Reverse Layer": play the layer's source backwards.
    pub fn time_reverse(&mut self) {
        let in_f = self.in_frame;
        let out_f = self.out_frame;
        let linear = crate::core::keyframe::InterpolationType::Linear;
        self.time_remap = Some(Animatable::Animated(vec![
            crate::core::keyframe::Keyframe::new(in_f, out_f as f32, linear),
            crate::core::keyframe::Keyframe::new(out_f, in_f as f32, linear),
        ]));
    }

    /// AE "Freeze Frame": hold one source frame across the whole span.
    pub fn freeze_at(&mut self, source_frame: u32) {
        self.time_remap = Some(Animatable::new_constant(source_frame as f32));
    }

    /// Remove time remapping so the source plays at comp time again.
    pub fn clear_time_remap(&mut self) {
        self.time_remap = None;
    }

    /// AE "Easy Ease" (F9): apply smooth bezier interpolation to every
    /// transform keyframe. Properties driven by expressions are untouched.
    pub fn easy_ease_transform(&mut self) {
        let t = &mut self.transform;
        if t.position_expression.is_none() {
            t.position.easy_ease();
        }
        if t.scale_expression.is_none() {
            t.scale.easy_ease();
        }
        if t.rotation_expression.is_none() {
            t.rotation.easy_ease();
        }
        if t.opacity_expression.is_none() {
            t.opacity.easy_ease();
        }
        if t.anchor_point_expression.is_none() {
            t.anchor_point.easy_ease();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMarker {
    pub frame: u32,
    pub label: String,
    pub color: [f32; 3],
}

// ─── 3D Light System ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LightType {
    Ambient,
    Point,
    Spot { cone_angle_deg: f32, cone_feather_pct: f32 },
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Light3D {
    pub id: String,
    pub name: String,
    pub light_type: LightType,
    pub color: [f32; 4],
    pub intensity: f32,
    pub position: Animatable<[f32; 3]>,
    pub casts_shadows: bool,
    /// Shadow opacity cast by this light (0–100, AE Shadow Darkness). Default 60.
    #[serde(default = "default_shadow_darkness")]
    pub shadow_darkness: f32,
    /// Distance falloff exponent (0 = none, 1 = linear, 2 = inverse-square). Default 1.
    #[serde(default)]
    pub falloff: f32,
    /// Maximum influence radius (0 = unlimited). Default 0.
    #[serde(default)]
    pub max_radius: f32,
}

impl Default for Light3D {
    fn default() -> Self {
        Self {
            id: "light_main".to_string(),
            name: "Key Light".to_string(),
            light_type: LightType::Point,
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 100.0,
            position: Animatable::new_constant([960.0, 540.0, -500.0]),
            casts_shadows: true,
            shadow_darkness: 60.0,
            falloff: 1.0,
            max_radius: 0.0,
        }
    }
}

// ─── Composition ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composition {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_frames: u32,
    pub layers: Vec<Layer>,

    pub motion_blur_shutter_angle: f32,
    pub background_color: [f32; 4],
    pub active_camera: Camera3D,
    /// Additional scene cameras for multi-camera switching. The camera with
    /// `active == true` wins; empty list falls back to `active_camera`.
    #[serde(default)]
    pub cameras: Vec<Camera3D>,
    pub lights: Vec<Light3D>,
    pub markers: Vec<TimelineMarker>,

    /// Sub-compositions for PreComp nesting (keyed by comp id).
    #[serde(default)]
    pub sub_compositions: Vec<Composition>,
    /// Blend layers in linear light (AE "Blend Colors Using 1.0 Gamma").
    /// Off = classic gamma-space compositing (legacy look).
    #[serde(default)]
    pub blend_linear: bool,
    /// Triangular-PDF dither on final 8-bit output (kills gradient banding).
    /// Off for legacy projects/tests to preserve byte-exact renders.
    #[serde(default)]
    pub dither_output: bool,
    /// Composition-level proxy settings for preview speed.
    #[serde(default)]
    pub comp_proxy: crate::core::proxy::CompProxy,
}

impl Composition {
    pub fn new(
        id: String,
        name: String,
        width: u32,
        height: u32,
        fps: u32,
        duration_frames: u32,
    ) -> Self {
        Self {
            id,
            name,
            width,
            height,
            fps,
            duration_frames,
            layers: Vec::new(),
            motion_blur_shutter_angle: 180.0,
            background_color: [0.05, 0.05, 0.08, 1.0],
            active_camera: Camera3D::default(),
            cameras: Vec::new(),
            lights: vec![Light3D::default()],
            markers: Vec::new(),
            sub_compositions: Vec::new(),

            blend_linear: false,

dither_output: false,
            comp_proxy: crate::core::proxy::CompProxy::default(),
        }
    }

    /// Look up a sub-composition by id (recursive search).
    /// The camera currently driving the render: first entry of `cameras`
    /// with `active == true`, else the legacy `active_camera` field.
    pub fn resolve_camera(&self) -> &Camera3D {
        self.cameras
            .iter()
            .find(|c| c.active)
            .unwrap_or(&self.active_camera)
    }

    pub fn resolve_camera_mut(&mut self) -> &mut Camera3D {
        if let Some(i) = self.cameras.iter().position(|c| c.active) {
            return &mut self.cameras[i];
        }
        &mut self.active_camera
    }

    /// Activate the camera at `idx` (deactivating others). Passing an index
    /// into `cameras`; out-of-range clears all flags (legacy camera resumes).
    pub fn set_active_camera(&mut self, idx: Option<usize>) {
        for (i, c) in self.cameras.iter_mut().enumerate() {
            c.active = Some(i) == idx;
        }
    }

    pub fn find_sub_comp(&self, comp_id: &str) -> Option<&Composition> {
        Self::find_sub_comp_limited(self, comp_id, 0)
    }

    /// Depth-limited recursive lookup: guards against cyclic sub_compositions graphs.
    fn find_sub_comp_limited<'a>(comp: &'a Composition, comp_id: &str, depth: u32) -> Option<&'a Composition> {
        const MAX_SUB_COMP_DEPTH: u32 = 32;
        if depth > MAX_SUB_COMP_DEPTH {
            return None;
        }
        for sub in &comp.sub_compositions {
            if sub.id == comp_id {
                return Some(sub);
            }
            if let Some(found) = Self::find_sub_comp_limited(sub, comp_id, depth + 1) {
                return Some(found);
            }
        }
        None
    }

    /// Resizes the composition to new dimensions and automatically remaps layer positions based on Constraints.
    pub fn resize_and_remap(&mut self, new_w: u32, new_h: u32, old_w: u32, old_h: u32) {
        self.width = new_w;
        self.height = new_h;
        for layer in &mut self.layers {
            let current_p = layer.transform.position.evaluate(0);
            let remapped = layer.constraints.remap_position(
                current_p,
                old_w as f32,
                old_h as f32,
                new_w as f32,
                new_h as f32,
            );
            layer.transform.position = Animatable::new_constant(remapped);
        }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    pub fn resolve_world_transform(
        &self,
        layer: &Layer,
        frame: u32,
    ) -> ([f32; 2], [f32; 2], f32, f32) {
        self.resolve_world_transform_limited(layer, frame, 0)
    }

    /// Depth-limited transform resolution: guards against cyclic parent_id chains
    /// (e.g. from hand-edited or corrupted project files) that would recurse forever.
    fn resolve_world_transform_limited(
        &self,
        layer: &Layer,
        frame: u32,
        depth: u32,
    ) -> ([f32; 2], [f32; 2], f32, f32) {
        const MAX_PARENT_DEPTH: u32 = 32;
        if depth > MAX_PARENT_DEPTH {
            // Cycle detected — fall back to the layer's own local transform
            let fps = self.fps;
            return (
                layer.transform.eval_position(frame, fps),
                layer.transform.eval_scale(frame, fps),
                layer.transform.eval_rotation(frame, fps),
                layer.transform.eval_opacity(frame, fps),
            );
        }
        let fps = self.fps;

        // Layer expressions with composition context (thisComp.layer(...) / thisLayer)
        let has_exprs = layer.transform.position_expression.is_some()
            || layer.transform.rotation_expression.is_some()
            || layer.transform.scale_expression.is_some()
            || layer.transform.opacity_expression.is_some();
        let (pos, scale, rot, opa) = if has_exprs {
            let comp_snap = expression_engine::build_comp_snapshot(self, frame);
            let this_snap = comp_snap.layers.get(&layer.name).cloned();
            fn raw_script(e: &Option<Expression>) -> Option<&String> {
                match e {
                    Some(Expression::Raw(s)) => Some(s),
                    _ => None,
                }
            }
            let base_pos = layer.transform.eval_position(frame, fps);
            let pos = raw_script(&layer.transform.position_expression)
                .map(|s| expression_engine::eval_v2_with_comp(s, base_pos, frame, fps, &comp_snap, this_snap.as_ref()))
                .unwrap_or(base_pos);
            let base_scale = layer.transform.eval_scale(frame, fps);
            let scale = raw_script(&layer.transform.scale_expression)
                .map(|s| expression_engine::eval_v2_with_comp(s, base_scale, frame, fps, &comp_snap, this_snap.as_ref()))
                .unwrap_or(base_scale);
            let base_rot = layer.transform.eval_rotation(frame, fps);
            let rot = raw_script(&layer.transform.rotation_expression)
                .map(|s| expression_engine::eval_f32_with_comp(s, base_rot, frame, fps, &comp_snap, this_snap.as_ref()))
                .unwrap_or(base_rot);
            let base_opa = layer.transform.eval_opacity(frame, fps);
            let opa = raw_script(&layer.transform.opacity_expression)
                .map(|s| expression_engine::eval_f32_with_comp(s, base_opa, frame, fps, &comp_snap, this_snap.as_ref()))
                .unwrap_or(base_opa);
            (pos, scale, rot, opa)
        } else {
            (
                layer.transform.eval_position(frame, fps),
                layer.transform.eval_scale(frame, fps),
                layer.transform.eval_rotation(frame, fps),
                layer.transform.eval_opacity(frame, fps),
            )
        };

        // ── Auto-Orient: override rotation from motion path / target point ──
        let rot = match layer.auto_orient {
            crate::core::auto_orient::AutoOrientMode::Off => rot,
            mode => crate::core::auto_orient::evaluate_auto_orient_rotation(layer, frame, mode)
                .unwrap_or(rot),
        };

        if let Some(pid) = &layer.parent_id {
            if let Some(parent) = self.layers.iter().find(|l| &l.id == pid) {
                let (ppos, pscale, prot, popa) = self.resolve_world_transform_limited(parent, frame, depth + 1);
                let rot_rad = prot.to_radians();
                let (s, c) = rot_rad.sin_cos();
                let world_x = pos[0] * pscale[0] / 100.0 * c - pos[1] * pscale[1] / 100.0 * s + ppos[0];
                let world_y = pos[0] * pscale[0] / 100.0 * s + pos[1] * pscale[1] / 100.0 * c + ppos[1];
                return (
                    [world_x, world_y],
                    [scale[0] * pscale[0] / 100.0, scale[1] * pscale[1] / 100.0],
                    rot + prot,
                    opa * popa / 100.0,
                );
            }
        }
        (pos, scale, rot, opa)
    }

    /// Set parent layer safely, returning false if a cycle would be introduced.
    pub fn set_layer_parent(&mut self, layer_id: &str, parent_id: Option<String>) -> bool {
        if let Some(ref pid) = parent_id {
            if pid == layer_id {
                return false;
            }
            let mut curr = pid.clone();
            let mut visited = std::collections::HashSet::new();
            visited.insert(layer_id.to_string());
            while let Some(parent_layer) = self.layers.iter().find(|l| l.id == curr) {
                if visited.contains(&parent_layer.id) {
                    return false; // Cycle detected!
                }
                visited.insert(parent_layer.id.clone());
                if let Some(ref next_pid) = parent_layer.parent_id {
                    curr = next_pid.clone();
                } else {
                    break;
                }
            }
        }

        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == layer_id) {
            layer.parent_id = parent_id;
            true
        } else {
            false
        }
    }

    /// Validate and sanitize parent-child links across all layers on deserialization or project load.
    /// Breaks any invalid circular parent dependencies to guarantee panic-free rendering.
    pub fn sanitize_parent_cycles(&mut self) -> usize {
        let id_map = self.build_layer_id_index_map();
        let mut visited = std::collections::HashSet::new();
        let mut to_clear = Vec::new();

        for i in 0..self.layers.len() {
            let layer_id = self.layers[i].id.clone();
            if let Some(ref pid) = self.layers[i].parent_id {
                visited.clear();
                visited.insert(layer_id.clone());

                let mut curr = pid.clone();

                while let Some(&idx) = id_map.get(curr.as_str()) {
                    let parent_layer = &self.layers[idx];
                    if !visited.insert(parent_layer.id.clone()) {
                        to_clear.push(i);
                        break;
                    }
                    if let Some(ref next_pid) = parent_layer.parent_id {
                        curr = next_pid.clone();
                    } else {
                        break;
                    }
                }
            }
        }

        let broken_count = to_clear.len();
        for idx in to_clear {
            log::warn!("Broken circular parent reference detected on layer idx {}. Clearing parent_id.", idx);
            self.layers[idx].parent_id = None;
        }
        broken_count
    }

    /// O(1) Layer ID to index lookup map generator.
    pub fn build_layer_id_index_map(&self) -> std::collections::HashMap<&str, usize> {
        let mut map = std::collections::HashMap::with_capacity(self.layers.len());
        for (idx, layer) in self.layers.iter().enumerate() {
            map.insert(layer.id.as_str(), idx);
        }
        map
    }

    /// Calculate and cache all layer world transforms in O(N) linear topological order with cycle protection and O(1) layer lookups.
    pub fn resolve_all_world_transforms_cached(
        &self,
        frame: u32,
    ) -> std::collections::HashMap<String, ([f32; 2], [f32; 2], f32, f32)> {
        let mut cache = std::collections::HashMap::with_capacity(self.layers.len());
        let mut visited = std::collections::HashSet::new();
        let id_map = self.build_layer_id_index_map();

        for layer in &self.layers {
            if !cache.contains_key(&layer.id) {
                visited.clear();
                self.resolve_layer_transform_recursive(layer, frame, &mut cache, &mut visited, &id_map);
            }
        }
        cache
    }

    fn resolve_layer_transform_recursive(
        &self,
        layer: &Layer,
        frame: u32,
        cache: &mut std::collections::HashMap<String, BoundingBox>,
        visited: &mut std::collections::HashSet<String>,
        id_map: &std::collections::HashMap<&str, usize>,
    ) -> BoundingBox {
        if let Some(cached) = cache.get(&layer.id) {
            return *cached;
        }

        let fps = self.fps;
        let pos = layer.transform.eval_position(frame, fps);
        let scale = layer.transform.eval_scale(frame, fps);
        let rot = layer.transform.eval_rotation(frame, fps);
        let opa = layer.transform.eval_opacity(frame, fps);

        if !visited.insert(layer.id.clone()) {
            // Cycle circuit breaker!
            return (pos, scale, rot, opa);
        }

        let res = if let Some(pid) = &layer.parent_id {
            if let Some(&parent_idx) = id_map.get(pid.as_str()) {
                let parent = &self.layers[parent_idx];
                let (ppos, pscale, prot, popa) = self.resolve_layer_transform_recursive(parent, frame, cache, visited, id_map);
                let rot_rad = prot.to_radians();
                let (s, c) = (rot_rad.sin(), rot_rad.cos());
                let world_x = pos[0] * pscale[0] / 100.0 * c - pos[1] * pscale[1] / 100.0 * s + ppos[0];
                let world_y = pos[0] * pscale[0] / 100.0 * s + pos[1] * pscale[1] / 100.0 * c + ppos[1];
                (
                    [world_x, world_y],
                    [scale[0] * pscale[0] / 100.0, scale[1] * pscale[1] / 100.0],
                    rot + prot,
                    opa * popa / 100.0,
                )
            } else {
                (pos, scale, rot, opa)
            }
        } else {
            (pos, scale, rot, opa)
        };

        visited.remove(&layer.id);
        cache.insert(layer.id.clone(), res);
        res
    }

    pub fn resolve_world_transform_3d(
        &self,
        layer: &Layer,
        frame: u32,
    ) -> [[f32; 4]; 4] {
        let pos3d = layer.transform_3d.position.evaluate(frame);
        let rot3d = layer.transform_3d.rotation.evaluate(frame);
        let scale3d = layer.transform_3d.scale.evaluate(frame);

        // Perspective Projection Matrix for Camera
        let fov_rad = self.active_camera.fov_degrees.to_radians();
        let aspect = self.width as f32 / self.height.max(1) as f32;
        // Guard against degenerate FOV (e.g. fov_degrees=0 → tan(0)=0 → division by zero)
        let f = if fov_rad.abs() < 1e-4 { 1e6 } else { 1.0 / (fov_rad * 0.5).tan() };
        let near = 0.1f32;
        let far = 10000.0f32;


        let proj_3d = [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, -f, 0.0, 0.0],
            [0.0, 0.0, far / (far - near), 1.0],
            [0.0, 0.0, -(far * near) / (far - near), 0.0],
        ];

        let rad_x = rot3d[0].to_radians();
        let rad_y = rot3d[1].to_radians();
        let rad_z = rot3d[2].to_radians();

        let cx = rad_x.cos(); let sx = rad_x.sin();
        let cy = rad_y.cos(); let sy = rad_y.sin();
        let cz = rad_z.cos(); let sz = rad_z.sin();

        // Euler rotation matrix Z * Y * X
        let rot_matrix = [
            [cy * cz, cy * sz, -sy, 0.0],
            [sx * sy * cz - cx * sz, sx * sy * sz + cx * cz, sx * cy, 0.0],
            [cx * sy * cz + sx * sz, cx * sy * sz - sx * cz, cx * cy, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let scale_matrix = [
            [scale3d[0] / 100.0, 0.0, 0.0, 0.0],
            [0.0, scale3d[1] / 100.0, 0.0, 0.0],
            [0.0, 0.0, scale3d[2] / 100.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let pos_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [pos3d[0] / (self.width as f32 * 0.5), pos3d[1] / (self.height as f32 * 0.5), pos3d[2] / 1000.0, 1.0],
        ];

        fn m4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
            let mut out = [[0.0; 4]; 4];
            for r in 0..4 {
                for c in 0..4 {
                    out[r][c] = a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c] + a[r][3] * b[3][c];
                }
            }
            out
        }

        m4_mul(proj_3d, m4_mul(pos_matrix, m4_mul(rot_matrix, scale_matrix)))
    }

    /// Evenly distribute selected layers spatially along horizontal (X) or vertical (Y) axes.
    pub fn distribute_selected_layers(&mut self, selected_indices: &[usize], horizontal: bool, frame: u32) {
        if selected_indices.len() < 3 {
            return;
        }
        let fps = self.fps;

        let mut layers_info: Vec<(usize, f32)> = selected_indices
            .iter()
            .copied()
            .filter(|&idx| idx < self.layers.len())
            .map(|idx| {
                let pos = self.layers[idx].transform.eval_position(frame, fps);
                let coord = if horizontal { pos[0] } else { pos[1] };
                (idx, coord)
            })
            .collect();

        if layers_info.len() < 3 {
            return;
        }

        layers_info.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let (Some(min_pos), Some(max_pos)) = (layers_info.first().map(|e| e.1), layers_info.last().map(|e| e.1)) else {
            return;
        };
        let count = layers_info.len();

        // Guard: if all layers are at the same coordinate, distribution is a no-op
        if (max_pos - min_pos).abs() < 0.001 {
            return;
        }

        let step = (max_pos - min_pos) / (count - 1) as f32;

        for (i, &(layer_idx, _)) in layers_info.iter().enumerate() {
            let target_coord = min_pos + step * i as f32;
            let current_pos = self.layers[layer_idx].transform.eval_position(frame, fps);
            let new_pos = if horizontal {
                [target_coord, current_pos[1]]
            } else {
                [current_pos[0], target_coord]
            };
            self.layers[layer_idx].transform.position = crate::core::property::Animatable::new_constant(new_pos);
        }
    }

    /// Pre-compose selected layers into a new nested Sub-Composition (AE Parity).
    /// Returns the newly created Sub-Composition if successful.
    pub fn precompose_layers(
        &mut self,
        layer_ids: &[String],
        new_comp_id: String,
        new_comp_name: String,
        mode: PrecompAttributesMode,
    ) -> Option<Composition> {
        if layer_ids.is_empty() {
            return None;
        }

        // Find lowest index of selected layers in parent comp to place the new precomp layer at that spot
        let mut min_idx = usize::MAX;
        let mut extracted_layers = Vec::new();

        let mut remaining_layers = Vec::new();
        for (i, layer) in self.layers.drain(..).enumerate() {
            if layer_ids.iter().any(|id| id == &layer.id) {
                min_idx = min_idx.min(i);
                extracted_layers.push(layer);
            } else {
                remaining_layers.push(layer);
            }
        }
        self.layers = remaining_layers;

        if extracted_layers.is_empty() {
            return None;
        }

        let mut new_sub_comp = Composition::new(
            new_comp_id.clone(),
            new_comp_name.clone(),
            self.width,
            self.height,
            self.fps,
            self.duration_frames,
        );

        let mut precomp_layer = Layer::new(
            format!("layer_precomp_{}", new_comp_id),
            new_comp_name,
            LayerType::PreComp { comp_id: new_comp_id.clone() },
            self.duration_frames,
        );
        precomp_layer.transform.position = Animatable::new_constant([self.width as f32 * 0.5, self.height as f32 * 0.5]);

        match mode {
            PrecompAttributesMode::MoveToNewComp => {
                // Move all layers and attributes into sub-comp
                new_sub_comp.layers = extracted_layers;
            }
            PrecompAttributesMode::LeaveInParent => {
                // Single-layer only: move source layer into sub-comp with default transform,
                // and keep original transform & effects on the precomp layer in the parent.
                if let Some(mut single_layer) = extracted_layers.into_iter().next() {
                    precomp_layer.transform = single_layer.transform.clone();
                    precomp_layer.effects = std::mem::take(&mut single_layer.effects);
                    precomp_layer.masks = std::mem::take(&mut single_layer.masks);
                    single_layer.transform = Transform2D::default();
                    new_sub_comp.layers = vec![single_layer];
                }
            }
        }

        let insert_idx = min_idx.min(self.layers.len());
        self.layers.insert(insert_idx, precomp_layer);
        self.sub_compositions.push(new_sub_comp.clone());

        Some(new_sub_comp)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecompAttributesMode {
    /// Leaves transforms, effects, and masks on the new precomp layer in the current comp. (Single layer only)
    LeaveInParent,
    /// Moves all layer transforms, effects, and masks inside the new sub-composition.
    MoveToNewComp,
}

// ─── Project Item & Asset Management ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectItemType {
    Composition { comp_idx: usize },
    Image { path: String, width: u32, height: u32 },
    Video { path: String, duration_sec: f32 },
    Audio { path: String, duration_sec: f32 },
    Solid { color: [f32; 4] },
    Folder { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub name: String,
    pub item_type: ProjectItemType,
    /// Containing folder (id of the Folder item), None = project root.
    #[serde(default)]
    pub parent_folder: Option<String>,
}

impl ProjectItem {
    pub fn new(id: impl Into<String>, name: impl Into<String>, item_type: ProjectItemType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            item_type,
            parent_folder: None,
        }
    }
}

// ─── Project ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub compositions: Vec<Composition>,
    pub active_composition_idx: usize,
    pub assets: Vec<ProjectItem>,
    /// Render engine preference persisted with the project:
    /// GPU compute effects (blurs) run when true AND a compatible adapter exists.
    #[serde(default)]
    pub use_gpu_compute: bool,
}

impl Default for Project {
    fn default() -> Self {
        let mut comp = Composition::new(
            "comp_main".to_string(),
            "Main Comp 1".to_string(),
            1920,
            1080,
            30,
            300,
        );

        // Background Solid
        let mut bg_layer = Layer::new(
            "layer_bg".to_string(),
            "Background".to_string(),
            LayerType::Solid {
                color: [0.08, 0.08, 0.12, 1.0],
            },
            300,
        );
        bg_layer.transform.scale = Animatable::new_constant([1920.0, 1080.0]);
        bg_layer.transform.position = Animatable::new_constant([960.0, 540.0]);
        bg_layer.label = LabelColor::Blue;
        comp.add_layer(bg_layer);

        // Main Title Text
        let mut text_layer = Layer::new(
            "layer_text".to_string(),
            "Main Title".to_string(),
            LayerType::new_text("After Effects OSS", 64, [1.0, 1.0, 1.0, 1.0]),
            300,
        );
        text_layer.transform.position = Animatable::new_constant([960.0, 500.0]);
        text_layer.label = LabelColor::Aqua;
        comp.add_layer(text_layer);

        // Null Object controller
        let mut null_ctrl = Layer::new_null(
            "layer_null_ctrl".to_string(),
            "Controller [NULL]".to_string(),
            300,
        );
        null_ctrl.transform.position = Animatable::new_constant([960.0, 540.0]);
        null_ctrl.label = LabelColor::Peach;
        comp.add_layer(null_ctrl);

        let default_assets = vec![
            ProjectItem::new("item_comp1", "Main Comp 1", ProjectItemType::Composition { comp_idx: 0 }),
            ProjectItem::new("item_bg_solid", "Dark Solid Background", ProjectItemType::Solid { color: [0.08, 0.08, 0.12, 1.0] }),
            ProjectItem::new("item_logo", "Logo_Vector.svg", ProjectItemType::Image { path: "assets/logo.svg".to_string(), width: 512, height: 512 }),
            ProjectItem::new("item_audio", "Intro_BGM.wav", ProjectItemType::Audio { path: "assets/audio.wav".to_string(), duration_sec: 10.0 }),
            ProjectItem::new("item_solids_folder", "Solids", ProjectItemType::Folder { name: "Solids".to_string() }),
        ];

        Self {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: default_assets,
            use_gpu_compute: false,
        }
    }
}

impl Project {
    pub fn active_composition(&self) -> &Composition {
        let idx = self.active_composition_idx.min(self.compositions.len().saturating_sub(1));
        &self.compositions[idx]
    }

    pub fn active_composition_mut(&mut self) -> &mut Composition {
        let idx = self.active_composition_idx.min(self.compositions.len().saturating_sub(1));
        &mut self.compositions[idx]
    }

    /// Collect all unique external asset file paths used across compositions and project items (Collect Files).
    pub fn collect_dependencies(&self) -> Vec<String> {
        let mut deps = std::collections::BTreeSet::new();
        // Project items
        for item in &self.assets {
            match &item.item_type {
                ProjectItemType::Image { path, .. }
                | ProjectItemType::Video { path, .. }
                | ProjectItemType::Audio { path, .. } => {
                    if !path.is_empty() {
                        deps.insert(path.clone());
                    }
                }
                _ => {}
            }
        }
        // Layers across all compositions (including sub_compositions)
        let mut visit_comp = |c: &Composition| {
            for l in &c.layers {
                match &l.layer_type {
                    LayerType::Image { path, .. } | LayerType::Audio { path, .. } => {
                        if !path.is_empty() {
                            deps.insert(path.clone());
                        }
                    }
                    LayerType::Video { source, frames_dir, .. } => {
                        if !source.is_empty() {
                            deps.insert(source.clone());
                        }
                        if !frames_dir.is_empty() {
                            deps.insert(frames_dir.clone());
                        }
                    }
                    _ => {}
                }
            }
        };
        for comp in &self.compositions {
            visit_comp(comp);
            for sub in &comp.sub_compositions {
                visit_comp(sub);
            }
        }
        deps.into_iter().collect()
    }
}

// ──────────────── Keyframe & Timeline Assistant Tools ────────────────

/// Rove Across Time (AE Parity):
/// Automatically spaces intermediate keyframes in time according to the spatial distance
/// between their 2D positions, keeping the overall motion velocity constant between the first and last keyframe.
pub fn rove_across_time(keyframes: &mut [crate::core::keyframe::Keyframe<[f32; 2]>]) {
    if keyframes.len() < 3 {
        return;
    }
    let first_frame = keyframes.first().unwrap().frame as f32;
    let last_frame = keyframes.last().unwrap().frame as f32;
    let total_time_span = last_frame - first_frame;
    if total_time_span <= 0.0 {
        return;
    }

    // Compute segment distances
    let mut distances = Vec::with_capacity(keyframes.len() - 1);
    let mut total_distance = 0.0f32;
    for i in 0..keyframes.len() - 1 {
        let p0 = keyframes[i].value;
        let p1 = keyframes[i + 1].value;
        let dist = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt();
        distances.push(dist);
        total_distance += dist;
    }

    if total_distance <= 0.001 {
        return;
    }

    // Distribute intermediate keyframes
    let mut accumulated = 0.0f32;
    for i in 1..keyframes.len() - 1 {
        accumulated += distances[i - 1];
        let ratio = accumulated / total_distance;
        keyframes[i].frame = (first_frame + ratio * total_time_span).round() as u32;
    }
}

/// Sequence Layers (AE Parity):
/// Arranges multiple layers end-to-end in time with optional overlap and opacity crossfade.
pub fn sequence_layers(layers: &mut [Layer], overlap_frames: u32, crossfade: bool) {
    if layers.is_empty() {
        return;
    }
    let mut cur_in = layers[0].in_frame;
    for layer in layers.iter_mut() {
        let duration = layer.out_frame.saturating_sub(layer.in_frame).max(1);
        layer.in_frame = cur_in;
        layer.out_frame = cur_in + duration;

        if crossfade && overlap_frames > 0 {
            // Build crossfade opacity keyframes
            let fade = overlap_frames.min(duration / 2);
            let mut kfs = Vec::new();
            kfs.push(crate::core::keyframe::Keyframe::new(cur_in, 0.0, crate::core::keyframe::InterpolationType::Linear));
            kfs.push(crate::core::keyframe::Keyframe::new(cur_in + fade, 100.0, crate::core::keyframe::InterpolationType::Linear));
            kfs.push(crate::core::keyframe::Keyframe::new(layer.out_frame.saturating_sub(fade), 100.0, crate::core::keyframe::InterpolationType::Linear));
            kfs.push(crate::core::keyframe::Keyframe::new(layer.out_frame, 0.0, crate::core::keyframe::InterpolationType::Linear));
            layer.transform.opacity = Animatable::new_animated(kfs);
        }

        cur_in = (layer.out_frame.saturating_sub(overlap_frames)).max(cur_in);
    }
}

/// Export timeline markers formatted as YouTube / Video Chapters string.
/// Format: `00:00 Intro\n01:23 Section Title\n...`
pub fn export_youtube_chapters(markers: &[TimelineMarker], fps: u32) -> String {
    let fps = fps.max(1);
    let mut sorted_markers = markers.to_vec();
    sorted_markers.sort_by_key(|m| m.frame);

    let mut lines = Vec::new();
    // Ensure 00:00 exists if first marker isn't at frame 0
    if sorted_markers.first().map(|m| m.frame).unwrap_or(1) > 0 {
        lines.push("00:00 Intro".to_string());
    }

    for m in &sorted_markers {
        let total_sec = m.frame / fps;
        let sec = total_sec % 60;
        let min = (total_sec / 60) % 60;
        let hrs = total_sec / 3600;
        let comment = if m.label.is_empty() { format!("Chapter {}", lines.len() + 1) } else { m.label.clone() };

        let time_str = if hrs > 0 {
            format!("{:02}:{:02}:{:02}", hrs, min, sec)
        } else {
            format!("{:02}:{:02}", min, sec)
        };
        lines.push(format!("{} {}", time_str, comment));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remap_frame_for_loop_cycle() {
        // Keyframe interval: 0 to 30 (span = 30)
        // Frame 15 -> 15
        assert_eq!(remap_frame_for_loop(15, 0, 30, false), 15);
        // Frame 30 -> 30
        assert_eq!(remap_frame_for_loop(30, 0, 30, false), 30);
        // Frame 45 -> (45-0)%30 = 15
        assert_eq!(remap_frame_for_loop(45, 0, 30, false), 15);
    }

    #[test]
    fn test_remap_frame_for_loop_pingpong() {
        // Keyframe interval: 0 to 30 (span = 30)
        // Cycle 0 (0..30): frame 15 -> 15
        assert_eq!(remap_frame_for_loop(15, 0, 30, true), 15);
        // Cycle 1 (30..60, odd cycle = reverse): frame 45 -> 30 - 15 = 15
        assert_eq!(remap_frame_for_loop(45, 0, 30, true), 15);
        // Cycle 2 (60..90, even cycle = forward): frame 75 -> 0 + 15 = 15
        assert_eq!(remap_frame_for_loop(75, 0, 30, true), 15);
    }

    #[test]
    fn test_resolve_all_world_transforms_cached() {
        let mut comp = Composition::new("c1".into(), "Comp".into(), 1920, 1080, 30, 300);
        let mut parent = Layer::new("p1".into(), "Parent".into(), LayerType::Null, 300);
        parent.transform.position = Animatable::new_constant([100.0, 100.0]);

        let mut child = Layer::new("c1".into(), "Child".into(), LayerType::Null, 300);
        child.parent_id = Some("p1".into());
        child.transform.position = Animatable::new_constant([50.0, 50.0]);

        comp.add_layer(parent);
        comp.add_layer(child);

        let cached = comp.resolve_all_world_transforms_cached(0);
        assert_eq!(cached.len(), 2);
        assert_eq!(cached.get("c1").unwrap().0, [150.0, 150.0]);
    }

    #[test]
    fn test_parent_cycle_prevention() {
        let mut comp = Composition::new("c1".into(), "Comp".into(), 1920, 1080, 30, 300);
        let l1 = Layer::new("l1".into(), "L1".into(), LayerType::Null, 300);
        let l2 = Layer::new("l2".into(), "L2".into(), LayerType::Null, 300);

        comp.add_layer(l1);
        comp.add_layer(l2);

        assert!(comp.set_layer_parent("l2", Some("l1".into())));
        assert!(!comp.set_layer_parent("l1", Some("l2".into())), "Cycle parent assignment must be rejected");
    }

    #[test]
    fn test_rove_across_time() {
        let mut kfs = vec![
            crate::core::keyframe::Keyframe::new(0, [0.0, 0.0], crate::core::keyframe::InterpolationType::Linear),
            crate::core::keyframe::Keyframe::new(10, [100.0, 0.0], crate::core::keyframe::InterpolationType::Linear), // Intermediate point at 50% distance
            crate::core::keyframe::Keyframe::new(60, [200.0, 0.0], crate::core::keyframe::InterpolationType::Linear),
        ];
        rove_across_time(&mut kfs);
        // Distance 0->100 is 100, 100->200 is 100 => midpoint should be remapped from 10 to 30
        assert_eq!(kfs[0].frame, 0);
        assert_eq!(kfs[1].frame, 30);
        assert_eq!(kfs[2].frame, 60);
    }

    #[test]
    fn test_sequence_layers() {
        let mut l1 = Layer::new("1".into(), "L1".into(), LayerType::Null, 30);
        l1.in_frame = 0; l1.out_frame = 30;
        let mut l2 = Layer::new("2".into(), "L2".into(), LayerType::Null, 30);
        l2.in_frame = 0; l2.out_frame = 30;

        let mut layers = vec![l1, l2];
        sequence_layers(&mut layers, 5, true);

        assert_eq!(layers[0].in_frame, 0);
        assert_eq!(layers[0].out_frame, 30);
        assert_eq!(layers[1].in_frame, 25);
        assert_eq!(layers[1].out_frame, 55);
    }

    #[test]
    fn test_export_youtube_chapters() {
        let markers = vec![
            TimelineMarker { frame: 0, label: "Introduction".into(), color: [1.0, 1.0, 1.0] },
            TimelineMarker { frame: 90, label: "Main Feature".into(), color: [1.0, 1.0, 1.0] },
        ];
        let chapters = export_youtube_chapters(&markers, 30);
        assert!(chapters.contains("00:00 Introduction"));
        assert!(chapters.contains("00:03 Main Feature"));
    }

    #[test]
    fn test_precompose_layers_move_and_leave_attributes() {
        let mut comp = Composition::new("main".into(), "Main".into(), 1920, 1080, 30, 300);
        let mut l1 = Layer::new("l1".into(), "Text 1".into(), LayerType::Null, 300);
        l1.transform.position = Animatable::new_constant([123.0, 456.0]);
        comp.add_layer(l1);

        let sub_comp = comp.precompose_layers(
            &["l1".into()],
            "sub1".into(),
            "Sub Comp 1".into(),
            PrecompAttributesMode::MoveToNewComp,
        ).expect("precompose should succeed");

        assert_eq!(comp.layers.len(), 1);
        assert!(matches!(comp.layers[0].layer_type, LayerType::PreComp { .. }));
        assert_eq!(sub_comp.layers.len(), 1);
        assert_eq!(sub_comp.layers[0].transform.position.evaluate(0), [123.0, 456.0]);
    }
}

#[cfg(test)]
mod multi_camera_tests {
    use super::*;

    #[test]
    fn test_resolve_camera_prefers_active_flag() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);
        assert_eq!(comp.resolve_camera().name, "Active Camera", "legacy fallback");

        let mut wide = Camera3D::default();
        wide.name = "Wide".into();
        wide.fov_degrees = 90.0;
        wide.active = false;
        comp.cameras.push(wide);
        // No active scene camera → legacy active_camera wins
        assert_eq!(comp.resolve_camera().fov_degrees, 50.0);

        comp.set_active_camera(Some(0));
        let cam = comp.resolve_camera();
        assert!((cam.fov_degrees - 90.0).abs() < 1e-4, "active camera wins");
    }

    #[test]
    fn test_set_active_camera_exclusive() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);
        for i in 0..2 {
            let mut c = Camera3D::default();
            c.name = format!("Cam{}", i);
            c.active = false;
            comp.cameras.push(c);
        }
        comp.set_active_camera(Some(1));
        assert!(comp.cameras[1].active);
        assert!(!comp.cameras[0].active);
        comp.set_active_camera(Some(0));
        assert!(comp.cameras[0].active && !comp.cameras[1].active);
        comp.set_active_camera(None);
        assert!(comp.cameras.iter().all(|c| !c.active), "None clears all");
        // Falls back to legacy when nothing active
        assert_eq!(comp.resolve_camera().name, "Active Camera");
    }

    #[test]
    fn test_resolve_camera_mut_targets_active() {
        let mut comp = Composition::new("c".into(), "C".into(), 100, 100, 30, 30);
        let mut c = Camera3D::default();
        c.name = "B".into();
        comp.cameras.push(c);
        comp.set_active_camera(Some(0));
        comp.resolve_camera_mut().fov_degrees = 120.0;
        assert!((comp.cameras[0].fov_degrees - 120.0).abs() < 1e-4);
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    /// Camera pulled back on -Z looking toward +Z (matches renderer convention:
    /// visible points have larger z than the camera).
    fn default_cam() -> Camera3D {
        let mut cam = Camera3D::default();
        cam.transform.position = Animatable::new_constant([0.0, 0.0, -600.0]);
        cam
    }

    #[test]
    fn test_project_point_centered_in_front() {
        let cam = default_cam();
        let sp = project_point_to_screen(&cam, [0.0, 0.0, -500.0], 1920.0, 1080.0);
        if let Some([x, y]) = sp {
            assert!((x - 0.5).abs() < 0.01, "centered point → 0.5, got {x}");
            assert!((y - 0.5).abs() < 0.01, "centered point → 0.5, got {y}");
        } else {
            panic!("point in front of camera must project");
        }
    }

    #[test]
    fn test_project_point_behind_camera_none() {
        let cam = default_cam();
        let sp = project_point_to_screen(&cam, [0.0, 0.0, -900.0], 1920.0, 1080.0);
        assert!(sp.is_none(), "point behind camera returns None");
    }

    #[test]
    fn test_project_point_right_of_center() {
        let cam = default_cam();
        let sp = project_point_to_screen(&cam, [250.0, 0.0, -500.0], 1000.0, 1000.0);
        let [x, _] = sp.expect("must project");
        assert!(x > 0.5, "point right of optical axis → x > 0.5, got {x}");
    }

    #[test]
    fn test_projection_matches_documented_formulas() {
        let cam = default_cam(); // pos [0,0,-600], fov 50
        let pos = [300.0, 200.0, -500.0];
        let sp = project_point_to_screen(&cam, pos, 1920.0, 1080.0).expect("projects");
        // Reference: d = point - campos; focal from vertical fov; sy flipped
        let focal = (1080.0 * 0.5) / (50f32.to_radians() * 0.5).tan();
        let expect_x = (960.0 + 300.0 * focal / 100.0) / 1920.0;
        let expect_y = (540.0 - 200.0 * focal / 100.0) / 1080.0;
        assert!((sp[0] - expect_x).abs() < 1e-4);
        assert!((sp[1] - expect_y).abs() < 1e-4);
    }
}


#[cfg(test)]
mod robustness_tests {
    use super::*;
    use crate::core::property::Animatable;

    #[test]
    fn test_cyclic_parent_chain_does_not_recurse_forever() {
        // Hand-edited / corrupted project: A -> B -> A
        let mut comp = Composition::new("c".into(), "Comp".into(), 64, 64, 30, 30);
        let mut a = Layer::new("a".into(), "A".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        let mut b = Layer::new("b".into(), "B".into(), LayerType::Null, 30);
        a.parent_id = Some("b".to_string());
        b.parent_id = Some("a".to_string());
        comp.layers.push(a);
        comp.layers.push(b);

        // Must not stack overflow; returns some sane transform
        let layer = &comp.layers[0];
        let (pos, scale, _rot, _opa) = comp.resolve_world_transform(layer, 0);
        assert!(pos.iter().all(|v| v.is_finite()), "position must be finite");
        assert!(scale.iter().all(|v| v.is_finite()), "scale must be finite");
    }

    #[test]
    fn test_self_parent_is_safe_at_render() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 32, 32, 30, 30);
        let mut l = Layer::new("s".into(), "SelfParent".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        l.parent_id = Some("s".to_string());
        comp.layers.push(l);

        let pixels = crate::core::software_renderer::render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }

    #[test]
    fn test_cyclic_precomp_nesting_terminates() {
        // Sub-comp A contains a PreComp layer pointing to B, B points back to A
        let mut comp_a = Composition::new("A".into(), "CompA".into(), 32, 32, 30, 30);
        let comp_b = Composition::new("B".into(), "CompB".into(), 32, 32, 30, 30);
        let pc = Layer::new("pc".into(), "Nested".into(), LayerType::PreComp { comp_id: "B".into() }, 30);
        comp_a.layers.push(pc);
        comp_a.sub_compositions.push(comp_b);
        let pc_back = Layer::new("pcb".into(), "Back".into(), LayerType::PreComp { comp_id: "A".into() }, 30);
        comp_a.sub_compositions[0].layers.push(pc_back);

        // Must terminate without stack overflow
        let pixels = crate::core::software_renderer::render_frame_to_pixels(&comp_a, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }

    #[test]
    fn test_nan_and_infinite_property_values_render_safely() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 32, 32, 30, 30);
        let mut l = Layer::new("n".into(), "NaN Layer".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        // Corrupted values via extreme keyframes
        l.transform.position = Animatable::new_animated(vec![
            crate::core::keyframe::Keyframe::new(0, [f32::NAN, f32::INFINITY], crate::core::keyframe::InterpolationType::Linear),
            crate::core::keyframe::Keyframe::new(10, [f32::NEG_INFINITY, 1e30], crate::core::keyframe::InterpolationType::Linear),
        ]);
        l.transform.scale = Animatable::new_constant([f32::NAN, -1e20]);
        l.transform.opacity = Animatable::new_constant(f32::NAN);
        comp.layers.push(l);

        // Must not panic or hang
        let pixels = crate::core::software_renderer::render_frame_to_pixels(&comp, 5, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }

    #[test]
    fn test_malformed_project_json_fails_gracefully() {
        let bad_payloads = [
            "",
            "{",
            "null",
            r#"{"compositions": "not-an-array"}"#,
            r#"{"compositions":[{"layers": 42}]}"#,
            "\u{0}\u{1}\u{2}",
        ];
        for payload in bad_payloads {
            let result: Result<Project, _> = serde_json::from_str(payload);
            assert!(result.is_err(), "malformed JSON must be rejected: {:?}", payload);
        }
    }

    #[test]
    fn test_expression_injection_is_contained() {
        // Expression sandbox must reject resource-abusive scripts without panicking
        use crate::core::expression_engine::{build_engine, eval_f32};
        let engine = build_engine();
        // Deep recursion attempt — should be capped or error out, never crash
        let evil = "fn f(n) { if n <= 0 { 0 } else { f(n-1) + 1 } } f(100000)";
        let _ = eval_f32(&engine, evil, 0.0, 0, 30); // must return fallback, not crash
        // Huge loop via string building — capped by max_operations
        let evil2 = "let s = \\\"\\\"; for i in 0..1000000 { s += \\\"x\\\"; } 42";
        let _ = eval_f32(&engine, evil2, 0.0, 0, 30);
    }
}

#[cfg(test)]
mod layer_time_ops_tests {
    use super::*;

    fn test_layer() -> Layer {
        let mut l = Layer::new(
            "l".into(),
            "L".into(),
            LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] },
            60,
        );
        l.in_frame = 10;
        l.out_frame = 40;
        l
    }

    #[test]
    fn test_enable_time_remapping_is_identity() {
        let mut l = test_layer();
        l.enable_time_remapping();
        assert_eq!(l.remap_frame(10), 10);
        assert_eq!(l.remap_frame(25), 25);
        assert_eq!(l.remap_frame(40), 40);
        // Idempotent: calling again must not clobber existing keyframes.
        l.enable_time_remapping();
        assert!(l.time_remap.is_some());
    }

    #[test]
    fn test_clear_time_remap_restores_passthrough() {
        let mut l = test_layer();
        l.freeze_at(17);
        assert_eq!(l.remap_frame(10), 17);
        l.clear_time_remap();
        assert_eq!(l.remap_frame(10), 10);
        assert_eq!(l.remap_frame(33), 33);
    }

    #[test]
    fn test_time_reverse_flaps_endpoints() {
        let mut l = test_layer();
        l.time_reverse();
        assert_eq!(l.remap_frame(10), 40);
        assert_eq!(l.remap_frame(25), 25);
        assert_eq!(l.remap_frame(40), 10);
    }

    #[test]
    fn test_freeze_at_holds_constant() {
        let mut l = test_layer();
        l.freeze_at(17);
        assert_eq!(l.remap_frame(10), 17);
        assert_eq!(l.remap_frame(99), 17);
    }

    #[test]
    fn test_easy_ease_transform_sets_bezier_and_skips_expressions() {
        use crate::core::keyframe::{InterpolationType, Keyframe};
        let mut l = test_layer();
        l.transform.position = Animatable::Animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(30, [100.0, 0.0], InterpolationType::Linear),
        ]);
        l.transform.rotation = Animatable::Animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(30, 90.0, InterpolationType::Linear),
        ]);
        // Expression-driven properties must be left alone.
        l.transform.opacity_expression = Some(crate::core::timeline::Expression::Wiggle {
            frequency: 2.0,
            amplitude: 10.0,
        });
        l.transform.opacity = Animatable::Animated(vec![
            Keyframe::new(0, 100.0, InterpolationType::Linear),
            Keyframe::new(30, 0.0, InterpolationType::Linear),
        ]);

        l.easy_ease_transform();

        if let Some(kfs) = l.transform.position.keyframes() {
            for kf in kfs {
                assert!(matches!(kf.interpolation, InterpolationType::Bezier { .. }));
            }
        } else {
            panic!("position should be animated");
        }
        if let Some(kfs) = l.transform.rotation.keyframes() {
            assert!(matches!(kfs[0].interpolation, InterpolationType::Bezier { .. }));
        }
        // Opacity has an expression: keyframes keep linear interpolation.
        if let Some(kfs) = l.transform.opacity.keyframes() {
            assert!(matches!(kfs[0].interpolation, InterpolationType::Linear));
        }
    }

    #[test]
    fn test_time_stretch_scales_span_and_source_rate() {
        let mut l = test_layer(); // in=10 out=40 (30 frames)
        l.time_stretch(2.0);
        // Span doubles.
        assert_eq!(l.in_frame, 10);
        assert_eq!(l.out_frame, 70);
        // Source advances at half rate: mid-span maps to mid source frame.
        assert_eq!(l.remap_frame(10), 10);
        assert_eq!(l.remap_frame(50), 30);
        assert_eq!(l.remap_frame(70), 40);

        // Speed-up: factor 0.5 halves the span; source runs at double rate.
        let mut fast = test_layer();
        fast.time_stretch(0.5);
        assert_eq!(fast.out_frame, 25);
        assert_eq!(fast.remap_frame(25), 40);

        // Degenerate factor is ignored.
        let mut safe = test_layer();
        safe.time_stretch(0.0);
        assert_eq!(safe.out_frame, 40);
        assert!(safe.time_remap.is_none());
    }

    #[test]
    fn test_easy_ease_on_constants_is_noop() {
        let mut l = test_layer();
        l.easy_ease_transform(); // all constants — must not panic or animate
        assert!(l.transform.position.keyframes().is_none());
    }
}

