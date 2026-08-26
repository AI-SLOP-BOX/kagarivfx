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

/// Save a project file atomically using temporary file creation and OS atomic replacement.
/// Guards against 0-byte file corruption during disk full or process interruption.
///
/// Also keeps a one-generation backup (`<name>.json.bak`) of the previously saved
/// file so the user can recover from an accidental save-over.
#[allow(dead_code)]
pub fn save_project_atomic<P: AsRef<std::path::Path>>(proj: &Project, path: P) -> Result<(), String> {
    let json_str = save_project_versioned(proj)?;
    let target_path = path.as_ref();
    let tmp_path = target_path.with_extension("json.tmp");
    let bak_path = target_path.with_extension("json.bak");

    // Preserve the previous generation before overwriting
    if target_path.exists() {
        let _ = std::fs::copy(target_path, &bak_path);
    }

    std::fs::write(&tmp_path, json_str.as_bytes())
        .map_err(|e| format!("Failed to write temporary project file: {}", e))?;

    std::fs::rename(&tmp_path, target_path)
        .map_err(|e| format!("Failed to atomically replace project file: {}", e))?;

    Ok(())
}

/// Loads `<path>` or, if missing/corrupt, falls back to `<path>.bak`.
/// Returns (project, used_backup).
pub fn load_project_with_backup<P: AsRef<std::path::Path>>(path: P) -> Result<(Project, bool), String> {
    let target = path.as_ref();
    let bak = target.with_extension("json.bak");

    if let Ok(json) = std::fs::read_to_string(target) {
        match load_project_migrated(&json) {
            Ok(p) => return Ok((p, false)),
            Err(e) => log::warn!("[Project] Primary file corrupt ({}); trying backup", e),
        }
    }
    if let Ok(json) = std::fs::read_to_string(&bak) {
        match load_project_migrated(&json) {
            Ok(p) => return Ok((p, true)),
            Err(e) => log::warn!("[Project] Backup also corrupt: {}", e),
        }
    }
    Err(format!(
        "Could not load '{}' and no valid backup at '{}'",
        target.display(),
        bak.display()
    ))
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

#[cfg(test)]
mod backup_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn project_named(name: &str) -> Project {
        let mut comp = Composition::new("c1".into(), name.into(), 32, 32, 30, 30);
        comp.layers.push(Layer::new("l".into(), "S".into(), LayerType::Solid { color: [1.0; 4] }, 30));
        Project { compositions: vec![comp], active_composition_idx: 0, assets: Vec::new(), use_gpu_compute: false }
    }

    #[test]
    fn test_atomic_save_creates_backup_of_previous_generation() {
        let dir = std::env::temp_dir().join(format!("aevfx_bak_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("proj.json");

        // First save: no backup yet
        save_project_atomic(&project_named("v1"), &path).unwrap();
        assert!(!path.with_extension("json.bak").exists());

        // Second save: v1 must be preserved as .bak
        save_project_atomic(&project_named("v2"), &path).unwrap();
        assert!(path.with_extension("json.bak").exists());

        let (loaded, used_backup) = load_project_with_backup(&path).unwrap();
        assert_eq!(loaded.compositions[0].name, "v2");
        assert!(!used_backup);

        let bak_loaded = load_project_migrated(
            &std::fs::read_to_string(path.with_extension("json.bak")).unwrap(),
        ).unwrap();
        assert_eq!(bak_loaded.compositions[0].name, "v1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_corrupt_primary_falls_back_to_backup() {
        let dir = std::env::temp_dir().join(format!("aevfx_bak2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("proj.json");

        // Two generations so a .bak exists, then simulate corruption of the primary
        save_project_atomic(&project_named("older_generation"), &path).unwrap();
        save_project_atomic(&project_named("newer_generation"), &path).unwrap();
        std::fs::write(&path, "{ truncated garbage").unwrap();

        // The backup holds the previous generation (newer_generation was in the
        // primary file that we corrupted)
        let (loaded, used_backup) = load_project_with_backup(&path).unwrap();
        assert!(used_backup);
        assert_eq!(loaded.compositions[0].name, "older_generation");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_both_missing_is_an_error() {
        let dir = std::env::temp_dir().join(format!("aevfx_bak3_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let result = load_project_with_backup(dir.join("nonexistent.json"));
        assert!(result.is_err());
    }
}
