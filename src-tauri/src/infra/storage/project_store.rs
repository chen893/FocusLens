use crate::domain::models::{
    AppError, CameraMotionProfile, ExportProfile, ProjectArtifacts, ProjectManifest, ProjectStatus,
    QualityMetrics, RecordingProfile, TimelineConfig,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const CURRENT_SCHEMA_VERSION: u8 = 1;

pub fn project_dir(project_root: &Path, project_id: &str) -> PathBuf {
    project_root.join(project_id)
}

pub fn manifest_path(project_root: &Path, project_id: &str) -> PathBuf {
    project_dir(project_root, project_id).join("project.json")
}

pub fn ensure_project_dirs(project_root: &Path, project_id: &str) -> Result<(), AppError> {
    let dir = project_dir(project_root, project_id);
    std::fs::create_dir_all(dir.join("assets")).map_err(|error| {
        AppError::new(
            "IO_ERROR",
            format!("failed to create assets dir: {error}"),
            Some("检查路径权限".to_string()),
        )
    })?;
    std::fs::create_dir_all(dir.join("renders")).map_err(|error| {
        AppError::new(
            "IO_ERROR",
            format!("failed to create renders dir: {error}"),
            Some("检查路径权限".to_string()),
        )
    })?;
    Ok(())
}

pub fn create_project_manifest(recording: RecordingProfile) -> ProjectManifest {
    let now = Utc::now();
    ProjectManifest {
        schema_version: CURRENT_SCHEMA_VERSION,
        app_version: "0.1.0".to_string(),
        title: None,
        created_at: now,
        updated_at: now,
        recording,
        camera_motion: CameraMotionProfile::default(),
        export: ExportProfile::default(),
        timeline: TimelineConfig::default(),
        artifacts: ProjectArtifacts::default(),
        quality: QualityMetrics::default(),
        status: ProjectStatus::ReadyToEdit,
        last_error: None,
    }
}

pub fn raw_recording_path(project_root: &Path, project_id: &str) -> PathBuf {
    project_dir(project_root, project_id)
        .join("assets")
        .join("recording_raw.mp4")
}

pub fn cursor_track_path(project_root: &Path, project_id: &str) -> PathBuf {
    project_dir(project_root, project_id)
        .join("assets")
        .join("cursor_track.json")
}

pub fn export_output_path(project_root: &Path, project_id: &str) -> PathBuf {
    project_dir(project_root, project_id)
        .join("renders")
        .join("output.mp4")
}

pub fn export_log_path(project_root: &Path, project_id: &str, task_id: &str) -> PathBuf {
    project_dir(project_root, project_id)
        .join("renders")
        .join(format!("export-{task_id}.log"))
}

pub fn save_manifest(
    project_root: &Path,
    project_id: &str,
    manifest: &ProjectManifest,
) -> Result<(), AppError> {
    ensure_project_dirs(project_root, project_id)?;
    let path = manifest_path(project_root, project_id);
    let content = serde_json::to_string_pretty(manifest).map_err(|error| {
        AppError::new(
            "SERDE_ERROR",
            format!("failed to serialize manifest: {error}"),
            None,
        )
    })?;
    std::fs::write(path, content).map_err(|error| {
        AppError::new(
            "IO_ERROR",
            format!("failed to write manifest: {error}"),
            Some("确认磁盘空间和路径权限".to_string()),
        )
    })?;
    Ok(())
}

pub fn load_manifest(project_root: &Path, project_id: &str) -> Result<ProjectManifest, AppError> {
    let path = manifest_path(project_root, project_id);
    if !path.exists() {
        return Err(AppError::new(
            "PROJECT_NOT_FOUND",
            format!("project manifest not found: {project_id}"),
            Some("先完成一次录制生成项目".to_string()),
        ));
    }
    load_manifest_from_file(&path)
}

pub fn load_manifest_from_file(path: &Path) -> Result<ProjectManifest, AppError> {
    let raw = std::fs::read_to_string(path).map_err(|error| {
        AppError::new(
            "IO_ERROR",
            format!("failed to read manifest: {error}"),
            Some("确认 project.json 可读".to_string()),
        )
    })?;

    let mut value: Value = serde_json::from_str(&raw).map_err(|error| {
        AppError::new(
            "SERDE_ERROR",
            format!("failed to parse manifest json: {error}"),
            None,
        )
    })?;

    let schema_version = value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u8;

    if schema_version > CURRENT_SCHEMA_VERSION {
        return Err(AppError::new(
            "UNSUPPORTED_SCHEMA",
            format!(
                "schemaVersion {schema_version} is newer than supported {}",
                CURRENT_SCHEMA_VERSION
            ),
            Some("请升级应用后重试".to_string()),
        ));
    }

    if schema_version < CURRENT_SCHEMA_VERSION {
        value = migrate_to_v1(value)?;
    }

    serde_json::from_value(value).map_err(|error| {
        AppError::new(
            "SERDE_ERROR",
            format!("failed to decode manifest: {error}"),
            None,
        )
    })
}

pub fn mark_recovery_marker(project_root: &Path, project_id: &str) -> Result<(), AppError> {
    let marker_path = project_dir(project_root, project_id).join("recovery.marker");
    std::fs::write(marker_path, "recoverable").map_err(|error| {
        AppError::new(
            "IO_ERROR",
            format!("failed to create recovery marker: {error}"),
            None,
        )
    })?;
    Ok(())
}

pub fn clear_recovery_marker(project_root: &Path, project_id: &str) -> Result<(), AppError> {
    let marker_path = project_dir(project_root, project_id).join("recovery.marker");
    if marker_path.exists() {
        std::fs::remove_file(marker_path).map_err(|error| {
            AppError::new(
                "IO_ERROR",
                format!("failed to clear recovery marker: {error}"),
                None,
            )
        })?;
    }
    Ok(())
}

fn migrate_to_v1(mut value: Value) -> Result<Value, AppError> {
    if !value.is_object() {
        return Err(AppError::new(
            "MIGRATION_ERROR",
            "legacy manifest should be a JSON object",
            None,
        ));
    }

    let now = Utc::now().to_rfc3339();
    let defaults = json!({
      "schemaVersion": CURRENT_SCHEMA_VERSION,
      "appVersion": "0.1.0",
      "title": null,
      "createdAt": now,
      "updatedAt": now,
      "recording": RecordingProfile::default(),
      "cameraMotion": CameraMotionProfile::default(),
      "export": ExportProfile::default(),
      "timeline": TimelineConfig::default(),
      "artifacts": ProjectArtifacts::default(),
      "quality": QualityMetrics::default(),
      "status": "ready_to_edit",
      "lastError": null
    });

    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::new("MIGRATION_ERROR", "manifest is not an object", None))?;
    let default_object = defaults.as_object().expect("defaults should be object");
    for (key, default_value) in default_object {
        if !object.contains_key(key) {
            object.insert(key.clone(), default_value.clone());
        }
    }
    object.insert(
        "schemaVersion".to_string(),
        Value::Number(CURRENT_SCHEMA_VERSION.into()),
    );
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{load_manifest_from_file, CURRENT_SCHEMA_VERSION};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn reject_future_schema() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("project.json");
        let content = json!({
          "schemaVersion": CURRENT_SCHEMA_VERSION + 1,
          "appVersion": "99.0.0"
        });
        std::fs::write(&path, serde_json::to_string(&content).unwrap()).unwrap();
        let result = load_manifest_from_file(&path);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().code, "UNSUPPORTED_SCHEMA");
    }

    #[test]
    fn migrate_legacy_schema_to_v1() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("project.json");
        let content = json!({
          "schemaVersion": 0
        });
        std::fs::write(&path, serde_json::to_string(&content).unwrap()).unwrap();
        let manifest = load_manifest_from_file(&path).unwrap();
        assert_eq!(manifest.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(manifest.app_version, "0.1.0");
    }
}
