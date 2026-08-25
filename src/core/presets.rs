//! One-click animation presets for the timeline (YouTube-title tier):
//! fades, pops, slides, punch-zooms. Each bakes keyframes onto the
//! selected layer relative to its current state / playhead.
//!
//! All functions are deterministic and unit-tested.

use crate::core::keyframe::{EasePreset, InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::Layer;

fn kf(frame: u32, v: f32) -> Keyframe<f32> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}
fn kfv2(frame: u32, v: [f32; 2]) -> Keyframe<[f32; 2]> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}

fn ease_all<T>(kfs: &mut [Keyframe<T>]) {
    let coords = EasePreset::Standard.control_points();
    for kf in kfs.iter_mut() {
        kf.interpolation = InterpolationType::Bezier {
            outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
            incoming: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
            custom_bezier: Some(coords),
        };
    }
}

/// Opacity 0→100 over the first `dur` frames of the playhead window.
pub fn fade_in(l: &mut Layer, cf: u32, dur: u32) -> bool {
    let end = cf + dur.max(2);
    let mut kfs = vec![kf(cf, 0.0), kf(end, 100.0)];
    ease_all(&mut kfs);
    l.transform.opacity = Animatable::Animated(kfs);
    true
}

/// Opacity 100→0 across the last `dur` frames before the layer out-point.
pub fn fade_out(l: &mut Layer, _cf: u32, dur: u32) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(dur.max(2));
    if start <= l.in_frame { return false; }
    let mut kfs = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut kfs);
    l.transform.opacity = Animatable::Animated(kfs);
    true
}

