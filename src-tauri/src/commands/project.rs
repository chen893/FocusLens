use crate::core::export::quality::validate_mvp_quality;
use crate::core::motion::tracker::{compute_motion_path, evaluate_metrics, CursorSample};
use crate::core::recovery::service::scan_recoverable_projects;
use crate::core::timeline::service::apply_timeline_patch;
use crate::domain::models::{
    AppError, CameraMotionPatch, CameraMotionProfile, ProjectManifest, RecoverableProject,
    TimelinePatch,
};
use crate::infra::storage::project_store::{load_manifest, project_dir, save_manifest};
use crate::state::RuntimeState;
use chrono::Utc;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListItem {
    pub project_id: String,
    pub title: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub status: crate::domain::models::ProjectStatus,
    pub duration_ms: u64,
    pub has_export: bool,
    pub export_path: Option<String>,
    pub raw_path: Option<String>,
}

#[tauri::command]
pub async fn load_project(
    state: State<'_, RuntimeState>,
    project_id: String,
) -> Result<ProjectManifest, AppError> {
    ensure_valid_project_id(&project_id)?;
    load_manifest(&state.project_root, &project_id)
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, RuntimeState>,
) -> Result<Vec<ProjectListItem>, AppError> {
    let entries = std::fs::read_dir(&state.project_root).map_err(|error| {
        AppError::new(
            "PROJECT_LIST_READ_FAIL",
            format!("failed to read project root: {error}"),
            Some("请检查项目目录是否可读".to_string()),
        )
    })?;
    let mut projects = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let project_id = entry.file_name().to_string_lossy().to_string();
        if project_id.trim().is_empty() {
            continue;
        }
        let manifest = match load_manifest(&state.project_root, &project_id) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        let duration_ms = manifest
            .timeline
            .trim_end_ms
            .saturating_sub(manifest.timeline.trim_start_ms);
        projects.push(ProjectListItem {
            project_id,
            title: manifest.title,
            created_at: manifest.created_at,
            updated_at: manifest.updated_at,
            status: manifest.status,
            duration_ms,
            has_export: manifest.artifacts.last_export_path.is_some(),
            export_path: manifest.artifacts.last_export_path,
            raw_path: manifest.artifacts.raw_recording_path,
        });
    }
    projects.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(projects)
}

#[tauri::command]
pub async fn update_project_title(
    state: State<'_, RuntimeState>,
    project_id: String,
    title: String,
) -> Result<(), AppError> {
    ensure_valid_project_id(&project_id)?;
    let mut manifest = load_manifest(&state.project_root, &project_id)?;
    let next_title = title.trim().to_string();
    manifest.title = if next_title.is_empty() {
        None
    } else {
        Some(next_title)
    };
    manifest.updated_at = Utc::now();
    save_manifest(&state.project_root, &project_id, &manifest)
}

#[tauri::command]
pub async fn delete_project(
    state: State<'_, RuntimeState>,
    project_id: String,
) -> Result<(), AppError> {
    ensure_valid_project_id(&project_id)?;
    {
        let sessions = state.recording_sessions.lock().map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording sessions",
                None,
            )
        })?;
        if sessions
            .values()
            .any(|session| session.project_id == project_id)
        {
            return Err(AppError::new(
                "PROJECT_BUSY",
                "项目正在录制中，无法删除",
                Some("请先停止录制再删除项目".to_string()),
            ));
        }
    }
    {
        let tasks = state
            .export_tasks
            .lock()
            .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock export tasks", None))?;
        if tasks.values().any(|task| {
            task.project_id == project_id
                && matches!(
                    task.state,
                    crate::domain::state_machine::ExportState::Queued
                        | crate::domain::state_machine::ExportState::Running
                        | crate::domain::state_machine::ExportState::Fallback
                )
        }) {
            return Err(AppError::new(
                "PROJECT_BUSY",
                "项目存在进行中的导出任务，无法删除",
                Some("请等待导出完成后再删除项目".to_string()),
            ));
        }
    }
    if let Ok(mut tasks) = state.export_tasks.lock() {
        tasks.retain(|_, task| task.project_id != project_id);
    }
    let dir = project_dir(&state.project_root, &project_id);
    if !dir.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(&dir).map_err(|error| {
        AppError::new(
            "PROJECT_DELETE_FAIL",
            format!("failed to delete project {project_id}: {error}"),
            Some("请关闭占用该项目文件的程序后重试".to_string()),
        )
    })
}

#[tauri::command]
pub async fn update_timeline(
    state: State<'_, RuntimeState>,
    project_id: String,
    patch: TimelinePatch,
) -> Result<(), AppError> {
    ensure_valid_project_id(&project_id)?;
    let mut manifest = load_manifest(&state.project_root, &project_id)?;
    apply_timeline_patch(&mut manifest, patch);
    if manifest.timeline.trim_end_ms > 0
        && manifest.timeline.trim_end_ms < manifest.timeline.trim_start_ms
    {
        return Err(AppError::new(
            "INVALID_TIMELINE",
            "trimEndMs must be greater than trimStartMs",
            Some("请调整裁剪区间".to_string()),
        ));
    }
    save_manifest(&state.project_root, &project_id, &manifest)
}

