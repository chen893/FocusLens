use crate::domain::models::{AppError, ExportProfile, RecordingProfile};
use crate::domain::state_machine::{ExportState, RecordingState};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct RecordingSession {
    pub session_id: String,
    pub project_id: String,
    pub profile: RecordingProfile,
    pub state: RecordingState,
    pub started_at: DateTime<Utc>,
    pub degrade_message: Option<String>,
}

#[derive(Debug)]
pub struct RecordingProcess {
    pub child: Child,
}

#[derive(Debug, Clone)]
pub struct ExportTask {
    pub task_id: String,
    pub project_id: String,
    pub profile: ExportProfile,
    pub state: ExportState,
    pub retries: u8,
    pub last_error: Option<AppError>,
}

#[derive(Debug, Clone)]
pub struct CursorTrackSample {
    pub t_ms: u64,
    pub x: f32,
    pub y: f32,
}

pub struct RuntimeState {
    pub project_root: PathBuf,
    pub recording_sessions: Mutex<HashMap<String, RecordingSession>>,
    pub recording_processes: Mutex<HashMap<String, RecordingProcess>>,
    pub cursor_tracks: Mutex<HashMap<String, Arc<Mutex<Vec<CursorTrackSample>>>>>,
    pub export_tasks: Mutex<HashMap<String, ExportTask>>,
    pub settings_path: PathBuf,
}

impl RuntimeState {
    pub fn new(project_root: PathBuf) -> Self {
        let settings_path = project_root
            .parent()
            .unwrap_or(project_root.as_path())
            .join("settings.json");
        Self {
            project_root,
            recording_sessions: Mutex::new(HashMap::new()),
            recording_processes: Mutex::new(HashMap::new()),
            cursor_tracks: Mutex::new(HashMap::new()),
            export_tasks: Mutex::new(HashMap::new()),
            settings_path,
        }
    }
}
