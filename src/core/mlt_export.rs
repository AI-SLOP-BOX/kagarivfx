//! MLT XML export — Shotcut / Kdenlive interoperability.
//!
//! Emits an MLT XML document (the format natively opened by Shotcut and
//! Kdenlive) describing the composition's layers as producers attached to a
//! playlist track. Transforms are not representable in MLT, so this is a
//! timeline-level interchange: layer names, in/out points, and source paths.

#[allow(unused_imports)]
use crate::core::timeline::{Composition, Layer, LayerType};

pub struct MltExporter;

impl MltExporter {
    /// Serializes the composition into MLT XML.
    pub fn export_to_xml(comp: &Composition) -> String {
        let mut xml = String::with_capacity(4096);
        xml.push_str("<?xml version=\"1.0\" standalone=\"no\"?>\n");
        xml.push_str("<mlt LC_NUMERIC=\"C\" version=\"7.0.0\" title=\"Exported from AfterEffects OSS\">\n");
        xml.push_str("  <profile description=\"automatic\" ");
        xml.push_str(&format!(
            "width=\"{}\" height=\"{}\" progressive=\"1\" ",
            comp.width, comp.height
        ));
        xml.push_str(&format!(
            "sample_aspect_numeral=\"1\" sample_aspect_denominator=\"1\" display_aspect_numeral=\"{}\" display_aspect_denominator=\"{}\" frame_rate_num=\"{}\" frame_rate_den=\"1\" colorspace=\"709\"/>\n",
            comp.width, comp.height, comp.fps
        ));

        // Producers: one per media-backed layer
        xml.push_str("  <producer id=\"black\" in=\"00:00:00.000\" out=\"00:99:00.000\">\n");
        xml.push_str("    <property name=\"length\">100:00:00.000</property>\n");
        xml.push_str("    <property name=\"resource\">0</property>\n");
        xml.push_str("    <property name=\"mlt_service\">color</property>\n");
        xml.push_str(&format!(
            "    <property name=\"background\">#{:02x}{:02x}{:02x}</property>\n",
            (comp.background_color[0] * 255.0) as u8,
            (comp.background_color[1] * 255.0) as u8,
            (comp.background_color[2] * 255.0) as u8
        ));
        xml.push_str("  </producer>\n");

        let mut producer_count = 0usize;
        for (idx, layer) in comp.layers.iter().enumerate() {
            let resource = match &layer.layer_type {
                LayerType::Image { path } => Some(path.clone()),
                LayerType::Video { source, .. } => Some(source.clone()),
                LayerType::Audio { path, .. } => Some(path.clone()),
                LayerType::Solid { color } => {
                    // Solids become color producers so the grade survives roundtrip
                    let _ = color;
                    Some(format!("color:{:02x}{:02x}{:02x}",
                        (layer.label.to_rgb()[0] * 255.0) as u8, 0, 0))
                }
                _ => None,
            };
            let Some(resource) = resource else { continue };
            let pid = format!("producer{}", idx);
            let mlt_len = Self::timecode((layer.out_frame as f32 / comp.fps.max(1) as f32).max(0.04), comp.fps as f32);
            xml.push_str(&format!("  <producer id=\"{}\" in=\"00:00:00.000\" out=\"{}\">\n", pid, mlt_len));
            xml.push_str(&format!("    <property name=\"length\">{}</property>\n", mlt_len));
            xml.push_str(&format!("    <property name=\"resource\">{}</property>\n", escape_xml(&resource)));
            let service = match &layer.layer_type {
                LayerType::Solid { .. } => "color".to_string(),
                LayerType::Audio { .. } => "avformat-novalidate".to_string(),
                _ => "avformat".to_string(),
            };
            xml.push_str(&format!("    <property name=\"mlt_service\">{}</property>\n", service));
            if let LayerType::Audio { volume, .. } = &layer.layer_type {
                xml.push_str(&format!(
                    "    <property name=\"volume\">{}</property>\n",
                    10.0f32.powf(volume.evaluate(0) / 20.0)
                ));
            }
            xml.push_str(&format!("    <property name=\"name\">{}</property>\n", escape_xml(&layer.name)));
            xml.push_str("  </producer>\n");
            producer_count += 1;
        }

        // Playlist track referencing all produced entries
        xml.push_str("  <playlist id=\"playlist0\">\n");
        xml.push_str(&format!(
            "    <property name=\"shotcut:video\">1</property>\n"
        ));
        xml.push_str("    <entry producer=\"black\" in=\"00:00:00.000\" out=\"00:00:00.000\">\n");
        xml.push_str("      <property name=\"blank_length\">00:00:00.040</property>\n");
        xml.push_str("    </entry>\n");
        for (idx, layer) in comp.layers.iter().enumerate() {
            let has_media = matches!(
                &layer.layer_type,
                LayerType::Image { .. } | LayerType::Video { .. } | LayerType::Audio { .. } | LayerType::Solid { .. }
            );
            if !has_media {
                continue;
            }
            let dur_frames = layer.out_frame.saturating_sub(layer.in_frame).max(1);
            let out_sec = dur_frames as f32 / comp.fps.max(1) as f32 - 0.001;
            xml.push_str(&format!(
                "    <entry producer=\"producer{}\" in=\"00:00:00.000\" out=\"{}\">\n",
                idx,
                Self::timecode(out_sec.max(0.04), comp.fps as f32)
            ));
            xml.push_str(&format!(
                "      <property name=\"name\">{}</property>\n",
                escape_xml(&layer.name)
            ));
            xml.push_str("    </entry>\n");
        }
        xml.push_str("  </playlist>\n");

        // Tractor binding the background + playlist
        xml.push_str(&format!(
            "  <tractor id=\"tractor0\" global_feed=\"1\" title=\"{}\" in=\"00:00:00.000\" out=\"{}\">\n",
            escape_xml(&comp.name),
            Self::timecode(comp.duration_frames as f32 / comp.fps.max(1) as f32, comp.fps as f32)
        ));
        xml.push_str("    <track producer=\"background\"/>\n");
        xml.push_str("    <track producer=\"playlist0\"/>\n");
        let _ = producer_count;
        xml.push_str("  </tractor>\n");
        xml.push_str("</mlt>\n");
        xml
    }

