use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProfile {
    pub capture_mode: CaptureMode,
    pub window_target: Option<String>,
    pub frame_rate: u8,
    pub resolution: Resolution,
    pub microphone_device_id: Option<String>,
    pub system_audio_enabled: bool,
    pub hotkeys: Hotkeys,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotkeys {
    pub start_stop: String,
    pub pause_resume: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraMotionProfile {
    pub enabled: bool,
    pub intensity: CameraIntensity,
    pub smoothing: f32,
    pub max_zoom: f32,
    pub idle_threshold_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProfile {
    pub format: ExportFormat,
    pub resolution: Resolution,
    pub bitrate_mbps: u8,
    pub fps: u8,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineConfig {
    pub trim_start_ms: u64,
    pub trim_end_ms: u64,
    pub aspect_ratio: AspectRatio,
    pub cursor_highlight_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub schema_version: u8,
    pub app_version: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub recording: RecordingProfile,
    pub camera_motion: CameraMotionProfile,
    pub export: ExportProfile,
    pub timeline: TimelineConfig,
    pub artifacts: ProjectArtifacts,
    pub quality: QualityMetrics,
    pub status: ProjectStatus,
    pub last_error: Option<AppError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectArtifacts {
    pub raw_recording_path: Option<String>,
    pub cursor_track_path: Option<String>,
    pub last_export_path: Option<String>,
    pub export_log_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityMetrics {
    pub av_offset_ms: i64,
    pub avg_drop_rate: f32,
    pub peak_drop_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePatch {
    pub trim_start_ms: Option<u64>,
    pub trim_end_ms: Option<u64>,
    pub aspect_ratio: Option<AspectRatio>,
    pub cursor_highlight_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CameraMotionPatch {
    pub enabled: Option<bool>,
    pub intensity: Option<CameraIntensity>,
    pub smoothing: Option<f32>,
    pub max_zoom: Option<f32>,
    pub idle_threshold_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverableProject {
    pub project_id: String,
    pub reason: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingDevice {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    pub start_stop: String,
    pub pause_resume: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatusEvent {
    pub session_id: String,
    pub status: String,
    pub duration_ms: u64,
    pub source_label: String,
    pub detail: String,
    pub degrade_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgressEvent {
    pub task_id: String,
    pub status: String,
    pub progress: u8,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl AppError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            suggestion,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    Fullscreen,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    #[serde(rename = "1080p")]
    R1080p,
    #[serde(rename = "720p")]
    R720p,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraIntensity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AspectRatio {
    #[serde(rename = "16:9")]
    Widescreen,
    #[serde(rename = "9:16")]
    Vertical,
    #[serde(rename = "1:1")]
    Square,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Mp4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoCodec {
    H264,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioCodec {
    Aac,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Recording,
    ReadyToEdit,
    Exporting,
    ExportFailed,
    ExportSucceeded,
}

impl Default for RecordingProfile {
    fn default() -> Self {
        Self {
            capture_mode: CaptureMode::Fullscreen,
            window_target: None,
            frame_rate: 30,
            resolution: Resolution::R1080p,
            microphone_device_id: None,
            system_audio_enabled: true,
            hotkeys: Hotkeys {
                start_stop: "Ctrl+Shift+R".to_string(),
                pause_resume: "Ctrl+Shift+P".to_string(),
            },
        }
    }
}

impl Default for CameraMotionProfile {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: CameraIntensity::Medium,
            smoothing: 0.68,
            max_zoom: 1.35,
            idle_threshold_ms: 500,
        }
    }
}

impl Default for ExportProfile {
    fn default() -> Self {
        Self {
            format: ExportFormat::Mp4,
            resolution: Resolution::R1080p,
            bitrate_mbps: 8,
            fps: 30,
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Aac,
        }
    }
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            trim_start_ms: 0,
            trim_end_ms: 0,
            aspect_ratio: AspectRatio::Widescreen,
            cursor_highlight_enabled: true,
        }
    }
}

impl Default for ProjectManifest {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            schema_version: 1,
            app_version: "0.1.0".to_string(),
            title: None,
            created_at: now,
            updated_at: now,
            recording: RecordingProfile::default(),
            camera_motion: CameraMotionProfile::default(),
            export: ExportProfile::default(),
            timeline: TimelineConfig::default(),
            artifacts: ProjectArtifacts::default(),
            quality: QualityMetrics::default(),
            status: ProjectStatus::Recording,
            last_error: None,
        }
    }
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self {
            av_offset_ms: 0,
            avg_drop_rate: 0.0,
            peak_drop_rate: 0.0,
        }
    }
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            start_stop: "Ctrl+Shift+R".to_string(),
            pause_resume: "Ctrl+Shift+P".to_string(),
        }
    }
}
