use crate::core::capture::metrics::parse_drop_rates;
use crate::core::export::service::planned_progress;
use crate::domain::models::{AppError, ExportProfile, ProjectStatus};
use crate::domain::state_machine::ExportState;
use crate::infra::ffmpeg::capabilities::detect_hardware_encoder;
use crate::infra::ffmpeg::export::{classify_export_error, export_with_fallback};
use crate::infra::ffmpeg::probe::{calc_av_offset_ms, probe_media};
use crate::infra::storage::project_store::{
    export_log_path, export_output_path, load_manifest, save_manifest,
};
use crate::state::{ExportTask, RuntimeState};
use chrono::Utc;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTaskStatusSnapshot {
    pub task_id: String,
    pub project_id: String,
    pub status: String,
    pub retries: u8,
    pub last_error: Option<AppError>,
}

#[tauri::command]
pub async fn get_export_task_status(
    state: State<'_, RuntimeState>,
    export_task_id: String,
) -> Result<ExportTaskStatusSnapshot, AppError> {
    let tasks = state
        .export_tasks
        .lock()
        .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock export tasks", None))?;
    let task = tasks.get(&export_task_id).ok_or_else(|| {
        AppError::new(
            "EXPORT_TASK_NOT_FOUND",
            format!("export task not found: {export_task_id}"),
            Some("请重新发起导出".to_string()),
        )
    })?;
    Ok(ExportTaskStatusSnapshot {
        task_id: task.task_id.clone(),
        project_id: task.project_id.clone(),
        status: export_state_key(task.state).to_string(),
        retries: task.retries,
        last_error: task.last_error.clone(),
    })
}

#[tauri::command]
pub async fn start_export(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    project_id: String,
    profile: ExportProfile,
) -> Result<String, AppError> {
    ensure_valid_project_id(&project_id)?;
    {
        let tasks = state
            .export_tasks
            .lock()
            .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock export tasks", None))?;
        if tasks.values().any(|task| {
            task.project_id == project_id
                && (task.state == ExportState::Queued
                    || task.state == ExportState::Running
                    || task.state == ExportState::Fallback)
        }) {
            return Err(AppError::new(
                "EXPORT_ALREADY_ACTIVE",
                "当前项目已有导出任务进行中",
                Some("请等待任务完成后再发起新导出".to_string()),
            ));
        }
    }

    let mut manifest = load_manifest(&state.project_root, &project_id)?;
    manifest.status = ProjectStatus::Exporting;
    manifest.export = profile.clone();
    manifest.updated_at = Utc::now();
    save_manifest(&state.project_root, &project_id, &manifest)?;

    let task_id = Uuid::new_v4().to_string();
    let task = ExportTask {
        task_id: task_id.clone(),
        project_id: project_id.clone(),
        profile: profile.clone(),
        state: ExportState::Queued,
        retries: 0,
        last_error: None,
    };
    {
        let mut tasks = state
            .export_tasks
            .lock()
            .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock export tasks", None))?;
        if tasks.values().any(|item| {
            item.project_id == project_id
                && (item.state == ExportState::Queued
                    || item.state == ExportState::Running
                    || item.state == ExportState::Fallback)
        }) {
            return Err(AppError::new(
                "EXPORT_ALREADY_ACTIVE",
                "当前项目已有导出任务进行中",
                Some("请等待任务完成后再发起新导出".to_string()),
            ));
        }
        tasks.insert(task_id.clone(), task);
    }

    schedule_export_pipeline(app, task_id.clone(), project_id, profile, 0);
    Ok(task_id)
}

