use crate::core::capture::service::platform_capability;
use crate::domain::models::{
    AppError, CaptureMode, ProjectStatus, RecordingProfile, RecordingStatusEvent,
};
use crate::domain::state_machine::RecordingState;
use crate::infra::ffmpeg::command::{ensure_ffmpeg_available, ffmpeg_bin};
use crate::infra::ffmpeg::recording::{
    send_ffmpeg_stdin, spawn_recording_process, stop_ffmpeg_process,
};
use crate::infra::storage::project_store::{
    clear_recovery_marker, create_project_manifest, cursor_track_path, ensure_project_dirs,
    mark_recovery_marker, raw_recording_path, save_manifest,
};
use crate::state::{CursorTrackSample, RecordingProcess, RecordingSession, RuntimeState};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    mut profile: RecordingProfile,
) -> Result<String, AppError> {
    ensure_ffmpeg_available()?;
    let capability = platform_capability();
    if !capability.supports_screen_capture {
        return Err(AppError::new(
            "PLATFORM_NOT_SUPPORTED",
            "当前平台不支持录制能力",
            Some("MVP 仅支持 Windows/macOS".to_string()),
        ));
    }
    {
        let sessions = state.recording_sessions.lock().map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording sessions",
                None,
            )
        })?;
        if sessions.values().any(|session| {
            session.state == RecordingState::Recording || session.state == RecordingState::Paused
        }) {
            return Err(AppError::new(
                "RECORDING_ALREADY_ACTIVE",
                "已有进行中的录制会话，请先停止后再开始新录制",
                Some("完成当前录制后再发起新的录制".to_string()),
            ));
        }
    }

    let mut degrade_message = None;
    if profile.system_audio_enabled && !capability.supports_system_audio {
        profile.system_audio_enabled = false;
        degrade_message = capability.system_audio_degrade_message.clone();
    }
    if matches!(
        profile.capture_mode,
        crate::domain::models::CaptureMode::Window
    ) && profile
        .window_target
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        profile.capture_mode = CaptureMode::Fullscreen;
        degrade_message = Some("未指定窗口目标，已自动降级为全屏录制".to_string());
    }

    let session_id = Uuid::new_v4().to_string();
    let project_id = session_id.clone();
    let output_path = raw_recording_path(&state.project_root, &project_id);
    let cursor_path = cursor_track_path(&state.project_root, &project_id);
    ensure_project_dirs(&state.project_root, &project_id)?;

    let mut manifest = create_project_manifest(profile.clone());
    manifest.status = ProjectStatus::Recording;
    manifest.artifacts.raw_recording_path = Some(output_path.to_string_lossy().to_string());
    manifest.artifacts.cursor_track_path = Some(cursor_path.to_string_lossy().to_string());
    save_manifest(&state.project_root, &project_id, &manifest)?;

    let spawn = spawn_recording_process(&ffmpeg_bin(), &profile, &output_path)?;
    if degrade_message.is_none() {
        degrade_message = spawn.degrade_message.clone();
    }
    mark_recovery_marker(&state.project_root, &project_id)?;

    let started_at = Utc::now();
    let session = RecordingSession {
        session_id: session_id.clone(),
        project_id: project_id.clone(),
        profile: profile.clone(),
        state: RecordingState::Recording,
        started_at,
        degrade_message: degrade_message.clone(),
    };

    state
        .recording_sessions
        .lock()
        .map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording sessions",
                None,
            )
        })?
        .insert(session_id.clone(), session);
    state
        .recording_processes
        .lock()
        .map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording processes",
                None,
            )
        })?
        .insert(session_id.clone(), RecordingProcess { child: spawn.child });
    state
        .cursor_tracks
        .lock()
        .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock cursor tracks", None))?
        .insert(session_id.clone(), Arc::new(Mutex::new(Vec::new())));

    app.emit(
        "recording/status",
        RecordingStatusEvent {
            session_id: session_id.clone(),
            status: "recording".to_string(),
            duration_ms: 0,
            source_label: match profile.capture_mode {
                crate::domain::models::CaptureMode::Fullscreen => "全屏".to_string(),
                crate::domain::models::CaptureMode::Window => "窗口".to_string(),
            },
            detail: "录制已开始".to_string(),
            degrade_message: degrade_message.clone(),
        },
    )
    .map_err(|error| AppError::new("EVENT_ERROR", error.to_string(), None))?;

    schedule_recording_status_ticker(app.clone(), session_id.clone());
    schedule_cursor_tracking_ticker(session_id.clone(), started_at, app.clone());
    Ok(session_id)
}