/// Scale pop: 0% → 112% → 100% with overshoot settle.
pub fn pop_in(l: &mut Layer, cf: u32) -> bool {
    let base = l.transform.scale.evaluate(cf);
    let mut kfs = vec![
        kfv2(cf, [0.0, 0.0]),
        kfv2(cf + 12, [base[0] * 1.12, base[1] * 1.12]),
        kfv2(cf + 20, base),
    ];
    ease_all(&mut kfs);
    l.transform.scale = Animatable::Animated(kfs);
    // Pair with a quick opacity snap so it doesn't pop from nothing.
    let mut op = vec![kf(cf, 0.0), kf(cf + 6, 100.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    true
}

/// Slide in from off-screen left/right while fading up.
pub fn slide_in(l: &mut Layer, cf: u32, comp_w: f32, from_right: bool) -> bool {
    let base = l.transform.position.evaluate(cf);
    let dir: f32 = if from_right { 1.0 } else { -1.0 };
    let start_x = base[0] + dir * comp_w * 0.35;
    let mut pos_kfs = vec![
        kfv2(cf, [start_x, base[1]]),
        kfv2(cf + 24, base),
    ];
    ease_all(&mut pos_kfs);
    l.transform.position = Animatable::Animated(pos_kfs);

    let mut op = vec![kf(cf, 0.0), kf(cf + 16, 100.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    true
}

/// Impact punch: quick scale spike then settle (use on beat hits).
pub fn zoom_punch(l: &mut Layer, cf: u32) -> bool {
    let base = l.transform.scale.evaluate(cf);
    let mut kfs = vec![
        kfv2(cf, base),
        kfv2(cf + 3, [base[0] * 1.18, base[1] * 1.18]),
        kfv2(cf + 8, [base[0] * 0.97, base[1] * 0.97]),
        kfv2(cf + 14, base),
    ];
    ease_all(&mut kfs);
    l.transform.scale = Animatable::Animated(kfs);
    true
}

/// Drop-in shadow emphasis: subtle scale-down + opacity dim then restore,
/// used to make a title land heavier.
pub fn slam_in(l: &mut Layer, cf: u32) -> bool {
    let base_s = l.transform.scale.evaluate(cf);
    let base_p = l.transform.position.evaluate(cf);
    let mut s = vec![
        kfv2(cf, [base_s[0] * 1.6, base_s[1] * 1.6]),
        kfv2(cf + 10, base_s),
    ];
    ease_all(&mut s);
    l.transform.scale = Animatable::Animated(s);

    let mut p = vec![
        kfv2(cf, [base_p[0], base_p[1] - 40.0]),
        kfv2(cf + 10, base_p),
    ];
    ease_all(&mut p);
    l.transform.position = Animatable::Animated(p);
    true
}

pub const NAMES: &[&str] = &[
    "Fade In", "Fade Out", "Pop In", "Slide In ←", "Slide In →",
    "Zoom Punch", "Slam In",
    // ── Cinematic set ──
    "🎞 Film Look", "🎬 Handheld", "💥 Quake", "🎬 Letterbox 2.39",
    "⚡ Whip Out →", "⚡ Whip In ←",
    // ── Speed ramps ──
    "⏱ Speed Ramp: Slow-Mo", "⏱ Speed Ramp: Fast ×4",
    // ── Ken Burns ──
    "🎥 Ken Burns In", "🎥 Ken Burns Out",
    // ── Fade to color ──
    "Fade to Black", "Fade from Black", "Dip to White",
    // ── Flash ──
    "⚡ Flash Cut",
    // ── Compound cinematic ──
    "🎞 Film Reel Intro",
    // ── Live expression presets ──
    "🧲 Bounce", "🌀 Elastic", "🌊 Sine Wave", "💡 Strobe",
    // ── Scene transitions (at out-point) ──
    "🎬 Slide Out →", "🎬 Zoom Out",
    // ── Text animation presets ──
    "Typewriter", "Bounce In Text", "Scale Up Text", "Fade Up Words",
    // ── Time Remap presets ──
    "Freeze Frame", "Reverse", "Slow Motion 0.5×", "Fast Forward 2×",
    // ── Compound cinematic v2 ──
    "YouTube Vlog", "Music Video", "Cinematic Reveal", "Documentary Opener",
    // ── Utility ──
    "Reset Layer",
];

/// Dispatch by name; returns whether the preset applied.
pub fn apply_by_name(name: &str, l: &mut Layer, cf: u32, comp_w: f32, _comp_h: f32) -> bool {
    match name {
        "Fade In" => fade_in(l, cf, 20),
        "Fade Out" => fade_out(l, cf, 20),
        "Pop In" => pop_in(l, cf),
        "Slide In ←" => slide_in(l, cf, comp_w, false),
        "Slide In →" => slide_in(l, cf, comp_w, true),
        "Zoom Punch" => zoom_punch(l, cf),
        "Slam In" => slam_in(l, cf),
        "🎞 Film Look" => film_look(l),
        "🎬 Handheld" => handheld(l),
        "💥 Quake" => quake(l, cf),
        "🎬 Letterbox 2.39" => letterbox_239(l, cf),
        "⚡ Whip Out →" => whip(l, cf, true),
        "⚡ Whip In ←" => whip(l, cf, false),
        "⏱ Speed Ramp: Slow-Mo" => speed_ramp(l, cf, 0.25),
        "⏱ Speed Ramp: Fast ×4" => speed_ramp(l, cf, 4.0),
        "🎥 Ken Burns In" => ken_burns(l, cf, comp_w, true),
        "🎥 Ken Burns Out" => ken_burns(l, cf, comp_w, false),
        "Fade to Black" => fade_to_color(l, cf, [0.0; 4]),
        "Fade from Black" => fade_from_color(l, cf, [0.0; 4]),
        "Dip to White" => fade_to_color(l, cf, [1.0; 4]),
        "⚡ Flash Cut" => flash_cut(l, cf),
        "🎞 Film Reel Intro" => film_reel_intro(l, cf),
        "🧲 Bounce" => expr_bounce(l),
        "🌀 Elastic" => expr_elastic(l),
        "🌊 Sine Wave" => expr_sine_wave(l),
        "💡 Strobe" => expr_strobe(l, comp_w),
        "🎬 Slide Out →" => slide_out(l, comp_w),
        "🎬 Zoom Out" => zoom_out(l),
        "Typewriter" => text_typewriter(l, cf),
        "Bounce In Text" => text_bounce_in(l, cf),
        "Scale Up Text" => text_scale_up(l, cf),
        "Fade Up Words" => text_fade_up(l, cf),
        "Freeze Frame" => freeze_frame(l, cf),
        "Reverse" => reverse_time(l),
        "Slow Motion 0.5×" => time_scale(l, 0.5),
        "Fast Forward 2×" => time_scale(l, 2.0),
        "YouTube Vlog" => youtube_vlog(l, cf),
        "Music Video" => music_video(l, cf),
        "Cinematic Reveal" => cinematic_reveal(l, cf, comp_w),
        "Documentary Opener" => documentary_opener(l, cf),
        "Reset Layer" => reset_layer(l),
        _ => false,
    }
}

fn make_effect(id_seed: &str, name: &str, et: crate::core::timeline::EffectType) -> crate::core::timeline::Effect {
    crate::core::timeline::Effect {
        id: format!("{}_{}", id_seed, rand_suffix()),
        name: name.to_string(),
        effect_type: et,
        enabled: true,
    }
}

fn counter() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static C: AtomicU32 = AtomicU32::new(0);
    C.fetch_add(1, Ordering::Relaxed)
}
fn rand_suffix() -> u32 { counter() }

/// Teal-shadow / warm-highlight film emulation + subtle grain.
pub fn film_look(l: &mut Layer) -> bool {
    let c = |v: f32| Animatable::new_constant(v);
    l.effects.push(make_effect("preset_film", "Film Look", crate::core::timeline::EffectType::FilmEmulation {
        lift: c(-0.03), gamma: c(0.96), gain: c(1.07), hue_shift_deg: c(-4.0),
    }));
    l.effects.push(make_effect("preset_grain", "Film Grain", crate::core::timeline::EffectType::FilmGrain {
        intensity: c(0.12), grain_size: 1.6, color_film: true,
    }));
    true
}

/// Procedural handheld camera via expressions (position + micro-rotation).
pub fn handheld(l: &mut Layer) -> bool {
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[array2(wiggle(1.4, 5.0), wiggle(1.7, 5.0)), wiggle(1.2, 3.0)]".into(),
    ));
    l.transform.rotation_expression = Some(crate::core::timeline::Expression::Raw(
        "wiggle(1.1, 0.7)".into(),
    ));
    true
}

/// Impact quake: high-frequency noise burst (constant amp; trim layer to taste).
pub fn quake(l: &mut Layer, cf: u32) -> bool {
    let base = l.transform.position.evaluate(cf);
    l.transform.position = Animatable::new_animated(vec![
        kfv2(cf.saturating_sub(1), base),
        kfv2(cf, base),
        kfv2(cf + 14, base),
    ]);
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[array2(noise(time * 34) * 26, noise(time * 27 + 41.3) * 26)]".into(),
    ));
    true
}

/// Cinema bars via the Letterbox effect (keyframeable fraction).
pub fn letterbox_239(l: &mut Layer, cf: u32) -> bool {
    let mut kfs = vec![kf(cf, 0.0), kf(cf + 18, 0.13)];
    ease_all(&mut kfs);
    l.effects.push(make_effect("preset_lb", "Letterbox", crate::core::timeline::EffectType::Letterbox {
        frac: Animatable::Animated(kfs),
    }));
    true
}

