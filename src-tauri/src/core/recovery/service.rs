use crate::domain::models::RecoverableProject;
use crate::infra::storage::project_store::{manifest_path, raw_recording_path};
use std::path::Path;

pub fn scan_recoverable_projects(project_root: &Path) -> Vec<RecoverableProject> {
    let mut recovered = Vec::new();
    let entries = match std::fs::read_dir(project_root) {
        Ok(entries) => entries,
        Err(_) => return recovered,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let project_id = match path.file_name().and_then(|name| name.to_str()) {
            Some(project_id) => project_id.to_string(),
            None => continue,
        };

        let marker = path.join("recovery.marker");
        let raw_path = raw_recording_path(project_root, &project_id);
        if marker.exists() && manifest_path(project_root, &project_id).exists() && raw_path.exists()
        {
            recovered.push(RecoverableProject {
                project_id,
                reason: "检测到未完成项目，支持恢复".to_string(),
                path: path.to_string_lossy().to_string(),
            });
        }
    }

    recovered
}