#[tauri::command]
pub async fn update_camera_motion(
    state: State<'_, RuntimeState>,
    project_id: String,
    patch: CameraMotionPatch,
) -> Result<(), AppError> {
    ensure_valid_project_id(&project_id)?;
    let mut manifest = load_manifest(&state.project_root, &project_id)?;
    if let Some(enabled) = patch.enabled {
        manifest.camera_motion.enabled = enabled;
    }
    if let Some(intensity) = patch.intensity {
        manifest.camera_motion.intensity = intensity;
    }
    if let Some(smoothing) = patch.smoothing {
        manifest.camera_motion.smoothing = smoothing.clamp(0.0, 1.0);
    }
    if let Some(max_zoom) = patch.max_zoom {
        manifest.camera_motion.max_zoom = max_zoom.clamp(1.0, 2.0);
    }
    if let Some(idle_threshold_ms) = patch.idle_threshold_ms {
        manifest.camera_motion.idle_threshold_ms = idle_threshold_ms.clamp(120, 900);
    }
    manifest.updated_at = Utc::now();
    save_manifest(&state.project_root, &project_id, &manifest)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraMotionQuality {
    pub transition_latency_ms: u64,
    pub idle_jitter_ratio: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityGateStatus {
    pub passed: bool,
    pub reasons: Vec<String>,
}

#[tauri::command]
pub async fn evaluate_camera_motion(
    state: State<'_, RuntimeState>,
    project_id: String,
    override_profile: Option<CameraMotionProfile>,
) -> Result<CameraMotionQuality, AppError> {
    ensure_valid_project_id(&project_id)?;
    let manifest = load_manifest(&state.project_root, &project_id)?;
    let profile = override_profile.unwrap_or(manifest.camera_motion);
    let cursor_path = manifest.artifacts.cursor_track_path.ok_or_else(|| {
        AppError::new(
            "CURSOR_TRACK_MISSING",
            "cursor track path missing in project",
            Some("请先完成录制后再评估镜头运动".to_string()),
        )
    })?;
    let raw = std::fs::read_to_string(&cursor_path).map_err(|error| {
        AppError::new(
            "CURSOR_TRACK_READ_FAIL",
            format!("failed to read cursor track: {error}"),
            None,
        )
    })?;
    let samples_json: Vec<serde_json::Value> = serde_json::from_str(&raw).map_err(|error| {
        AppError::new(
            "CURSOR_TRACK_PARSE_FAIL",
            format!("failed to parse cursor track: {error}"),
            None,
        )
    })?;
    let samples = samples_json
        .iter()
        .filter_map(|item| {
            Some(CursorSample {
                t_ms: item.get("tMs")?.as_u64()?,
                x: item.get("x")?.as_f64()? as f32,
                y: item.get("y")?.as_f64()? as f32,
            })
        })
        .collect::<Vec<_>>();
    let path = compute_motion_path(&samples, &profile);
    let metrics = evaluate_metrics(&samples, &path);
    Ok(CameraMotionQuality {
        transition_latency_ms: metrics.transition_latency_ms,
        idle_jitter_ratio: metrics.idle_jitter_ratio,
    })
}

#[tauri::command]
pub async fn validate_quality_gate(
    state: State<'_, RuntimeState>,
    project_id: String,
) -> Result<QualityGateStatus, AppError> {
    ensure_valid_project_id(&project_id)?;
    let manifest = load_manifest(&state.project_root, &project_id)?;
    let mut reasons = Vec::new();
    if !matches!(
        manifest.status,
        crate::domain::models::ProjectStatus::ExportSucceeded
    ) {
        reasons.push("尚未完成成功导出，无法进行质量门槛校验".to_string());
    }
    let last_export = manifest.artifacts.last_export_path.clone();
    let export_log = manifest.artifacts.export_log_path.clone();
    if last_export
        .as_deref()
        .map(|path| !std::path::Path::new(path).exists())
        .unwrap_or(true)
    {
        reasons.push("缺少导出视频文件，无法校验 A/V 指标".to_string());
    }
    if export_log
        .as_deref()
        .map(|path| !std::path::Path::new(path).exists())
        .unwrap_or(true)
    {
        reasons.push("缺少导出日志，无法校验掉帧率指标".to_string());
    }

    let result = validate_mvp_quality(
        manifest.quality.av_offset_ms,
        manifest.quality.avg_drop_rate,
        manifest.quality.peak_drop_rate,
    );
    reasons.extend(result.reasons);
    Ok(QualityGateStatus {
        passed: reasons.is_empty() && result.passed,
        reasons,
    })
}

#[tauri::command]
pub async fn recover_projects(
    state: State<'_, RuntimeState>,
) -> Result<Vec<RecoverableProject>, AppError> {
    Ok(scan_recoverable_projects(&state.project_root))
}

fn ensure_valid_project_id(project_id: &str) -> Result<(), AppError> {
    let trimmed = project_id.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err(AppError::new(
            "INVALID_PROJECT_ID",
            "invalid project id",
            Some("请使用列表中的项目，不要手工输入路径".to_string()),
        ));
    }
    Ok(())
}