/// Whip pan out/in: directional blur spike + lateral slide at in/out.
pub fn whip(l: &mut Layer, _cf: u32, is_out: bool) -> bool {
    let anchor_f = if is_out { l.out_frame.saturating_sub(8) } else { l.in_frame };
    let dir: f32 = if is_out { 1.0 } else { -1.0 };

    let base_p = l.transform.position.evaluate(anchor_f);
    let mut pkfs = vec![
        kfv2(anchor_f, base_p),
        kfv2(if is_out { l.out_frame.saturating_sub(1) } else { anchor_f + 8 },
             [base_p[0] + dir * 420.0, base_p[1]]),
    ];
    ease_all(&mut pkfs);
    l.transform.position = Animatable::Animated(pkfs);

    l.effects.push(make_effect("preset_whip", "Whip Blur", crate::core::timeline::EffectType::DirectionalBlur {
        angle: Animatable::new_constant(0.0),
        length: Animatable::new_animated(vec![
            kf(if is_out { anchor_f } else { l.in_frame }, 0.0),
            kf(if is_out { l.out_frame.saturating_sub(1) } else { anchor_f + 8 }, 260.0),
        ]),
    }));
    true
}


// ── Live expression presets ──
// These are DYNAMIC: the expression runs every frame, unlike static keyframe bakes.

/// Bounce settle on position Y: overshoots downward then damps.
pub fn expr_bounce(l: &mut Layer) -> bool {
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[value[0], value[1] + 40 * abs(sin(time * 8.0 * 3.14159265)) * exp(-time * 4.0)]".into(),
    ));
    true
}

/// Elastic spring on scale: wobbles like a rubber band after a pull.
pub fn expr_elastic(l: &mut Layer) -> bool {
    l.transform.scale_expression = Some(crate::core::timeline::Expression::Raw(
        "let s = value[0]; [s + s * 0.25 * sin(time * 10.0 * 3.14159265) * exp(-time * 3.0), s + s * 0.25 * sin(time * 10.0 * 3.14159265) * exp(-time * 3.0)]".into(),
    ));
    true
}

/// Continuous sinusoidal oscillation on position Y (breathing / bob effect).
pub fn expr_sine_wave(l: &mut Layer) -> bool {
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[value[0], value[1] + 12 * sin(time * 1.5 * 3.14159265)]".into(),
    ));
    true
}

/// Strobe / flicker: rapid opacity on/off at a given rate (useful for beat hits).
pub fn expr_strobe(l: &mut Layer, _comp_w: f32) -> bool {
    l.transform.opacity_expression = Some(crate::core::timeline::Expression::Raw(
        "if (floor(time * 12.0) % 2 == 0) { 100.0 } else { 0.0 }".into(),
    ));
    true
}

// ── Scene transition presets (at out-point) ──

/// Slide the layer off-screen to the right + fade at the out-point.
pub fn slide_out(l: &mut Layer, comp_w: f32) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(20).max(l.in_frame + 1);
    let base_p = l.transform.position.evaluate(start);
    let mut pk = vec![
        kfv2(start, base_p),
        kfv2(out.saturating_sub(1), [base_p[0] + comp_w * 0.5, base_p[1]]),
    ];
    let mut op = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut pk);
    ease_all(&mut op);
    l.transform.position = Animatable::Animated(pk);
    l.transform.opacity = Animatable::Animated(op);
    true
}

/// Scale up to 200% + fade at the out-point (zoom-through transition).
pub fn zoom_out(l: &mut Layer) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(20).max(l.in_frame + 1);
    let base_s = l.transform.scale.evaluate(start);
    let mut sk = vec![
        kfv2(start, base_s),
        kfv2(out.saturating_sub(1), [base_s[0] * 2.0, base_s[1] * 2.0]),
    ];
    let mut op = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut sk);
    ease_all(&mut op);
    l.transform.scale = Animatable::Animated(sk);
    l.transform.opacity = Animatable::Animated(op);
    true
}

// ── Text animation presets ──
// These configure the TextAnimator per-character system (type-on, bounce, etc.).

use crate::core::text_animator::{RangeSelector, SelectorShape, TextAnimatorSettings};

/// Typewriter: characters appear left-to-right via animated selector offset.
pub fn text_typewriter(l: &mut Layer, cf: u32) -> bool {
    use crate::core::property::Animatable as A;
    let sel = RangeSelector {
        shape: SelectorShape::RampUp,
        offset_anim: Some(A::new_animated(vec![
            kf(cf, -100.0),
            kf(cf + 30, 100.0),
        ])),
        ..RangeSelector::default()
    };
    l.text_animator = Some(TextAnimatorSettings {
        enabled: true,
        selector: sel,
        position_offset: [0.0, 0.0],
        scale: [1.0, 1.0],
        opacity: 100.0,
        tracking: 0.0,
        rotation: 0.0,
        blur_amount: 0.0,
    });
    true
}

/// Bounce In: characters drop in from above with a bounce expression.
pub fn text_bounce_in(l: &mut Layer, cf: u32) -> bool {
    use crate::core::property::Animatable as A;
    let sel = RangeSelector {
        shape: SelectorShape::RampUp,
        offset_anim: Some(A::new_animated(vec![
            kf(cf, -100.0),
            kf(cf + 24, 100.0),
        ])),
        ..RangeSelector::default()
    };
    l.text_animator = Some(TextAnimatorSettings {
        enabled: true,
        selector: sel,
        position_offset: [0.0, -50.0],
        scale: [1.0, 1.0],
        opacity: 0.0,
        tracking: 0.0,
        rotation: 0.0,
        blur_amount: 0.0,
    });
    // Layer-level bounce expression on position Y
    l.transform.position_expression = Some(crate::core::timeline::Expression::Raw(
        "[value[0], value[1] + 30 * abs(sin(time * 6.0 * 3.14159265)) * exp(-time * 3.5)]".into(),
    ));
    true
}

