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
    let mut proj: Project = if let Ok(proj) = serde_json::from_str::<Project>(json_str) {
        proj
    } else if let Ok(wrapper) = serde_json::from_str::<VersionedProjectFile>(json_str) {
        let migrated_val = migrate_schema_json(wrapper.schema_version, wrapper.project_data)?;
        serde_json::from_value::<Project>(migrated_val)
            .map_err(|e| format!("Failed to parse project after migration: {}", e))?
    } else {
        let val: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid JSON project file: {}", e))?;
        let schema_ver = val.get("schema_version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let migrated = migrate_schema_json(schema_ver, val)?;
        serde_json::from_value::<Project>(migrated)
            .map_err(|e| format!("Schema migration error: {}", e))?
    };

    // Sanitize any broken or circular parent-child layer links from deserialization
    for comp in &mut proj.compositions {
        comp.sanitize_parent_cycles();
    }

    Ok(proj)
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
                // Schema v0 -> v1 migration: ensure active_composition_idx and name exist
                if let Some(obj) = data.as_object_mut() {
                    if !obj.contains_key("name") {
                        obj.insert("name".to_string(), serde_json::json!("Untitled Project"));
                    }
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
        let proj = Project::default();
        let json = save_project_versioned(&proj).unwrap();
        let loaded = load_project_migrated(&json).unwrap();
        assert_eq!(loaded.compositions.len(), proj.compositions.len());
    }

    #[test]
    fn test_migrate_v0_json() {
        let proj = Project::default();
        let mut proj_val = serde_json::to_value(&proj).unwrap();
        if let Some(obj) = proj_val.as_object_mut() {
            obj.remove("active_composition_idx");
        }
        let v0_json = serde_json::to_string(&proj_val).unwrap();

        let loaded = load_project_migrated(&v0_json).unwrap();
        assert_eq!(loaded.active_composition_idx, 0);
    }
}
