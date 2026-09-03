//! OpenFX (OFX) Standard Third-Party Plugin Host Protocol Engine (AE Parity).
//!
//! Provides the foundational host architecture, descriptor reflection, and parameter
//! suite bridging for industry-standard OFX visual effect plugins.

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum OfxParamType {
    Double {
        min: f64,
        max: f64,
        default_value: f64,
    },
    Integer {
        min: i32,
        max: i32,
        default_value: i32,
    },
    Boolean {
        default_value: bool,
    },
    RGB {
        default_value: [f32; 3],
    },
    RGBA {
        default_value: [f32; 4],
    },
    Choice {
        options: Vec<String>,
        default_index: usize,
    },
    StringParam {
        default_value: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OfxParamDescriptor {
    pub name: String,
    pub label: String,
    pub hint: Option<String>,
    pub param_type: OfxParamType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OfxPluginDescriptor {
    pub plugin_id: String,
    pub major_version: u32,
    pub minor_version: u32,
    pub name: String,
    pub category: String,
    pub parameters: Vec<OfxParamDescriptor>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OfxPluginInstance {
    pub plugin_id: String,
    pub instance_id: String,
    pub param_values: std::collections::HashMap<String, serde_json::Value>,
}

impl OfxPluginInstance {
    pub fn new(descriptor: &OfxPluginDescriptor, instance_id: impl Into<String>) -> Self {
        let mut param_values = std::collections::HashMap::new();
        for param in &descriptor.parameters {
            let val = match &param.param_type {
                OfxParamType::Double { default_value, .. } => serde_json::json!(default_value),
                OfxParamType::Integer { default_value, .. } => serde_json::json!(default_value),
                OfxParamType::Boolean { default_value } => serde_json::json!(default_value),
                OfxParamType::RGB { default_value } => serde_json::json!(default_value),
                OfxParamType::RGBA { default_value } => serde_json::json!(default_value),
                OfxParamType::Choice { default_index, .. } => serde_json::json!(default_index),
                OfxParamType::StringParam { default_value } => serde_json::json!(default_value),
            };
            param_values.insert(param.name.clone(), val);
        }

        Self {
            plugin_id: descriptor.plugin_id.clone(),
            instance_id: instance_id.into(),
            param_values,
        }
    }

    pub fn set_double(&mut self, param_name: &str, val: f64) {
        self.param_values
            .insert(param_name.to_string(), serde_json::json!(val));
    }

    pub fn get_double(&self, param_name: &str) -> Option<f64> {
        self.param_values.get(param_name).and_then(|v| v.as_f64())
    }
}

/// Host manager registry for discovered OFX bundles.
pub struct OfxHostRegistry {
    pub plugins: Vec<OfxPluginDescriptor>,
}

impl Default for OfxHostRegistry {
    fn default() -> Self {
        Self {
            plugins: vec![OfxPluginDescriptor {
                plugin_id: "com.genarts.sapphire.glow".into(),
                major_version: 2024,
                minor_version: 1,
                name: "S_Glow".into(),
                category: "Sapphire Lighting".into(),
                parameters: vec![
                    OfxParamDescriptor {
                        name: "brightness".into(),
                        label: "Brightness".into(),
                        hint: Some("Overall glow intensity".into()),
                        param_type: OfxParamType::Double {
                            min: 0.0,
                            max: 10.0,
                            default_value: 1.0,
                        },
                    },
                    OfxParamDescriptor {
                        name: "color".into(),
                        label: "Glow Color".into(),
                        hint: None,
                        param_type: OfxParamType::RGB {
                            default_value: [1.0, 1.0, 1.0],
                        },
                    },
                ],
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ofx_plugin_instance_instantiation_and_parameter_access() {
        let registry = OfxHostRegistry::default();
        let desc = &registry.plugins[0];

        let mut instance = OfxPluginInstance::new(desc, "glow_inst_1");
        assert_eq!(instance.get_double("brightness"), Some(1.0));

        instance.set_double("brightness", 3.5);
        assert_eq!(instance.get_double("brightness"), Some(3.5));
    }
}