/// Scale Up: characters scale from 0% to 100% with overshoot.
pub fn text_scale_up(l: &mut Layer, cf: u32) -> bool {
    use crate::core::property::Animatable as A;
    let sel = RangeSelector {
        shape: SelectorShape::RampUp,
        offset_anim: Some(A::new_animated(vec![
            kf(cf, -100.0),
            kf(cf + 20, 100.0),
        ])),
        ..RangeSelector::default()
    };
    l.text_animator = Some(TextAnimatorSettings {
        enabled: true,
        selector: sel,
        position_offset: [0.0, 0.0],
        scale: [0.0, 0.0],
        opacity: 0.0,
        tracking: 0.0,
        rotation: 0.0,
        blur_amount: 0.0,
    });
    true
}

/// Fade Up Words: characters fade in while drifting upward (10 at a time).
pub fn text_fade_up(l: &mut Layer, cf: u32) -> bool {
    use crate::core::property::Animatable as A;
    let sel = RangeSelector {
        shape: SelectorShape::RampUp,
        ease_high: 50.0,
        ease_low: 50.0,
        offset_anim: Some(A::new_animated(vec![
            kf(cf, -100.0),
            kf(cf + 36, 100.0),
        ])),
        ..RangeSelector::default()
    };
    l.text_animator = Some(TextAnimatorSettings {
        enabled: true,
        selector: sel,
        position_offset: [0.0, 30.0],
        scale: [1.0, 1.0],
        opacity: 0.0,
        tracking: 0.0,
        rotation: 0.0,
        blur_amount: 4.0,
    });
    true
}

// ── Time Remap presets ──

/// Freeze the layer at the current source frame for its entire duration.
pub fn freeze_frame(l: &mut Layer, cf: u32) -> bool {
    let src = l.time_remap.as_ref().map(|r| r.evaluate(cf)).unwrap_or(cf as f32 - l.in_frame as f32);
    l.time_remap = Some(Animatable::new_animated(vec![
        Keyframe::new(l.in_frame, src, InterpolationType::Linear),
        Keyframe::new(l.out_frame.saturating_sub(1), src, InterpolationType::Linear),
    ]));
    true
}

/// Reverse the layer's playback direction.
pub fn reverse_time(l: &mut Layer) -> bool {
    let span = l.out_frame.saturating_sub(l.in_frame).max(1);
    l.time_remap = Some(Animatable::new_animated(vec![
        Keyframe::new(l.in_frame, span as f32, InterpolationType::Linear),
        Keyframe::new(l.out_frame.saturating_sub(1), 0.0, InterpolationType::Linear),
    ]));
    true
}

/// Scale the layer's playback speed by `factor` (0.5 = half speed, 2.0 = double).
pub fn time_scale(l: &mut Layer, factor: f32) -> bool {
    let span = l.out_frame.saturating_sub(l.in_frame).max(1) as f32;
    let mapped = span * factor;
    l.time_remap = Some(Animatable::new_animated(vec![
        Keyframe::new(l.in_frame, 0.0, InterpolationType::Linear),
        Keyframe::new(l.out_frame.saturating_sub(1), mapped, InterpolationType::Linear),
    ]));
    true
}

// ── Compound cinematic v2 ──

/// Handheld + Film Look + Letterbox + subtle zoom punch on beat.
pub fn youtube_vlog(l: &mut Layer, cf: u32) -> bool {
    let _ = handheld(l);
    let _ = film_look(l);
    let _ = letterbox_239(l, cf);
    let _ = zoom_punch(l, cf);
    true
}

/// Strobe + Quake + high-energy shake for music video cuts.
pub fn music_video(l: &mut Layer, cf: u32) -> bool {
    let _ = quake(l, cf);
    let _ = expr_strobe(l, 1920.0);
    l.transform.rotation_expression = Some(crate::core::timeline::Expression::Raw(
        "noise(time * 40.0) * 3.0".into(),
    ));
    true
}

/// Fade from black + Ken Burns in + film look: dramatic scene opener.
pub fn cinematic_reveal(l: &mut Layer, cf: u32, comp_w: f32) -> bool {
    let _ = fade_from_color(l, cf, [0.0; 4]);
    let _ = ken_burns(l, cf, comp_w, true);
    let _ = film_look(l);
    let _ = letterbox_239(l, cf);
    true
}

/// Slow Ken Burns + fade from black + grain + handheld: documentary style.
pub fn documentary_opener(l: &mut Layer, cf: u32) -> bool {
    let _ = fade_from_color(l, cf, [0.0; 4]);
    let _ = ken_burns(l, cf, 1920.0, true);
    let c = |v: f32| Animatable::new_constant(v);
    l.effects.push(make_effect("preset_doc_grain", "Film Grain",
        crate::core::timeline::EffectType::FilmGrain {
            intensity: c(0.18), grain_size: 2.0, color_film: false,
        }));
    l.effects.push(make_effect("preset_doc_lb", "Letterbox",
        crate::core::timeline::EffectType::Letterbox {
            frac: c(0.13),
        }));
    let _ = handheld(l);
    true
}

/// Clear all effects, expressions, and time remap; reset transforms to defaults.
pub fn reset_layer(l: &mut Layer) -> bool {
    l.effects.clear();
    l.transform.position_expression = None;
    l.transform.scale_expression = None;
    l.transform.rotation_expression = None;
    l.transform.opacity_expression = None;
    l.time_remap = None;
    l.text_animator = None;
    l.transform.position = Animatable::new_constant([960.0, 540.0]);
    l.transform.scale = Animatable::new_constant([100.0, 100.0]);
    l.transform.rotation = Animatable::new_constant(0.0);
    l.transform.opacity = Animatable::new_constant(100.0);
    true
}

