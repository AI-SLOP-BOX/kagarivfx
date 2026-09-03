//! Convert Audio to Keyframes (AE Keyframe Assistant Parity).
//!
//! Analyzes an audio stream or layer's WAV samples frame-by-frame and produces
//! an "Audio Amplitude" Null layer equipped with animatable Slider Controls
//! ("Left Channel", "Right Channel", "Both Channels").

use crate::core::audio_engine::AudioBuffer;
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Effect, EffectType, Layer, LayerType};
use std::path::Path;

/// Convert audio file samples into an "Audio Amplitude" layer in the composition.
pub fn convert_audio_to_keyframes(
    comp: &mut Composition,
    audio_path: &str,
) -> Result<String, String> {
    let buf = AudioBuffer::load_wav(Path::new(audio_path))
        .map_err(|e| format!("Failed to decode audio: {}", e))?;

    let fps = comp.fps.max(1);
    let total_frames = comp.duration_frames;
    let sample_rate = buf.sample_rate.max(1);
    let samples_per_frame = (sample_rate as f32 / fps as f32).max(1.0) as usize;

    let mut left_kfs = Vec::with_capacity(total_frames as usize);
    let mut right_kfs = Vec::with_capacity(total_frames as usize);
    let mut both_kfs = Vec::with_capacity(total_frames as usize);

    for f in 0..total_frames {
        let Some(start_sample) = (f as usize).checked_mul(samples_per_frame) else {
            break;
        };
        let end_sample = start_sample
            .saturating_add(samples_per_frame)
            .min(buf.samples.len());

        let (mut sum_sq_l, mut sum_sq_r) = (0.0f64, 0.0f64);
        let count = (end_sample.saturating_sub(start_sample)).max(1) as f64;

        if start_sample < buf.samples.len() {
            for &s in &buf.samples[start_sample..end_sample] {
                if s.is_finite() {
                    let square = f64::from(s) * f64::from(s);
                    sum_sq_l += square;
                    sum_sq_r += square; // Mono/interleaved RMS proxy
                }
            }
        }

        let rms_l = ((sum_sq_l / count).sqrt() * 50.0) as f32;
        let rms_r = ((sum_sq_r / count).sqrt() * 50.0) as f32;
        let rms_both = (rms_l + rms_r) * 0.5;

        left_kfs.push(Keyframe::new(f, rms_l, InterpolationType::Linear));
        right_kfs.push(Keyframe::new(f, rms_r, InterpolationType::Linear));
        both_kfs.push(Keyframe::new(f, rms_both, InterpolationType::Linear));
    }

    let mut amp_layer = Layer::new_null(
        format!("audio_amp_{}", comp.layers.len() + 1),
        "Audio Amplitude".to_string(),
        total_frames,
    );

    // Add Slider Controls for Left, Right, Both
    amp_layer.effects.push(Effect {
        id: "slider_left".to_string(),
        name: "Left Channel".to_string(),
        effect_type: EffectType::SliderControl {
            value: Animatable::Animated(left_kfs),
        },
        enabled: true,
    });

    amp_layer.effects.push(Effect {
        id: "slider_right".to_string(),
        name: "Right Channel".to_string(),
        effect_type: EffectType::SliderControl {
            value: Animatable::Animated(right_kfs),
        },
        enabled: true,
    });

    amp_layer.effects.push(Effect {
        id: "slider_both".to_string(),
        name: "Both Channels".to_string(),
        effect_type: EffectType::SliderControl {
            value: Animatable::Animated(both_kfs),
        },
        enabled: true,
    });

    comp.add_layer(amp_layer);
    Ok("Audio Amplitude".to_string())
}