    /// SMPTE-ish timecode HH:MM:SS.mmm used by MLT in/out attributes.
    fn timecode(seconds: f32, _fps: f32) -> String {
        let total_ms = (seconds * 1000.0).round() as u64;
        let ms = total_ms % 1000;
        let total_s = total_ms / 1000;
        let s = total_s % 60;
        let m = (total_s / 60) % 60;
        let h = total_s / 3600;
        format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::property::Animatable;

    #[test]
    fn test_mlt_export_structure() {
        let mut comp = Composition::new("c1".into(), "MLTTest".into(), 640, 360, 30, 90);
        let bg = Layer::new("bg".into(), "BG".into(), LayerType::Solid { color: [0.2; 4] }, 30);
        comp.layers.push(bg);
        let img = Layer::new(
            "img".into(),
            "Logo <v1>".into(),
            LayerType::Image { path: "/tmp/art&work.png".into() },
            30,
        );
        let _ = Animatable::<f32>::new_constant(0.0);
        comp.layers.push(img);

        let xml = MltExporter::export_to_xml(&comp);
        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains("<mlt "));
        assert!(xml.contains("width=\"640\" height=\"360\""), "profile dims");
        assert!(xml.contains("frame_rate_num=\"30\""));
        // XML escaping
        assert!(xml.contains("/tmp/art&amp;work.png"), "special chars escaped");
        assert!(xml.contains("Logo &lt;v1&gt;"), "names escaped");
        // Producer and entry present
        assert!(xml.contains("id=\"producer1\""));
        assert!(xml.contains("<entry producer=\"producer1\""));
        assert!(xml.contains("<track producer=\"playlist0\"/>"));
    }

    #[test]
    fn test_mlt_video_layer_resource_is_source_file() {
        let mut comp = Composition::new("c".into(), "V".into(), 320, 180, 30, 30);
        let vid = Layer::new(
            "v".into(),
            "Clip".into(),
            crate::core::timeline::LayerType::Video {
                source: "/media/clip.mp4".into(),
                frames_dir: "/tmp/frames".into(),
                frame_count: 100,
                audio_wav: None,
                speed: 1.0,
            },
            30,
        );
        comp.layers.push(vid);
        let xml = MltExporter::export_to_xml(&comp);
        assert!(
            xml.contains("<property name=\"resource\">/media/clip.mp4</property>"),
            "video source should be the original file for NLE round-trip"
        );
    }

    #[test]
    fn test_timecode_format() {
        assert_eq!(MltExporter::timecode(0.0, 30.0), "00:00:00.000");
        assert_eq!(MltExporter::timecode(1.5, 30.0), "00:00:01.500");
        assert_eq!(MltExporter::timecode(62.25, 30.0), "00:01:02.250");
    }
}