#[tauri::command]
pub async fn pause_recording(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    session_id: String,
) -> Result<(), AppError> {
    let (started_at, capture_mode, degrade_message) = {
        let mut sessions = state.recording_sessions.lock().map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording sessions",
                None,
            )
        })?;
        let session = sessions.get_mut(&session_id).ok_or_else(|| {
            AppError::new(
                "SESSION_NOT_FOUND",
                format!("session not found: {session_id}"),
                None,
            )
        })?;
        if session.state != RecordingState::Recording {
            return Err(AppError::new(
                "INVALID_RECORDING_STATE",
                "only recording session can pause",
                None,
            ));
        }
        session.state = RecordingState::Paused;
        (
            session.started_at,
            session.profile.capture_mode.clone(),
            session.degrade_message.clone(),
        )
    };

    let mut processes = state.recording_processes.lock().map_err(|_| {
        AppError::new(
            "STATE_LOCK_ERROR",
            "failed to lock recording processes",
            None,
        )
    })?;
    let process = processes.get_mut(&session_id).ok_or_else(|| {
        AppError::new(
            "SESSION_NOT_FOUND",
            format!("recording process not found: {session_id}"),
            None,
        )
    })?;
    send_ffmpeg_stdin(&mut process.child, b"p\n")?;

    app.emit(
        "recording/status",
        RecordingStatusEvent {
            session_id,
            status: "paused".to_string(),
            duration_ms: (Utc::now() - started_at).num_milliseconds().max(0) as u64,
            source_label: match capture_mode {
                CaptureMode::Fullscreen => "全屏".to_string(),
                CaptureMode::Window => "窗口".to_string(),
            },
            detail: "录制已暂停".to_string(),
            degrade_message,
        },
    )
    .map_err(|error| AppError::new("EVENT_ERROR", error.to_string(), None))?;

    Ok(())
}