#[tauri::command]
pub async fn retry_export(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    export_task_id: String,
) -> Result<String, AppError> {
    let (project_id, profile, retries) = {
        let tasks = state
            .export_tasks
            .lock()
            .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock export tasks", None))?;
        let task = tasks.get(&export_task_id).ok_or_else(|| {
            AppError::new(
                "EXPORT_TASK_NOT_FOUND",
                format!("export task not found: {export_task_id}"),
                Some("请重新发起导出".to_string()),
            )
        })?;
        if tasks.values().any(|item| {
            item.task_id != export_task_id
                && item.project_id == task.project_id
                && (item.state == ExportState::Queued
                    || item.state == ExportState::Running
                    || item.state == ExportState::Fallback)
        }) {
            return Err(AppError::new(
                "EXPORT_ALREADY_ACTIVE",
                "当前项目已有导出任务进行中",
                Some("请等待任务完成后再重试".to_string()),
            ));
        }
        (
            task.project_id.clone(),
            task.profile.clone(),
            task.retries.saturating_add(1),
        )
    };

    let new_task_id = Uuid::new_v4().to_string();
    let task = ExportTask {
        task_id: new_task_id.clone(),
        project_id: project_id.clone(),
        profile: profile.clone(),
        state: ExportState::Queued,
        retries,
        last_error: None,
    };
    state
        .export_tasks
        .lock()
        .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock export tasks", None))?
        .insert(new_task_id.clone(), task);

    schedule_export_pipeline(app, new_task_id.clone(), project_id, profile, retries);
    Ok(new_task_id)
}

fn schedule_export_pipeline(
    app: AppHandle,
    task_id: String,
    project_id: String,
    profile: ExportProfile,
    retries: u8,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) =
            run_export_pipeline(&app, &task_id, &project_id, &profile, retries).await
        {
            let _ = app.emit(
                "export/progress",
                serde_json::json!({
                  "taskId": task_id,
                  "status": "failed",
                  "progress": 100,
                  "detail": error.message
                }),
            );
            if let Some(state) = app.try_state::<RuntimeState>() {
                if let Ok(mut tasks) = state.export_tasks.lock() {
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.state = ExportState::Failed;
                        task.last_error = Some(error.clone());
                    }
                }
                let _ = mark_project_export_failed(&state, &project_id, error);
            }
        }
    });
}

async fn run_export_pipeline(
    app: &AppHandle,
    task_id: &str,
    project_id: &str,
    profile: &ExportProfile,
    _retries: u8,
) -> Result<(), AppError> {
    let state = app.state::<RuntimeState>();
    let manifest = load_manifest(&state.project_root, project_id)?;
    let input_path = manifest
        .artifacts
        .raw_recording_path
        .as_ref()
        .ok_or_else(|| {
            AppError::new(
                "PROJECT_ASSET_MISSING",
                "project raw recording path missing",
                Some("请先完成录制并确认 assets/recording_raw.mp4 存在".to_string()),
            )
        })?
        .clone();
    let input_path = std::path::PathBuf::from(input_path);
    if !input_path.exists() {
        return Err(AppError::new(
            "PROJECT_ASSET_MISSING",
            "recording asset file not found",
            Some("请重新录制后再导出".to_string()),
        ));
    }

    let output_path = export_output_path(&state.project_root, project_id);
    let log_path = export_log_path(&state.project_root, project_id, task_id);

    let hw = detect_hardware_encoder();
    tracing::info!("hardware encoder detect: {}", hw.detail);
    let events = planned_progress(task_id, hw.clone());
    for event in events.iter().take(3) {
        sleep(Duration::from_millis(200)).await;
        app.emit("export/progress", event)
            .map_err(|error| AppError::new("EVENT_ERROR", error.to_string(), None))?;
        update_task_status(app, task_id, &event.status)?;
    }

    let result = export_with_fallback(&manifest, &input_path, &output_path, profile)?;
    let log_body = if result.stderr.is_empty() {
        "no stderr output".to_string()
    } else {
        result.stderr.clone()
    };
    std::fs::write(&log_path, log_body.as_bytes()).map_err(|error| {
        AppError::new(
            "IO_ERROR",
            format!("failed to write export log: {error}"),
            None,
        )
    })?;

    if !result.success {
        let app_error = classify_export_error(&result.stderr);
        return Err(app_error);
    }

    let used_fallback = result.used_codec == "libx264" && hw.codec != "libx264";
    if used_fallback {
        app.emit(
            "export/progress",
            serde_json::json!({
              "taskId": task_id,
              "status": "fallback",
              "progress": 62,
              "detail": "硬件编码失败，已回退软件编码"
            }),
        )
        .map_err(|error| AppError::new("EVENT_ERROR", error.to_string(), None))?;
        update_task_status(app, task_id, "fallback")?;
    }

    app.emit(
        "export/progress",
        serde_json::json!({
          "taskId": task_id,
          "status": "running",
          "progress": 85,
          "detail": "正在封装 MP4"
        }),
    )
    .map_err(|error| AppError::new("EVENT_ERROR", error.to_string(), None))?;
    update_task_status(app, task_id, "running")?;

    update_task_status(app, task_id, "success")?;
    mark_project_export_success(app, project_id, &output_path, &log_path)?;

    app.emit(
        "export/progress",
        serde_json::json!({
          "taskId": task_id,
          "status": "success",
          "progress": 100,
          "detail": "导出完成"
        }),
    )
    .map_err(|error| AppError::new("EVENT_ERROR", error.to_string(), None))?;
    Ok(())
}

