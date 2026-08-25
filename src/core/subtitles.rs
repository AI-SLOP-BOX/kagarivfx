//! Subtitle interchange: parse SRT / WebVTT caption files and convert them
//! to timed Text layers (and back). NLEs like Kdenlive/Shotcut own the
//! speech-to-text step (Whisper etc.); this app consumes/produces the
//! caption interchange formats so graphics stay in our lane.

use crate::core::property::Animatable;
use crate::core::timeline::{Layer, LayerType};

#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleCue {
    pub start_frame: u32,
    pub end_frame: u32,
    pub text: String,
}

/// Parse `HH:MM:SS,mmm` or `HH:MM:SS.mmm` into seconds.
fn parse_timecode(s: &str) -> Option<f64> {
    let s = s.trim();
    let (h, rest) = s.split_once(':')?;
    let (m, rest) = rest.split_once(':')?;
    let sec_part = rest.replace(',', ".");
    let secs: f64 = sec_part.parse().ok()?;
    Some(h.parse::<f64>().ok()? * 3600.0 + m.parse::<f64>().ok()? * 60.0 + secs)
}

fn tc_to_frame(tc: &str, fps: f32) -> Option<u32> {
    Some((parse_timecode(tc)? * fps as f64).round() as u32)
}

/// Parse SRT or WebVTT cue blocks. Both share `start --> end` timing.
pub fn parse_srt(input: &str, fps: u32) -> Vec<SubtitleCue> {
    let fps = fps.max(1) as f32;
    let mut cues = Vec::new();
    let mut block: Vec<String> = Vec::new();

    let flush = |block: &mut Vec<String>, cues: &mut Vec<SubtitleCue>| {
        if block.len() < 2 {
            block.clear();
            return;
        }
        // Find the timecode line regardless of leading index line.
        let t_idx = block.iter().position(|l| l.contains("-->"));
        if let Some(t_idx) = t_idx {
            let tc = &block[t_idx];
            if let Some((a, b)) = tc.split_once("-->") {
                if let (Some(sf), Some(ef)) = (tc_to_frame(a, fps), tc_to_frame(b, fps)) {
                    let text: Vec<&str> = block[t_idx + 1..].iter().map(|s| s.as_str()).collect();
                    let text = text.join("\n").trim().to_string();
                    if !text.is_empty() && ef > sf {
                        cues.push(SubtitleCue { start_frame: sf, end_frame: ef, text });
                    }
                }
            }
        }
        block.clear();
    };

    for line in input.lines() {
        let l = line.trim_end();
        if l.trim().is_empty() || l.trim() == "WEBVTT" || l.starts_with("NOTE ") {
            flush(&mut block, &mut cues);
        } else {
            block.push(l.to_string());
        }
    }
    flush(&mut block, &mut cues);
    cues
}

/// Convert cues into styled caption Text layers (bottom-center).
/// Returns layers ready to push onto a composition.
pub fn cues_to_layers(
    cues: &[SubtitleCue],
    comp_w: f32,
    comp_h: f32,
    font_size: u32,
) -> Vec<Layer> {
    cues.iter().enumerate().map(|(i, cue)| {
        let first_line = cue.text.lines().next().unwrap_or("").chars().take(24).collect::<String>();
        let name = format!("Caption {:03} · {}", i + 1, first_line);
        let mut layer = Layer::new(
            format!("caption_{}", i + 1),
            name,
            LayerType::new_text(cue.text.clone(), font_size, [1.0, 1.0, 1.0, 1.0]),
            1,
        );
        // Bottom-center placement + readable outline
        layer.transform.position = Animatable::Constant([comp_w / 2.0, comp_h * 0.86]);
        if let Some(tf) = &mut layer.text_formatting {
            tf.alignment = 1; // center
            tf.stroke_color = Some([0.0, 0.0, 0.0, 1.0]);
            tf.stroke_width = font_size as f32 * 0.08;
        }
        layer.in_frame = cue.start_frame.min(cue.end_frame.saturating_sub(1));
        layer.out_frame = cue.end_frame.max(layer.in_frame + 1);
        layer
    }).collect()
}

/// Serialize caption-prefixed text layers back out as SRT.
pub fn layers_to_srt(layers: &[Layer], fps: u32) -> String {
    let fps = fps.max(1) as f32;
    fn fmt_tc(frames: u32, fps: f32) -> String {
        let total_secs = frames as f64 / fps as f64;
        let h = (total_secs / 3600.0).floor() as u32;
        let m = ((total_secs / 60.0).floor() as u32) % 60;
        let s = (total_secs.floor() as u32) % 60;
        let ms = ((total_secs - total_secs.floor()) * 1000.0).round() as u32;
        format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
    }

    let mut cues: Vec<(u32, u32, &str)> = Vec::new();
    for l in layers {
        if !l.name.starts_with("Caption") { continue; }
        if let LayerType::Text { text, .. } = &l.layer_type {
            if l.out_frame > l.in_frame {
                cues.push((l.in_frame, l.out_frame, text.as_str()));
            }
        }
    }
    cues.sort_by_key(|c| c.0);

    let mut out = String::new();
    for (i, (sf, ef, text)) in cues.iter().enumerate() {
        out.push_str(&format!("{}\n{} --> {}\n{}\n\n", i + 1, fmt_tc(*sf, fps), fmt_tc(*ef, fps), text));
    }
    out
}


#[cfg(test)]
mod tests {
    use super::*;

    const SRT: &str = "1\n00:00:01,000 --> 00:00:03,500\nHello world\n\n2\n00:00:04,000 --> 00:00:06,000\nSecond line\ncaption text\n";

    #[test]
    fn test_parse_srt_basic() {
        let cues = parse_srt(SRT, 30);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start_frame, 30);
        assert_eq!(cues[0].end_frame, 105);
        assert_eq!(cues[0].text, "Hello world");
    }

    #[test]
    fn test_parse_vtt_with_header() {
        let vtt = "WEBVTT\n\n00:00:00.500 --> 00:00:02.000\nHi\n";
        let cues = parse_srt(vtt, 60);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].start_frame, 30);
        assert_eq!(cues[0].text, "Hi");
    }

    #[test]
    fn test_cues_to_layers_timing_and_style() {
        let cues = parse_srt(SRT, 30);
        let layers = cues_to_layers(&cues, 1920.0, 1080.0, 48);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].in_frame, 30);
        assert_eq!(layers[0].out_frame, 105);
        assert!(layers[0].name.starts_with("Caption 001"));
        if let Some(tf) = &layers[0].text_formatting {
            assert_eq!(tf.alignment, 1);
            assert!(tf.stroke_width > 0.0);
        }
        if let LayerType::Text { text, .. } = &layers[1].layer_type {
            assert!(text.contains('\n'));
        }
    }

    #[test]
    fn test_round_trip_layers_to_srt() {
        let cues = parse_srt(SRT, 30);
        let layers = cues_to_layers(&cues, 1920.0, 1080.0, 48);
        let srt = layers_to_srt(&layers, 30);
        let reparsed = parse_srt(&srt, 30);
        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].text, "Hello world");
        assert_eq!(reparsed[1].end_frame, 180);
    }
}