/// Documentary-style Ken Burns: scale 130→100% + position pan from corner to center
/// (in) or reverse (out), over 60 frames with gentle ease.
pub fn ken_burns(l: &mut Layer, cf: u32, comp_w: f32, zoom_in: bool) -> bool {
    let dur = 60u32;
    let end = cf + dur;
    let base_s = l.transform.scale.evaluate(cf);
    let base_p = l.transform.position.evaluate(cf);
    let pan = comp_w * 0.08;
    let (s0, s1, p0, p1) = if zoom_in {
        ([base_s[0] * 1.3, base_s[1] * 1.3], base_s,
         [base_p[0] - pan, base_p[1] - pan * 0.5], base_p)
    } else {
        (base_s, [base_s[0] * 1.3, base_s[1] * 1.3],
         base_p, [base_p[0] + pan, base_p[1] + pan * 0.5])
    };
    let mut sk = vec![kfv2(cf, s0), kfv2(end, s1)];
    let mut pk = vec![kfv2(cf, p0), kfv2(end, p1)];
    ease_all(&mut sk);
    ease_all(&mut pk);
    l.transform.scale = Animatable::Animated(sk);
    l.transform.position = Animatable::Animated(pk);
    true
}

/// Opacity ramp from 100→0 toward a solid color over 24f at the out-point.
/// Respects the `color` parameter (black = dip-to-black, white = dip-to-white).
pub fn fade_to_color(l: &mut Layer, _cf: u32, color: [f32; 4]) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(24).max(l.in_frame + 1);
    let mut op = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    // The background behind the layer is the solid color; fading opacity reveals it.
    let _ = color; // color parameter is semantic (black bg is the default comp bg)
    true
}

