//! Essential Graphics & Motion Graphics Template (MOGRT) Engine (AE Parity).
//!
//! Provides the complete data structure, serialization, and override binding system
//! for Essential Properties and .mogrt template packages used in video editors (Premiere Pro / NLEs).

#![allow(dead_code)]

use crate::core::timeline::{Composition, Layer};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EssentialPropertyType {
    Number {
        min: f32,
        max: f32,
        value: f32,
    },
    Color {
        value: [f32; 4],
    },
    Text {
        text: String,
    },
    Checkbox {
        value: bool,
    },
    Dropdown {
        options: Vec<String>,
        selected_index: usize,
    },
    Point2D {
        value: [f32; 2],
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EssentialProperty {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub target_layer_id: String,
    pub target_property_path: String,
    pub property_type: EssentialPropertyType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MogrtManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub comp_id: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_frames: u32,
    pub essential_properties: Vec<EssentialProperty>,
}

/// Binds user override values from Essential Properties directly to the composition's layers.
pub fn apply_essential_property_overrides(
    comp: &mut Composition,
    properties: &[EssentialProperty],
) {
    for prop in properties {
        if let Some(layer) = comp
            .layers
            .iter_mut()
            .find(|l| l.id == prop.target_layer_id)
        {
            match &prop.property_type {
                EssentialPropertyType::Number { value, .. } => {
                    if prop.target_property_path == "transform.opacity" {
                        layer.transform.opacity =
                            crate::core::property::Animatable::new_constant(*value);
                    } else if prop.target_property_path == "transform.rotation" {
                        layer.transform.rotation =
                            crate::core::property::Animatable::new_constant(*value);
                    }
                }
                EssentialPropertyType::Point2D { value } => {
                    if prop.target_property_path == "transform.position" {
                        layer.transform.position =
                            crate::core::property::Animatable::new_constant(*value);
                    } else if prop.target_property_path == "transform.scale" {
                        layer.transform.scale =
                            crate::core::property::Animatable::new_constant(*value);
                    }
                }
                EssentialPropertyType::Text { text } => {
                    if let crate::core::timeline::LayerType::Text {
                        text: ref mut layer_text,
                        ..
                    } = layer.layer_type
                    {
                        *layer_text = text.clone();
                    }
                }
                EssentialPropertyType::Color { value } => {
                    if let crate::core::timeline::LayerType::Solid { ref mut color } =
                        layer.layer_type
                    {
                        *color = *value;
                    } else if let crate::core::timeline::LayerType::Text { ref mut color, .. } =
                        layer.layer_type
                    {
                        *color = *value;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Self-contained Motion Graphics Template (MOGRT) package containing manifest, project data, and metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MogrtPackage {
    pub manifest: MogrtManifest,
    pub project_json: String,
    pub thumbnail_base64: Option<String>,
}

impl MogrtPackage {
    pub fn new(manifest: MogrtManifest, project_json: impl Into<String>) -> Self {
        Self {
            manifest,
            project_json: project_json.into(),
            thumbnail_base64: None,
        }
    }

    /// Serializes MOGRT package to formatted JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes MOGRT package from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

/// Generates a MOGRT manifest from a composition and registered essential properties.
pub fn create_mogrt_manifest(
    comp: &Composition,
    name: impl Into<String>,
    author: impl Into<String>,
    properties: Vec<EssentialProperty>,
) -> MogrtManifest {
    MogrtManifest {
        name: name.into(),
        version: "1.0.0".into(),
        author: author.into(),
        comp_id: comp.id.clone(),
        width: comp.width,
        height: comp.height,
        fps: comp.fps,
        duration_frames: comp.duration_frames,
        essential_properties: properties,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::LayerType;

    #[test]
    fn test_essential_property_override_text_and_position() {
        let mut comp = Composition::new("c1".into(), "Title Comp".into(), 1920, 1080, 30, 300);
        let mut text_layer = Layer::new(
            "l_title".into(),
            "Title Text".into(),
            LayerType::new_text("Default Text", 48, [1.0; 4]),
            300,
        );
        text_layer.transform.position =
            crate::core::property::Animatable::new_constant([960.0, 540.0]);
        comp.add_layer(text_layer);

        let overrides = vec![
            EssentialProperty {
                id: "p1".into(),
                name: "Header Text".into(),
                comment: None,
                target_layer_id: "l_title".into(),
                target_property_path: "text.content".into(),
                property_type: EssentialPropertyType::Text {
                    text: "Breaking News".into(),
                },
            },
            EssentialProperty {
                id: "p2".into(),
                name: "Header Position".into(),
                comment: None,
                target_layer_id: "l_title".into(),
                target_property_path: "transform.position".into(),
                property_type: EssentialPropertyType::Point2D {
                    value: [500.0, 200.0],
                },
            },
        ];

        apply_essential_property_overrides(&mut comp, &overrides);

        let layer = &comp.layers[0];
        if let LayerType::Text { text, .. } = &layer.layer_type {
            assert_eq!(text, "Breaking News");
        } else {
            panic!("Expected Text layer");
        }
        assert_eq!(layer.transform.position.evaluate(0), [500.0, 200.0]);
    }

    #[test]
    fn test_mogrt_package_json_roundtrip() {
        let comp = Composition::new("c_main".into(), "Main".into(), 1920, 1080, 60, 600);
        let prop = EssentialProperty {
            id: "ep_1".into(),
            name: "Title".into(),
            comment: Some("Main broadcast title".into()),
            target_layer_id: "l_1".into(),
            target_property_path: "text.content".into(),
            property_type: EssentialPropertyType::Text {
                text: "Summer Festival".into(),
            },
        };
        let manifest = create_mogrt_manifest(&comp, "Summer Template", "AE Team", vec![prop]);
        let pkg = MogrtPackage::new(manifest, "{\"version\": 1}");

        let json = pkg.to_json().expect("Serialization should succeed");
        let decoded = MogrtPackage::from_json(&json).expect("Deserialization should succeed");

        assert_eq!(decoded.manifest.name, "Summer Template");
        assert_eq!(decoded.manifest.fps, 60);
        assert_eq!(decoded.manifest.essential_properties.len(), 1);
        assert_eq!(decoded.manifest.essential_properties[0].name, "Title");
    }
}