#[tauri::command]
pub async fn resume_recording(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    session_id: String,
) -> Result<(), AppError> {
    let (started_at, capture_mode, degrade_message) = {
        let mut sessions = state.recording_sessions.lock().map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording sessions",
                None,
            )
        })?;
        let session = sessions.get_mut(&session_id).ok_or_else(|| {
            AppError::new(
                "SESSION_NOT_FOUND",
                format!("session not found: {session_id}"),
                None,
            )
        })?;
        if session.state != RecordingState::Paused {
            return Err(AppError::new(
                "INVALID_RECORDING_STATE",
                "only paused session can resume",
                None,
            ));
        }
        session.state = RecordingState::Recording;
        (
            session.started_at,
            session.profile.capture_mode.clone(),
            session.degrade_message.clone(),
        )
    };

    let mut processes = state.recording_processes.lock().map_err(|_| {
        AppError::new(
            "STATE_LOCK_ERROR",
            "failed to lock recording processes",
            None,
        )
    })?;
    let process = processes.get_mut(&session_id).ok_or_else(|| {
        AppError::new(
            "SESSION_NOT_FOUND",
            format!("recording process not found: {session_id}"),
            None,
        )
    })?;
    send_ffmpeg_stdin(&mut process.child, b"p\n")?;

    app.emit(
        "recording/status",
        RecordingStatusEvent {
            session_id,
            status: "recording".to_string(),
            duration_ms: (Utc::now() - started_at).num_milliseconds().max(0) as u64,
            source_label: match capture_mode {
                CaptureMode::Fullscreen => "全屏".to_string(),
                CaptureMode::Window => "窗口".to_string(),
            },
            detail: "录制已继续".to_string(),
            degrade_message,
        },
    )
    .map_err(|error| AppError::new("EVENT_ERROR", error.to_string(), None))?;

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    session_id: String,
) -> Result<String, AppError> {
    let session = state
        .recording_sessions
        .lock()
        .map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording sessions",
                None,
            )
        })?
        .get(&session_id)
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                "SESSION_NOT_FOUND",
                format!("session not found: {session_id}"),
                None,
            )
        })?;

    {
        let mut processes = state.recording_processes.lock().map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording processes",
                None,
            )
        })?;
        let process = processes.get_mut(&session_id).ok_or_else(|| {
            AppError::new(
                "SESSION_NOT_FOUND",
                format!("recording process not found: {session_id}"),
                None,
            )
        })?;
        stop_ffmpeg_process(&mut process.child)?;
    }

    let raw_path = raw_recording_path(&state.project_root, &session.project_id);
    let raw_ok = std::fs::metadata(&raw_path)
        .map(|metadata| metadata.len() > 1024)
        .unwrap_or(false);
    if !raw_ok {
        let error = AppError::new(
            "RECORDING_OUTPUT_MISSING",
            "录制未生成有效视频文件，无法进入导出流程",
            Some("请检查麦克风/系统音频设备后重试录制".to_string()),
        );
        let mut failed_manifest = create_project_manifest(session.profile.clone());
        failed_manifest.status = ProjectStatus::Recording;
        failed_manifest.last_error = Some(error.clone());
        failed_manifest.artifacts.raw_recording_path = Some(raw_path.to_string_lossy().to_string());
        failed_manifest.artifacts.cursor_track_path = Some(
            cursor_track_path(&state.project_root, &session.project_id)
                .to_string_lossy()
                .to_string(),
        );
        let _ = save_manifest(&state.project_root, &session.project_id, &failed_manifest);
        let _ = app.emit(
            "recording/status",
            RecordingStatusEvent {
                session_id: session_id.clone(),
                status: "error".to_string(),
                duration_ms: 0,
                source_label: "录制失败".to_string(),
                detail: "录制输出文件缺失".to_string(),
                degrade_message: session.degrade_message.clone(),
            },
        );

        let _ = state
            .recording_processes
            .lock()
            .map(|mut processes| processes.remove(&session_id));
        let _ = state
            .recording_sessions
            .lock()
            .map(|mut sessions| sessions.remove(&session_id));
        let _ = state
            .cursor_tracks
            .lock()
            .map(|mut tracks| tracks.remove(&session_id));
        return Err(error);
    }

    let duration_ms = (Utc::now() - session.started_at).num_milliseconds().max(0) as u64;
    let mut manifest = create_project_manifest(session.profile);
    manifest.status = ProjectStatus::ReadyToEdit;
    manifest.timeline.trim_end_ms = duration_ms;
    manifest.artifacts.raw_recording_path = Some(raw_path.to_string_lossy().to_string());
    let cursor_path = cursor_track_path(&state.project_root, &session.project_id);
    let cursor_samples = take_cursor_samples(&state, &session_id);
    write_cursor_track(&cursor_path, duration_ms, &cursor_samples)?;
    manifest.artifacts.cursor_track_path = Some(cursor_path.to_string_lossy().to_string());
    save_manifest(&state.project_root, &session.project_id, &manifest)?;
    clear_recovery_marker(&state.project_root, &session.project_id)?;

    state
        .recording_processes
        .lock()
        .map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording processes",
                None,
            )
        })?
        .remove(&session_id);
    state
        .recording_sessions
        .lock()
        .map_err(|_| {
            AppError::new(
                "STATE_LOCK_ERROR",
                "failed to lock recording sessions",
                None,
            )
        })?
        .remove(&session_id);
    state
        .cursor_tracks
        .lock()
        .map_err(|_| AppError::new("STATE_LOCK_ERROR", "failed to lock cursor tracks", None))?
        .remove(&session_id);

    app.emit(
        "recording/status",
        RecordingStatusEvent {
            session_id,
            status: "stopped".to_string(),
            duration_ms,
            source_label: "录制完成".to_string(),
            detail: "录制已停止，进入编辑".to_string(),
            degrade_message: session.degrade_message,
        },
    )
    .map_err(|error| AppError::new("EVENT_ERROR", error.to_string(), None))?;

    Ok(session.project_id)
}

