/// Forward & Backward Compatible Project Schema Migration Engine.
///
/// Ensures saved project files (.json) remain 100% loadable even as
/// fields are added, renamed, or refactored across application versions.

use serde::{Deserialize, Serialize};
use crate::core::timeline::Project;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedProjectFile {
    pub schema_version: u32,
    pub project_data: serde_json::Value,
}

/// Load a project JSON string safely, executing schema migrations if needed.
#[allow(dead_code)]
pub fn load_project_migrated(json_str: &str) -> Result<Project, String> {
    // Attempt direct deserialization first
    if let Ok(proj) = serde_json::from_str::<Project>(json_str) {
        return Ok(proj);
    }

    // Try parsing as VersionedProjectFile wrapper
    if let Ok(wrapper) = serde_json::from_str::<VersionedProjectFile>(json_str) {
        let migrated_val = migrate_schema_json(wrapper.schema_version, wrapper.project_data)?;
        return serde_json::from_value::<Project>(migrated_val)
            .map_err(|e| format!("Failed to parse project after migration: {}", e));
    }

    // Fallback attempt: try value-level migration from raw JSON
    let val: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid JSON project file: {}", e))?;

    let schema_ver = val.get("schema_version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let migrated = migrate_schema_json(schema_ver, val)?;
    serde_json::from_value::<Project>(migrated)
        .map_err(|e| format!("Schema migration error: {}", e))
}

/// Serialize a Project to versioned JSON string.
#[allow(dead_code)]
pub fn save_project_versioned(proj: &Project) -> Result<String, String> {
    let proj_val = serde_json::to_value(proj).map_err(|e| e.to_string())?;
    let wrapper = VersionedProjectFile {
        schema_version: CURRENT_SCHEMA_VERSION,
        project_data: proj_val,
    };
    serde_json::to_string_pretty(&wrapper).map_err(|e| e.to_string())
}

/// Migrate JSON schema from `from_version` to `CURRENT_SCHEMA_VERSION`.
fn migrate_schema_json(from_version: u32, mut data: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut version = from_version;

    while version < CURRENT_SCHEMA_VERSION {
        match version {
            0 => {
                // Schema v0 -> v1 migration: ensure active_composition_idx exists
                if let Some(obj) = data.as_object_mut() {
                    if !obj.contains_key("active_composition_idx") {
                        obj.insert("active_composition_idx".to_string(), serde_json::json!(0));
                    }
                    if !obj.contains_key("assets") {
                        obj.insert("assets".to_string(), serde_json::json!([]));
                    }
                }
                version = 1;
            }
            _ => break,
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_versioned_project() {
        let proj = Project::default_demo_project();
        let json = save_project_versioned(&proj).unwrap();
        let loaded = load_project_migrated(&json).unwrap();
        assert_eq!(loaded.compositions.len(), proj.compositions.len());
    }

    #[test]
    fn test_migrate_v0_json() {
        let v0_json = r#"{
            "compositions": [{
                "id": "c1", "name": "Comp 1", "width": 1920, "height": 1080, "fps": 30, "duration_frames": 300,
                "layers": [], "motion_blur_shutter_angle": 180.0, "background_color": [0.0,0.0,0.0,1.0],
                "active_camera": {"fov_degrees": 50.0, "position": {"type": "Constant", "value": [0.0,0.0,1000.0]}},
                "lights": [], "markers": []
            }]
        }"#;
        let loaded = load_project_migrated(v0_json).unwrap();
        assert_eq!(loaded.active_composition_idx, 0);
    }
}
