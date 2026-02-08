use crate::core::capture::service::{list_audio_devices, platform_capability};
use crate::domain::models::{AppError, HotkeySettings, RecordingDevice};
use crate::state::RuntimeState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SettingsFile {
    hotkeys: HotkeySettings,
}

#[tauri::command]
pub async fn get_platform_capability() -> crate::core::capture::service::PlatformCapability {
    platform_capability()
}

#[tauri::command]
pub async fn list_audio_input_devices() -> Vec<RecordingDevice> {
    list_audio_devices()
}

#[tauri::command]
pub async fn load_hotkeys(state: State<'_, RuntimeState>) -> Result<HotkeySettings, AppError> {
    let settings = load_or_default_settings(&state)?;
    Ok(settings.hotkeys)
}

#[tauri::command]
pub async fn save_hotkeys(
    state: State<'_, RuntimeState>,
    hotkeys: HotkeySettings,
) -> Result<(), AppError> {
    let settings = SettingsFile { hotkeys };
    write_settings(&state, &settings)
}

fn load_or_default_settings(state: &RuntimeState) -> Result<SettingsFile, AppError> {
    if !state.settings_path.exists() {
        let settings = SettingsFile {
            hotkeys: HotkeySettings::default(),
        };
        write_settings(state, &settings)?;
        return Ok(settings);
    }
    let content = std::fs::read_to_string(&state.settings_path).map_err(|error| {
        AppError::new(
            "SETTINGS_READ_FAIL",
            format!("failed to read settings: {error}"),
            None,
        )
    })?;
    serde_json::from_str::<SettingsFile>(&content).map_err(|error| {
        AppError::new(
            "SETTINGS_PARSE_FAIL",
            format!("failed to parse settings: {error}"),
            None,
        )
    })
}

fn write_settings(state: &RuntimeState, settings: &SettingsFile) -> Result<(), AppError> {
    if let Some(parent) = state.settings_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            AppError::new(
                "SETTINGS_WRITE_FAIL",
                format!("failed to create settings dir: {error}"),
                None,
            )
        })?;
    }
    let raw = serde_json::to_string_pretty(settings).map_err(|error| {
        AppError::new(
            "SETTINGS_WRITE_FAIL",
            format!("failed to serialize settings: {error}"),
            None,
        )
    })?;
    std::fs::write(&state.settings_path, raw).map_err(|error| {
        AppError::new(
            "SETTINGS_WRITE_FAIL",
            format!("failed to write settings: {error}"),
            None,
        )
    })
}
