use crate::core::property::Animatable;
use crate::core::expression_engine;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeType {
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
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            name: "Active Camera".to_string(),
            active: true,
            fov_degrees: 50.0,
            focus_distance: 1000.0,
            aperture: 2.8,
            transform: Transform3D::default(),
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerStyle {
    pub drop_shadow: DropShadowStyle,
    pub outer_glow: OuterGlowStyle,
    pub stroke: StrokeStyle,
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
    
    // ── AE Blend Mode ──
    pub blend_mode: BlendMode,

    pub is_adjustment_layer: bool,
    pub is_guide_layer: bool,
    pub is_shy: bool,
    pub effects_enabled: bool,
    pub is_collapsed: bool,

    // ── AE Masking System ──
    pub masks: Vec<crate::core::mask::Mask>,

    // ── AE Layer Style System ──
    pub style: LayerStyle,

    // ── Text Formatting System ──
    pub text_formatting: Option<TextFormatting>,

    // ── Text Animator (per-character animation) ──
    #[serde(default)]
    pub text_animator: Option<crate::core::text_animator::TextAnimatorSettings>,

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
            blend_mode: BlendMode::Normal,
            is_adjustment_layer: is_adj,
            is_guide_layer: false,
            is_shy: false,
            effects_enabled: true,
            is_collapsed: false,
            masks: Vec::new(),
            style: LayerStyle::default(),
            text_formatting: None,
            text_animator: None,
            preserve_transparency: false,
            trim_paths: None,
            shape_repeater: None,
            constraints: crate::core::layer_constraints::LayerConstraints::default(),
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

    pub fn remap_frame(&self, frame: u32) -> u32 {
        match &self.time_remap {
            Some(anim) => anim.evaluate(frame) as u32,
            None => frame,
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
    pub lights: Vec<Light3D>,
    pub markers: Vec<TimelineMarker>,

    /// Sub-compositions for PreComp nesting (keyed by comp id).
    #[serde(default)]
    pub sub_compositions: Vec<Composition>,
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
            lights: vec![Light3D::default()],
            markers: Vec::new(),
            sub_compositions: Vec::new(),
        }
    }

    /// Look up a sub-composition by id (recursive search).
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
}

// ─── Project Item & Asset Management ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectItemType {
    Composition { comp_idx: usize },
    Image { path: String, width: u32, height: u32 },
    Audio { path: String, duration_sec: f32 },
    Solid { color: [f32; 4] },
    Folder { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub name: String,
    pub item_type: ProjectItemType,
}

impl ProjectItem {
    pub fn new(id: impl Into<String>, name: impl Into<String>, item_type: ProjectItemType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            item_type,
        }
    }
}

// ─── Project ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub compositions: Vec<Composition>,
    pub active_composition_idx: usize,
    pub assets: Vec<ProjectItem>,
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
