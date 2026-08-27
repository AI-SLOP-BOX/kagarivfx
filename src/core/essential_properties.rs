use serde::{Serialize, Deserialize};

/// A master property exposed from a precomp for external override.
/// Analogous to AE's Essential Properties (formerly Master Properties).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EssentialProperty {
    /// Display name shown in the Essential Properties panel.
    pub name: String,
    /// The property type that determines which UI control is shown.
    pub prop_type: EssentialPropertyType,
    /// Current value (overridden by the host comp if set).
    #[serde(default)]
    pub value: EssentialValue,
    /// Whether this property has been overridden at the instance level.
    #[serde(default)]
    pub overridden: bool,
    /// Min value for slider-type properties.
    #[serde(default = "default_min")]
    pub min_value: f32,
    /// Max value for slider-type properties.
    #[serde(default = "default_max")]
    pub max_value: f32,
    /// Dropdown options for menu-type properties.
    #[serde(default)]
    pub options: Vec<String>,
}

fn default_min() -> f32 { 0.0 }
fn default_max() -> f32 { 100.0 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EssentialPropertyType {
    Slider,
    Angle,
    Checkbox,
    Color,
    Point2D,
    Point3D,
    Dropdown,
    LayerRef,
    CompRef,
    FontSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EssentialValue {
    Float(f32),
    Bool(bool),
    Color([f32; 4]),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Text(String),
    Index(usize),
}

impl Default for EssentialValue {
    fn default() -> Self { EssentialValue::Float(0.0) }
}

/// Collects all essential properties defined across precomp layers in a composition.
pub fn collect_essential_properties(
    comp: &crate::core::timeline::Composition,
) -> Vec<(usize, String, EssentialProperty)> {
    let mut props = Vec::new();
    for (layer_idx, layer) in comp.layers.iter().enumerate() {
        if let crate::core::timeline::LayerType::PreComp { .. } = &layer.layer_type {
            for ep in &layer.essential_properties {
                props.push((layer_idx, ep.name.clone(), ep.clone()));
            }
        }
    }
    props
}

/// Apply overridden essential property values into a precomp's layer stack.
/// This is called before rendering a precomp to inject parent-comp overrides.
pub fn apply_essential_overrides(
    precomp: &mut crate::core::timeline::Composition,
    overrides: &[EssentialProperty],
) {
    for ep in overrides {
        if !ep.overridden { continue; }
        for layer in &mut precomp.layers {
            match &mut layer.layer_type {
                crate::core::timeline::LayerType::Text { text, .. } => {
                    if ep.name == "Text" {
                        if let EssentialValue::Text(t) = &ep.value {
                            *text = t.clone();
                        }
                    }
                }
                crate::core::timeline::LayerType::Solid { color } => {
                    if ep.name == "Color" {
                        if let EssentialValue::Color(c) = &ep.value {
                            *color = *c;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_essential_property_types() {
        let ep = EssentialProperty {
            name: "Master Opacity".into(),
            prop_type: EssentialPropertyType::Slider,
            value: EssentialValue::Float(50.0),
            overridden: true,
            min_value: 0.0,
            max_value: 100.0,
            options: vec![],
        };
        assert!(ep.overridden);
        assert_eq!(ep.prop_type, EssentialPropertyType::Slider);
    }

    #[test]
    fn test_essential_value_default() {
        let v = EssentialValue::default();
        match v {
            EssentialValue::Float(f) => assert_eq!(f, 0.0),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_collect_essential_empty_comp() {
        let comp = crate::core::timeline::Composition::new(
            "c1".into(), "Test".into(), 1920, 1080, 30, 300,
        );
        let props = collect_essential_properties(&comp);
        assert!(props.is_empty());
    }

    #[test]
    fn test_apply_essential_overrides_noop() {
        let mut comp = crate::core::timeline::Composition::new(
            "c1".into(), "Test".into(), 1920, 1080, 30, 300,
        );
        apply_essential_overrides(&mut comp, &[]);
        assert!(comp.layers.is_empty());
    }
}
