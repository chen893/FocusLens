use crate::domain::models::RecordingDevice;
use crate::infra::ffmpeg::command::ffmpeg_supports_input_format;
use serde::Serialize;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapability {
    pub platform: String,
    pub supports_screen_capture: bool,
    pub supports_window_capture: bool,
    pub supports_microphone: bool,
    pub supports_system_audio: bool,
    pub system_audio_degrade_message: Option<String>,
}

pub fn platform_capability() -> PlatformCapability {
    #[cfg(target_os = "windows")]
    {
        let supports_system_audio = ffmpeg_supports_input_format("wasapi");
        PlatformCapability {
            platform: "windows".to_string(),
            supports_screen_capture: true,
            supports_window_capture: true,
            supports_microphone: true,
            supports_system_audio,
            system_audio_degrade_message: if supports_system_audio {
                None
            } else {
                Some("当前 ffmpeg 不支持 WASAPI，系统音频将自动关闭".to_string())
            },
        }
    }
    #[cfg(target_os = "macos")]
    {
        PlatformCapability {
            platform: "macos".to_string(),
            supports_screen_capture: true,
            supports_window_capture: true,
            supports_microphone: true,
            supports_system_audio: false,
            system_audio_degrade_message: Some("当前环境不支持系统音频，仅录制麦克风".to_string()),
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        PlatformCapability {
            platform: "unsupported".to_string(),
            supports_screen_capture: false,
            supports_window_capture: false,
            supports_microphone: false,
            supports_system_audio: false,
            system_audio_degrade_message: Some("当前平台不在 MVP 支持范围".to_string()),
        }
    }
}

pub fn list_audio_devices() -> Vec<RecordingDevice> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new(
            std::env::var("FOCUSLENS_FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string()),
        )
        .arg("-hide_banner")
        .arg("-list_devices")
        .arg("true")
        .arg("-f")
        .arg("dshow")
        .arg("-i")
        .arg("dummy")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
        if let Ok(output) = output {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut devices = Vec::new();
            for line in stderr.lines() {
                let trimmed = line.trim();
                if !trimmed.contains('"') {
                    continue;
                }
                if !trimmed.to_lowercase().contains("audio") {
                    continue;
                }
                if let Some(label) = extract_quoted(trimmed) {
                    devices.push(RecordingDevice {
                        id: label.clone(),
                        label,
                        kind: "microphone".to_string(),
                    });
                }
            }
            if !devices.is_empty() {
                return devices;
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new(
            std::env::var("FOCUSLENS_FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string()),
        )
        .arg("-hide_banner")
        .arg("-f")
        .arg("avfoundation")
        .arg("-list_devices")
        .arg("true")
        .arg("-i")
        .arg("")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
        if let Ok(output) = output {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut devices = Vec::new();
            let mut in_audio_section = false;
            for line in stderr.lines() {
                let trimmed = line.trim();
                if trimmed.contains("AVFoundation audio devices") {
                    in_audio_section = true;
                    continue;
                }
                if trimmed.contains("AVFoundation video devices") {
                    in_audio_section = false;
                    continue;
                }
                if !in_audio_section {
                    continue;
                }
                if let Some((id, label)) = extract_device_index_and_label(trimmed) {
                    devices.push(RecordingDevice {
                        id,
                        label,
                        kind: "microphone".to_string(),
                    });
                }
            }
            if !devices.is_empty() {
                return devices;
            }
        }
    }
    vec![RecordingDevice {
        id: "default".to_string(),
        label: "Default Microphone".to_string(),
        kind: "microphone".to_string(),
    }]
}

fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')?;
    Some(line[start + 1..start + 1 + end].to_string())
}

#[cfg(target_os = "macos")]
fn extract_device_index_and_label(line: &str) -> Option<(String, String)> {
    let left = line.find('[')?;
    let right = line[left + 1..].find(']')?;
    let idx = line[left + 1..left + 1 + right].trim().to_string();
    let label = extract_quoted(line)?;
    Some((idx, label))
}