fn update_task_status(app: &AppHandle, task_id: &str, status: &str) -> Result<(), AppError> {
    let state = app.state::<RuntimeState>();
    let mut tasks = state
        .export_tasks
        .lock()
        .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock export tasks", None))?;
    if let Some(task) = tasks.get_mut(task_id) {
        task.state = match status {
            "queued" => ExportState::Queued,
            "running" => ExportState::Running,
            "fallback" => ExportState::Fallback,
            "success" => ExportState::Success,
            "failed" => ExportState::Failed,
            _ => task.state,
        };
    }
    Ok(())
}

fn export_state_key(state: ExportState) -> &'static str {
    match state {
        ExportState::Queued => "queued",
        ExportState::Running => "running",
        ExportState::Fallback => "fallback",
        ExportState::Success => "success",
        ExportState::Failed => "failed",
    }
}

fn mark_project_export_success(
    app: &AppHandle,
    project_id: &str,
    output_path: &std::path::Path,
    log_path: &std::path::Path,
) -> Result<(), AppError> {
    let state = app.state::<RuntimeState>();
    let mut manifest = load_manifest(&state.project_root, project_id)?;
    manifest.status = ProjectStatus::ExportSucceeded;
    manifest.updated_at = Utc::now();
    manifest.artifacts.last_export_path = Some(output_path.to_string_lossy().to_string());
    manifest.artifacts.export_log_path = Some(log_path.to_string_lossy().to_string());

    if let Ok(summary) = probe_media(output_path) {
        manifest.quality.av_offset_ms =
            calc_av_offset_ms(summary.video_duration_ms, summary.audio_duration_ms);
        if manifest.timeline.trim_end_ms == 0 {
            manifest.timeline.trim_end_ms = summary.container_duration_ms;
        }
    }
    if let Ok(log_raw) = std::fs::read_to_string(log_path) {
        if log_raw.contains("drop=") {
            let (avg_drop, peak_drop) = parse_drop_rates(&log_raw);
            manifest.quality.avg_drop_rate = avg_drop;
            manifest.quality.peak_drop_rate = peak_drop;
        } else {
            manifest.quality.avg_drop_rate = -1.0;
            manifest.quality.peak_drop_rate = -1.0;
        }
    }
    save_manifest(&state.project_root, project_id, &manifest)
}

fn mark_project_export_failed(
    state: &RuntimeState,
    project_id: &str,
    error: AppError,
) -> Result<(), AppError> {
    let mut manifest = load_manifest(&state.project_root, project_id)?;
    manifest.status = ProjectStatus::ExportFailed;
    manifest.last_error = Some(error);
    manifest.updated_at = Utc::now();
    save_manifest(&state.project_root, project_id, &manifest)
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
            Some("请使用系统生成的项目 ID".to_string()),
        ));
    }
    Ok(())
}