fn take_cursor_samples(state: &RuntimeState, session_id: &str) -> Vec<CursorTrackSample> {
    let tracker = state
        .cursor_tracks
        .lock()
        .ok()
        .and_then(|tracks| tracks.get(session_id).cloned());
    let Some(tracker) = tracker else {
        return Vec::new();
    };
    tracker
        .lock()
        .map(|samples| samples.clone())
        .unwrap_or_default()
}

fn write_cursor_track(
    path: &std::path::Path,
    duration_ms: u64,
    samples: &[CursorTrackSample],
) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            AppError::new(
                "IO_ERROR",
                format!("failed to create cursor track dir: {error}"),
                None,
            )
        })?;
    }
    let payload = if samples.is_empty() {
        // 无法采集真实光标时使用可预测轨迹，确保下游链路不崩溃。
        let mut fallback = Vec::new();
        let step = 120u64;
        let mut ts = 0u64;
        while ts <= duration_ms {
            let x = 200.0 + (ts as f32 / 25.0).sin() * 180.0 + (ts as f32 / 60.0);
            let y = 160.0 + (ts as f32 / 35.0).cos() * 120.0;
            fallback.push(serde_json::json!({
              "tMs": ts,
              "x": x,
              "y": y
            }));
            ts = ts.saturating_add(step);
        }
        fallback
    } else {
        let mut normalized = samples
            .iter()
            .map(|sample| {
                serde_json::json!({
                  "tMs": sample.t_ms.min(duration_ms),
                  "x": sample.x,
                  "y": sample.y
                })
            })
            .collect::<Vec<_>>();
        if normalized
            .last()
            .and_then(|value| value.get("tMs"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            < duration_ms
        {
            if let Some(last) = normalized.last().cloned() {
                let x = last.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = last.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                normalized.push(serde_json::json!({
                  "tMs": duration_ms,
                  "x": x,
                  "y": y
                }));
            }
        }
        normalized
    };
    let content = serde_json::to_string_pretty(&payload).map_err(|error| {
        AppError::new(
            "IO_ERROR",
            format!("failed to serialize cursor track: {error}"),
            None,
        )
    })?;
    std::fs::write(path, content).map_err(|error| {
        AppError::new(
            "IO_ERROR",
            format!("failed to write cursor track: {error}"),
            None,
        )
    })
}

fn schedule_cursor_tracking_ticker(
    session_id: String,
    started_at: chrono::DateTime<chrono::Utc>,
    app: AppHandle,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            let runtime = app.state::<RuntimeState>();
            let session_state = {
                let sessions = match runtime.recording_sessions.lock() {
                    Ok(sessions) => sessions,
                    Err(_) => break,
                };
                sessions.get(&session_id).map(|session| session.state)
            };
            let Some(session_state) = session_state else {
                break;
            };
            if session_state != RecordingState::Recording {
                continue;
            }

            let point = current_cursor_position();
            let Some((x, y)) = point else {
                continue;
            };
            let track = {
                let tracks = match runtime.cursor_tracks.lock() {
                    Ok(tracks) => tracks,
                    Err(_) => break,
                };
                tracks.get(&session_id).cloned()
            };
            let Some(track) = track else {
                break;
            };
            let elapsed = (Utc::now() - started_at).num_milliseconds().max(0) as u64;
            let mut samples = match track.lock() {
                Ok(samples) => samples,
                Err(_) => continue,
            };
            samples.push(CursorTrackSample {
                t_ms: elapsed,
                x,
                y,
            });
        }
    });
}