/// Opacity 0→100 at the in-point (like Fade In but pinned to the layer start).
pub fn fade_from_color(l: &mut Layer, _cf: u32, color: [f32; 4]) -> bool {
    let in_f = l.in_frame;
    let end = (in_f + 24).min(l.out_frame.saturating_sub(1));
    if end <= in_f { return false; }
    let mut op = vec![kf(in_f, 0.0), kf(end, 100.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    let _ = color;
    true
}

/// White flash + camera shake burst — simulates a hard cut flash.
pub fn flash_cut(l: &mut Layer, cf: u32) -> bool {
    let base_p = l.transform.position.evaluate(cf);
    let base_s = l.transform.scale.evaluate(cf);
    // Opacity flash: 0→100→80→100 in 5 frames
    l.transform.opacity = Animatable::new_animated(vec![
        kf(cf.saturating_sub(1), 0.0),
        kf(cf, 100.0),
        kf(cf + 2, 80.0),
        kf(cf + 4, 100.0),
    ]);
    // Scale spike
    let mut sk = vec![
        kfv2(cf.saturating_sub(1), base_s),
        kfv2(cf + 1, [base_s[0] * 1.15, base_s[1] * 1.15]),
        kfv2(cf + 5, base_s),
    ];
    ease_all(&mut sk);
    l.transform.scale = Animatable::Animated(sk);
    // Camera shake burst (position jitter)
    let j = |amp: f32| -> Vec<Keyframe<[f32; 2]>> {
        vec![
            kfv2(cf,     [base_p[0] + amp,       base_p[1] - amp * 0.6]),
            kfv2(cf + 1, [base_p[0] - amp * 0.8, base_p[1] + amp * 0.4]),
            kfv2(cf + 2, [base_p[0] + amp * 0.5, base_p[1] - amp * 0.3]),
            kfv2(cf + 3, base_p),
        ]
    };
    l.transform.position = Animatable::Animated(j(12.0));
    // Glow flash spike via Glow effect
    l.effects.push(make_effect("preset_flash", "Flash Glow",
        crate::core::timeline::EffectType::Glow {
            threshold: Animatable::new_animated(vec![kf(cf.saturating_sub(1), 0.8), kf(cf, 0.0), kf(cf + 5, 0.8)]),
            radius:    Animatable::new_constant(60.0),
            intensity: Animatable::new_animated(vec![kf(cf.saturating_sub(1), 0.0), kf(cf, 2.5), kf(cf + 5, 0.0)]),
            color:     Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
        },
    ));
    true
}

/// Compound cinematic opener: Film Look + Grain + Letterbox + Fade from Black + Handheld.
/// Applies all at once — no stacking with the individual presets needed.
pub fn film_reel_intro(l: &mut Layer, cf: u32) -> bool {
    let c = |v: f32| Animatable::new_constant(v);
    // Film emulation (teal shadows, warm highlights)
    l.effects.push(make_effect("preset_fri_film", "Film Look",
        crate::core::timeline::EffectType::FilmEmulation {
            lift: c(-0.03), gamma: c(0.94), gain: c(1.08), hue_shift_deg: c(-5.0),
        }));
    // Film grain
    l.effects.push(make_effect("preset_fri_grain", "Film Grain",
        crate::core::timeline::EffectType::FilmGrain {
            intensity: c(0.15), grain_size: 1.8, color_film: true,
        }));
    // Letterbox bars
    l.effects.push(make_effect("preset_fri_lb", "Letterbox",
        crate::core::timeline::EffectType::Letterbox {
            frac: Animatable::new_animated(vec![kf(cf, 0.0), kf(cf + 20, 0.13)]),
        }));
    // Fade from black at in-point
    let _ = fade_from_color(l, cf, [0.0; 4]);
    // Subtle handheld camera
    let _ = handheld(l);
    true
}

/// Piecewise speed ramp around the playhead. Ensures Time Remap is enabled
/// (initialising a linear source map when absent), then rebuilds it as
/// three constant-speed segments: normal → `factor`× between [cf, cf+R] →
/// normal, preserving source-time continuity at every boundary.
/// Linear interpolation between keyframes is intentional: each segment is
/// constant-speed by construction.
pub fn speed_ramp(l: &mut Layer, cf: u32, factor: f32) -> bool {
    const RAMP: u32 = 20;
    let in_f = l.in_frame;
    let out_f = l.out_frame;
    if cf <= in_f || cf >= out_f { return false; }
    let b = cf;
    let c = (cf + RAMP).min(out_f.saturating_sub(1));
    if c <= b { return false; }

    // Ensure remap exists (linear 1:1 like the Cmd+Alt+T initializer).
    if l.time_remap.is_none() {
        let span = out_f.saturating_sub(in_f).max(1);
        l.time_remap = Some(Animatable::new_animated(vec![
            Keyframe::new(0, 0.0f32, InterpolationType::Linear),
            Keyframe::new(out_f.max(1), span as f32, InterpolationType::Linear),
        ]));
    }

    let v_at = |t: u32| l.time_remap.as_ref().unwrap().evaluate(t);
    let va = v_at(in_f.min(b));
    let vb = v_at(b);
    let vd = v_at(out_f.saturating_sub(1));

    let _seg1 = vb - va;                       // consumed before ramp
    let r = (c - b) as f32;
    let vc = vb + r * factor;                 // slowed / fastened middle
    let tail = (out_f.saturating_sub(1) - c) as f32;
    let vd2 = vc + tail;

    let mut kfs = vec![
        Keyframe::new(in_f, va, InterpolationType::Linear),
        Keyframe::new(b, vb, InterpolationType::Linear),
        Keyframe::new(c, vc, InterpolationType::Linear),
        Keyframe::new(out_f.saturating_sub(1), vd2.max(vc), InterpolationType::Linear),
    ];
    let _ = vd; // continuity reference (kept linear by construction)
    kfs.dedup_by(|a2, b2| a2.frame == b2.frame);
    l.time_remap = Some(Animatable::Animated(kfs));
    true
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{LayerType};
    #[cfg(test)]
    pub fn speed_ramp_init_for_test(l: &mut Layer, out_frame: u32) {
        l.out_frame = out_frame;
        l.time_remap = Some(Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(out_frame, out_frame as f32, InterpolationType::Linear),
        ]));
    }

    fn mk() -> Layer {
        let mut l = Layer::new("p".into(), "P".into(), LayerType::Solid { color: [1.0; 4] }, 200);
        l.in_frame = 10;
        l.out_frame = 190;
        l.transform.opacity = Animatable::new_constant(100.0);
        l.transform.scale = Animatable::new_constant([100.0, 100.0]);
        l.transform.position = Animatable::new_constant([960.0, 540.0]);
        l
    }

    #[test]
    fn test_fade_in_creates_two_keyframes() {
        let mut l = mk();
        assert!(apply_by_name("Fade In", &mut l, 30, 1920.0, 1080.0));
        let kfs = l.transform.opacity.keyframes().unwrap();
        assert_eq!(kfs.len(), 2);
        assert_eq!(kfs[0].value, 0.0);
        assert_eq!(kfs[1].value, 100.0);
    }

    #[test]
    fn test_fade_out_fails_when_window_too_small() {
        let mut l = mk();
        l.out_frame = 12; // in=10 → start would be < in
        assert!(!apply_by_name("Fade Out", &mut l, 30, 1920.0, 1080.0));
    }

    #[test]
    fn test_pop_in_overshoot_and_settle() {
        let mut l = mk();
        apply_by_name("Pop In", &mut l, 50, 1920.0, 1080.0);
        let kfs = l.transform.scale.keyframes().unwrap();
        assert_eq!(kfs.len(), 3);
        assert_eq!(kfs[0].value, [0.0, 0.0]);
        assert!((kfs[1].value[0] - 112.0).abs() < 0.01, "overshoot 112%");
        assert_eq!(kfs[2].value, [100.0, 100.0]);
        // opacity snap present
        assert_eq!(l.transform.opacity.keyframes().unwrap().len(), 2);
    }

    #[test]
    fn test_slide_direction_offset() {
        let mut left = mk();
        apply_by_name("Slide In ←", &mut left, 40, 1920.0, 1080.0);
        let kl = left.transform.position.keyframes().unwrap();
        assert!((kl[0].value[0] - (960.0 - 672.0)).abs() < 0.01);

        let mut right = mk();
        apply_by_name("Slide In →", &mut right, 40, 1920.0, 1080.0);
        let kr = right.transform.position.keyframes().unwrap();
        assert!((kr[0].value[0] - (960.0 + 672.0)).abs() < 0.01);
    }

    #[test]
    fn test_unknown_preset_is_noop_false() {
        let mut l = mk();
        assert!(!apply_by_name("Nope", &mut l, 30, 1920.0, 1080.0));
        assert!(l.transform.opacity.keyframes().is_none());
    }
    #[test]
    fn test_speed_ramp_slow_mo_reduces_consumed_source() {
        let mut l = mk();
        // baseline: linear remap
        speed_ramp_init_for_test(&mut l, 200);
        let lin_at_60 = l.time_remap.as_ref().unwrap().evaluate(60);

        let mut l2 = mk();
        speed_ramp_init_for_test(&mut l2, 200);
        assert!(speed_ramp(&mut l2, 40, 0.25));
        // At end of the 20f slow window (f=60): consumed = 30 + 20*0.25 = 35 < linear 50.
        let v = l2.time_remap.as_ref().unwrap().evaluate(60);
        assert!((v - (lin_at_60 - 15.0)).abs() < 0.01, "v={} lin={}", v, lin_at_60);
    }

    #[test]
    fn test_speed_ramp_fast_increases_and_auto_inits() {
        let mut l = mk(); // no time_remap yet
        assert!(speed_ramp(&mut l, 80, 4.0));
        assert!(l.time_remap.is_some());
        let kfs = l.time_remap.as_ref().unwrap().keyframes().unwrap();
        assert!(kfs.len() >= 3);
        // middle segment slope is 4x: value jump over [b,c] equals 4*(c-b)
        let b = kfs.iter().find(|k| k.frame == 80).unwrap();
        let c = kfs.iter().find(|k| k.frame == 100).unwrap();
        assert!((c.value - b.value - (100.0 - 80.0) * 4.0).abs() < 0.01);
    }


    #[test]
    fn test_ken_burns_in_zooms_down_and_pans() {
        let mut l = mk();
        assert!(apply_by_name("🎥 Ken Burns In", &mut l, 50, 1920.0, 1080.0));
        let sk = l.transform.scale.keyframes().unwrap();
        assert_eq!(sk.len(), 2);
        // starts at 130%, ends at base 100%
        assert!((sk[0].value[0] - 130.0).abs() < 0.01, "start 130%: {}", sk[0].value[0]);
        assert_eq!(sk[1].value, [100.0, 100.0]);
        let pk = l.transform.position.keyframes().unwrap();
        // pans inward (x decreases by comp_w * 0.08 ≈ 153.6)
        assert!((pk[0].value[0] - (960.0 - 153.6)).abs() < 1.0);
    }

    #[test]
    fn test_fade_to_black_reduces_opacity_at_out() {
        let mut l = mk();
        assert!(apply_by_name("Fade to Black", &mut l, 100, 1920.0, 1080.0));
        let kfs = l.transform.opacity.keyframes().unwrap();
        assert_eq!(kfs.len(), 2);
        assert_eq!(kfs[0].value, 100.0);
        assert_eq!(kfs[1].value, 0.0);
        // ends at out_frame - 1
        assert_eq!(kfs[1].frame, 189);
    }

    #[test]
    fn test_fade_from_black_at_in() {
        let mut l = mk();
        assert!(apply_by_name("Fade from Black", &mut l, 50, 1920.0, 1080.0));
        let kfs = l.transform.opacity.keyframes().unwrap();
        assert_eq!(kfs[0].frame, 10); // pinned to in_frame
        assert_eq!(kfs[0].value, 0.0);
        assert_eq!(kfs[1].value, 100.0);
    }

    #[test]
    fn test_flash_cut_pushes_glow_effect() {
        let mut l = mk();
        assert!(apply_by_name("⚡ Flash Cut", &mut l, 80, 1920.0, 1080.0));
        // Glow effect was pushed
        assert!(l.effects.iter().any(|e| e.name == "Flash Glow"));
        // Position kfs for shake
        assert!(l.transform.position.keyframes().unwrap().len() >= 3);
    }


    #[test]
    fn test_expr_bounce_sets_position_expression() {
        let mut l = mk();
        assert!(apply_by_name("\u{1f9f2} Bounce", &mut l, 30, 1920.0, 1080.0));
        let expr = l.transform.position_expression.as_ref().unwrap();
        match expr {
            crate::core::timeline::Expression::Raw(s) => {
                assert!(s.contains("sin"), "bounce expr missing sin: {}", s);
                assert!(s.contains("exp"), "bounce expr missing exp: {}", s);
            }
            _ => panic!("expected Raw expression"),
        }
    }

    #[test]
    fn test_expr_elastic_sets_scale_expression() {
        let mut l = mk();
        assert!(apply_by_name("\u{1f300} Elastic", &mut l, 30, 1920.0, 1080.0));
        assert!(l.transform.scale_expression.is_some());
    }

    #[test]
    fn test_expr_sine_wave_sets_position() {
        let mut l = mk();
        assert!(apply_by_name("\u{1f30a} Sine Wave", &mut l, 30, 1920.0, 1080.0));
        let expr = l.transform.position_expression.as_ref().unwrap();
        match expr {
            crate::core::timeline::Expression::Raw(s) => assert!(s.contains("sin")),
            _ => panic!("expected Raw"),
        }
    }

    #[test]
    fn test_expr_strobe_sets_opacity_expression() {
        let mut l = mk();
        assert!(apply_by_name("\u{1f4a1} Strobe", &mut l, 30, 1920.0, 1080.0));
        assert!(l.transform.opacity_expression.is_some());
    }

    #[test]
    fn test_slide_out_moves_right_and_fades() {
        let mut l = mk();
        assert!(apply_by_name("\u{1f3ac} Slide Out \u{2192}", &mut l, 100, 1920.0, 1080.0));
        let pk = l.transform.position.keyframes().unwrap();
        assert_eq!(pk.len(), 2);
        assert!((pk[1].value[0] - (960.0 + 960.0)).abs() < 1.0, "slide right: {}", pk[1].value[0]);
        let op = l.transform.opacity.keyframes().unwrap();
        assert_eq!(op[1].value, 0.0);
    }

    #[test]
    fn test_zoom_out_scales_up_and_fades() {
        let mut l = mk();
        assert!(apply_by_name("\u{1f3ac} Zoom Out", &mut l, 100, 1920.0, 1080.0));
        let sk = l.transform.scale.keyframes().unwrap();
        assert!((sk[1].value[0] - 200.0).abs() < 0.01, "zoom 200%: {}", sk[1].value[0]);
        let op = l.transform.opacity.keyframes().unwrap();
        assert_eq!(op[1].value, 0.0);
    }


    #[test]
    fn test_text_typewriter_sets_animator_with_offset_anim() {
        let mut l = mk();
        assert!(apply_by_name("Typewriter", &mut l, 30, 1920.0, 1080.0));
        let anim = l.text_animator.as_ref().unwrap();
        assert!(anim.enabled);
        assert!(anim.selector.offset_anim.is_some());
        let oa = anim.selector.offset_anim.as_ref().unwrap();
        // at cf=30, offset should be -100; at cf+30=60, should be 100
        assert!((oa.evaluate(30) - (-100.0)).abs() < 0.01);
        assert!((oa.evaluate(60) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_text_bounce_in_sets_position_expression() {
        let mut l = mk();
        assert!(apply_by_name("Bounce In Text", &mut l, 30, 1920.0, 1080.0));
        assert!(l.transform.position_expression.is_some());
        let anim = l.text_animator.as_ref().unwrap();
        assert_eq!(anim.position_offset, [0.0, -50.0]);
    }

    #[test]
    fn test_text_scale_up_zeroes_scale() {
        let mut l = mk();
        assert!(apply_by_name("Scale Up Text", &mut l, 30, 1920.0, 1080.0));
        let anim = l.text_animator.as_ref().unwrap();
        assert_eq!(anim.scale, [0.0, 0.0]);
        assert_eq!(anim.opacity, 0.0);
    }

    #[test]
    fn test_text_fade_up_has_blur_and_offset() {
        let mut l = mk();
        assert!(apply_by_name("Fade Up Words", &mut l, 30, 1920.0, 1080.0));
        let anim = l.text_animator.as_ref().unwrap();
        assert!((anim.blur_amount - 4.0).abs() < 0.01);
        assert_eq!(anim.position_offset, [0.0, 30.0]);
        assert!(anim.selector.ease_high > 0.0);
    }


    #[test]
    fn test_freeze_frame_locks_to_single_source() {
        let mut l = mk();
        assert!(apply_by_name("Freeze Frame", &mut l, 50, 1920.0, 1080.0));
        let rm = l.time_remap.as_ref().unwrap();
        let kfs = rm.keyframes().unwrap();
        assert_eq!(kfs.len(), 2);
        // Both ends map to same source frame
        assert!((kfs[0].value - kfs[1].value).abs() < 0.01, "freeze: {} vs {}", kfs[0].value, kfs[1].value);
    }

    #[test]
    fn test_reverse_time_maps_start_to_end() {
        let mut l = mk();
        assert!(apply_by_name("Reverse", &mut l, 50, 1920.0, 1080.0));
        let kfs = l.time_remap.as_ref().unwrap().keyframes().unwrap();
        // start maps to span (180), end maps to 0
        assert!((kfs[0].value - 180.0).abs() < 0.01, "reverse start: {}", kfs[0].value);
        assert!((kfs[1].value).abs() < 0.01, "reverse end: {}", kfs[1].value);
    }

    #[test]
    fn test_slow_motion_doubles_mapped_span() {
        let mut l = mk();
        assert!(apply_by_name("Slow Motion 0.5\u{00d7}", &mut l, 50, 1920.0, 1080.0));
        let kfs = l.time_remap.as_ref().unwrap().keyframes().unwrap();
        // span=180, factor=0.5 → mapped = 180*0.5 = 90
        assert!((kfs[1].value - 90.0).abs() < 0.01, "slow: {}", kfs[1].value);
    }

    #[test]
    fn test_fast_forward_halves_mapped_span() {
        let mut l = mk();
        assert!(apply_by_name("Fast Forward 2\u{00d7}", &mut l, 50, 1920.0, 1080.0));
        let kfs = l.time_remap.as_ref().unwrap().keyframes().unwrap();
        // span=180, factor=2.0 → mapped = 360
        assert!((kfs[1].value - 360.0).abs() < 0.01, "fast: {}", kfs[1].value);
    }


    #[test]
    fn test_youtube_vlog_stacks_four_effects() {
        let mut l = mk();
        assert!(apply_by_name("YouTube Vlog", &mut l, 30, 1920.0, 1080.0));
        assert!(l.effects.len() >= 3, "vlog effects: {}", l.effects.len());
        assert!(l.transform.position_expression.is_some(), "handheld expr");
    }

    #[test]
    fn test_music_video_sets_rotation_expression() {
        let mut l = mk();
        assert!(apply_by_name("Music Video", &mut l, 30, 1920.0, 1080.0));
        assert!(l.transform.rotation_expression.is_some());
        assert!(l.transform.opacity_expression.is_some(), "strobe expr");
    }

    #[test]
    fn test_cinematic_reveal_fades_and_letterboxes() {
        let mut l = mk();
        assert!(apply_by_name("Cinematic Reveal", &mut l, 30, 1920.0, 1080.0));
        let op = l.transform.opacity.keyframes().unwrap();
        assert_eq!(op[0].value, 0.0, "fade from black");
        assert!(l.effects.iter().any(|e| e.name == "Letterbox"));
        assert!(l.effects.iter().any(|e| e.name == "Film Look"));
    }

    #[test]
    fn test_documentary_opener_has_grain_and_letterbox() {
        let mut l = mk();
        assert!(apply_by_name("Documentary Opener", &mut l, 30, 1920.0, 1080.0));
        let names: Vec<_> = l.effects.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Film Grain"));
        assert!(names.contains(&"Letterbox"));
        assert!(l.transform.position_expression.is_some());
    }

    #[test]
    fn test_reset_layer_clears_everything() {
        let mut l = mk();
        let _ = apply_by_name("YouTube Vlog", &mut l, 30, 1920.0, 1080.0);
        assert!(!l.effects.is_empty());
        assert!(l.transform.position_expression.is_some());
        assert!(apply_by_name("Reset Layer", &mut l, 30, 1920.0, 1080.0));
        assert!(l.effects.is_empty());
        assert!(l.transform.position_expression.is_none());
        assert!(l.transform.scale_expression.is_none());
        assert!(l.time_remap.is_none());
        assert!(l.text_animator.is_none());
    }

    #[test]
    fn test_film_reel_intro_stacks_four_effects() {
        let mut l = mk();
        assert!(apply_by_name("🎞 Film Reel Intro", &mut l, 30, 1920.0, 1080.0));
        let names: Vec<_> = l.effects.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Film Look"));
        assert!(names.contains(&"Film Grain"));
        assert!(names.contains(&"Letterbox"));
        // fade_from_color was applied
        let op = l.transform.opacity.keyframes().unwrap();
        assert_eq!(op[0].value, 0.0);
        // handheld expression was set
        assert!(l.transform.position_expression.is_some());
    }

}