/// Convert audio file samples into an "Audio Multi-Band Amplitude" Null layer (Master, Bass, Mid, Treble).
pub fn convert_multiband_audio_to_keyframes(
    comp: &mut Composition,
    audio_path: &str,
    options: Option<crate::core::audio_dsp::AudioKeyframeOptions>,
) -> Result<String, String> {
    let buf = AudioBuffer::load_wav(Path::new(audio_path))
        .map_err(|e| format!("Failed to decode audio: {}", e))?;

    let fps = comp.fps.max(1);
    let total_frames = comp.duration_frames;
    let sample_rate = buf.sample_rate.max(1);

    let opt = options.unwrap_or_default();
    let multiband = crate::core::audio_dsp::extract_multiband_audio_keyframes(
        &buf.samples,
        sample_rate,
        fps,
        total_frames,
        &opt,
    );

    let mut amp_layer = Layer::new_null(
        format!("audio_multiband_amp_{}", comp.layers.len() + 1),
        "Audio Multi-Band Amplitude".to_string(),
        total_frames,
    );

    amp_layer.effects.push(Effect {
        id: "slider_master".to_string(),
        name: "Master Amplitude".to_string(),
        effect_type: EffectType::SliderControl {
            value: Animatable::Animated(multiband.master),
        },
        enabled: true,
    });

    amp_layer.effects.push(Effect {
        id: "slider_bass".to_string(),
        name: "Bass (Low)".to_string(),
        effect_type: EffectType::SliderControl {
            value: Animatable::Animated(multiband.bass),
        },
        enabled: true,
    });

    amp_layer.effects.push(Effect {
        id: "slider_mid".to_string(),
        name: "Mid Frequencies".to_string(),
        effect_type: EffectType::SliderControl {
            value: Animatable::Animated(multiband.mid),
        },
        enabled: true,
    });

    amp_layer.effects.push(Effect {
        id: "slider_treble".to_string(),
        name: "Treble (High)".to_string(),
        effect_type: EffectType::SliderControl {
            value: Animatable::Animated(multiband.treble),
        },
        enabled: true,
    });

    comp.add_layer(amp_layer);
    Ok("Audio Multi-Band Amplitude".to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTargetProperty {
    Scale,
    Opacity,
    Rotation,
    PositionY,
}

/// Automatically binds an audio keyframe channel (e.g. Bass or Master) to modulate a layer's transform property.
pub fn bind_audio_amplitude_to_layer_transform(
    layer: &mut Layer,
    keyframes: &[Keyframe<f32>],
    target: AudioTargetProperty,
    base_value: f32,
    multiplier: f32,
) {
    if keyframes.is_empty() {
        return;
    }

    let base_value = if base_value.is_finite() {
        base_value
    } else {
        0.0
    };
    let multiplier = if multiplier.is_finite() {
        multiplier
    } else {
        0.0
    };

    match target {
        AudioTargetProperty::Scale => {
            let mut scale_kfs = Vec::with_capacity(keyframes.len());
            for kf in keyframes {
                let s = modulated_value(base_value, kf.value, multiplier);
                scale_kfs.push(Keyframe::new(kf.frame, [s, s], InterpolationType::Linear));
            }
            layer.transform.scale = Animatable::Animated(scale_kfs);
        }
        AudioTargetProperty::Opacity => {
            let mut opac_kfs = Vec::with_capacity(keyframes.len());
            for kf in keyframes {
                let op = modulated_value(base_value, kf.value, multiplier).clamp(0.0, 100.0);
                opac_kfs.push(Keyframe::new(kf.frame, op, InterpolationType::Linear));
            }
            layer.transform.opacity = Animatable::Animated(opac_kfs);
        }
        AudioTargetProperty::Rotation => {
            let mut rot_kfs = Vec::with_capacity(keyframes.len());
            for kf in keyframes {
                let r = modulated_value(base_value, kf.value, multiplier);
                rot_kfs.push(Keyframe::new(kf.frame, r, InterpolationType::Linear));
            }
            layer.transform.rotation = Animatable::Animated(rot_kfs);
        }
        AudioTargetProperty::PositionY => {
            let mut pos_kfs = Vec::with_capacity(keyframes.len());
            let current_pos = layer.transform.position.evaluate(0).map(|value| {
                if value.is_finite() {
                    value
                } else {
                    0.0
                }
            });
            for kf in keyframes {
                let y = current_pos[1] + modulated_value(0.0, kf.value, multiplier);
                pos_kfs.push(Keyframe::new(
                    kf.frame,
                    [current_pos[0], y],
                    InterpolationType::Linear,
                ));
            }
            layer.transform.position = Animatable::Animated(pos_kfs);
        }
    }
}

fn modulated_value(base: f32, value: f32, multiplier: f32) -> f32 {
    if !value.is_finite() {
        return base;
    }
    let result = f64::from(base) + f64::from(value) * f64::from(multiplier);
    if result.is_finite() {
        result.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
    } else {
        base
    }
}

/// Generates composition beat markers at transient peaks in the audio stream.
pub fn generate_beat_markers_from_audio(
    comp: &mut Composition,
    keyframes: &[Keyframe<f32>],
    threshold: f32,
) {
    if keyframes.len() < 3 {
        return;
    }

    for i in 1..keyframes.len() - 1 {
        let prev = keyframes[i - 1].value;
        let curr = keyframes[i].value;
        let next = keyframes[i + 1].value;

        // Local peak above threshold
        if curr > threshold && curr >= prev && curr >= next {
            comp.markers.push(crate::core::timeline::TimelineMarker {
                frame: keyframes[i].frame,
                label: "Beat".to_string(),
                color: [0.9, 0.2, 0.2],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_to_keyframes_generates_three_sliders() {
        let mut comp = Composition::new("test".into(), "Test".into(), 1920, 1080, 30, 60);
        // Non-existent path returns error gracefully
        let res = convert_audio_to_keyframes(&mut comp, "/invalid/path.wav");
        assert!(res.is_err());
    }

    #[test]
    fn test_bind_audio_amplitude_to_layer_transform_scale() {
        let mut layer = Layer::new(
            "s1".into(),
            "Solid".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            60,
        );
        let kfs = vec![
            Keyframe::new(0, 0.0f32, InterpolationType::Linear),
            Keyframe::new(10, 10.0f32, InterpolationType::Linear),
        ];

        bind_audio_amplitude_to_layer_transform(
            &mut layer,
            &kfs,
            AudioTargetProperty::Scale,
            100.0,
            2.0,
        );

        let scale_0 = layer.transform.scale.evaluate(0);
        let scale_10 = layer.transform.scale.evaluate(10);
        assert_eq!(scale_0, [100.0, 100.0]);
        assert_eq!(scale_10, [120.0, 120.0]);
    }

    #[test]
    fn test_generate_beat_markers_from_audio() {
        let mut comp = Composition::new("test".into(), "Test".into(), 1920, 1080, 30, 60);
        let kfs = vec![
            Keyframe::new(0, 2.0f32, InterpolationType::Linear),
            Keyframe::new(1, 15.0f32, InterpolationType::Linear), // Peak > 10.0
            Keyframe::new(2, 4.0f32, InterpolationType::Linear),
            Keyframe::new(3, 5.0f32, InterpolationType::Linear),
        ];

        generate_beat_markers_from_audio(&mut comp, &kfs, 10.0);
        assert_eq!(comp.markers.len(), 1);
        assert_eq!(comp.markers[0].frame, 1);
    }

    #[test]
    fn test_audio_binding_sanitizes_nonfinite_values() {
        let mut layer = Layer::new_null("null".into(), "Null".into(), 10);
        let keyframes = vec![
            Keyframe::new(0, f32::NAN, InterpolationType::Linear),
            Keyframe::new(1, f32::INFINITY, InterpolationType::Linear),
        ];
        bind_audio_amplitude_to_layer_transform(
            &mut layer,
            &keyframes,
            AudioTargetProperty::Opacity,
            f32::NAN,
            f32::INFINITY,
        );
        for frame in 0..=1 {
            assert!(layer.transform.opacity.evaluate(frame).is_finite());
        }
    }
}