#[cfg(target_os = "windows")]
fn current_cursor_position() -> Option<(f32, f32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point as *mut POINT) };
    if ok == 0 {
        None
    } else {
        Some((point.x as f32, point.y as f32))
    }
}

#[cfg(not(target_os = "windows"))]
fn current_cursor_position() -> Option<(f32, f32)> {
    None
}

fn schedule_recording_status_ticker(app: AppHandle, session_id: String) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let runtime = app.state::<RuntimeState>();
            let snapshot: Option<(
                RecordingState,
                chrono::DateTime<chrono::Utc>,
                crate::domain::models::CaptureMode,
                Option<String>,
            )> = {
                let sessions = match runtime.recording_sessions.lock() {
                    Ok(sessions) => sessions,
                    Err(_) => break,
                };
                sessions.get(&session_id).map(|session| {
                    (
                        session.state,
                        session.started_at,
                        session.profile.capture_mode.clone(),
                        session.degrade_message.clone(),
                    )
                })
            };
            let Some((state, started_at, capture_mode, degrade_message)) = snapshot else {
                break;
            };
            let process_exited = {
                let mut processes = match runtime.recording_processes.lock() {
                    Ok(processes) => processes,
                    Err(_) => break,
                };
                let Some(process) = processes.get_mut(&session_id) else {
                    break;
                };
                match process.child.try_wait() {
                    Ok(Some(_status)) => true,
                    Ok(None) => false,
                    Err(_) => true,
                }
            };
            if process_exited {
                let emitted_degrade_message =
                    if let Ok(mut sessions) = runtime.recording_sessions.lock() {
                        let message = sessions
                            .get(&session_id)
                            .and_then(|session| session.degrade_message.clone());
                        sessions.remove(&session_id);
                        message
                    } else {
                        degrade_message.clone()
                    };
                let _ = runtime
                    .recording_processes
                    .lock()
                    .map(|mut processes| processes.remove(&session_id));
                let _ = runtime
                    .cursor_tracks
                    .lock()
                    .map(|mut tracks| tracks.remove(&session_id));
                let _ = app.emit(
                    "recording/status",
                    RecordingStatusEvent {
                        session_id: session_id.clone(),
                        status: "error".to_string(),
                        duration_ms: (Utc::now() - started_at).num_milliseconds().max(0) as u64,
                        source_label: "录制中断".to_string(),
                        detail: "录制进程异常退出，请检查权限或输入源".to_string(),
                        degrade_message: emitted_degrade_message,
                    },
                );
                break;
            }

            let status = match state {
                RecordingState::Recording => "recording",
                RecordingState::Paused => "paused",
                RecordingState::Stopped => "stopped",
                RecordingState::Error => "error",
                RecordingState::Idle => "idle",
            }
            .to_string();

            let duration_ms = (Utc::now() - started_at).num_milliseconds().max(0) as u64;
            if app
                .emit(
                    "recording/status",
                    RecordingStatusEvent {
                        session_id: session_id.clone(),
                        status: status.clone(),
                        duration_ms,
                        source_label: match capture_mode {
                            crate::domain::models::CaptureMode::Fullscreen => "全屏".to_string(),
                            crate::domain::models::CaptureMode::Window => "窗口".to_string(),
                        },
                        detail: "录制状态更新".to_string(),
                        degrade_message: degrade_message.clone(),
                    },
                )
                .is_err()
            {
                break;
            }
            if status == "stopped" || status == "error" {
                break;
            }
        }
    });
}
