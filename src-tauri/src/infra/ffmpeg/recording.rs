#[cfg(not(target_os = "windows"))]
use crate::domain::models::Resolution;
use crate::domain::models::{AppError, CaptureMode, RecordingProfile};
#[cfg(target_os = "windows")]
use crate::infra::ffmpeg::command::ffmpeg_supports_input_format;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[cfg(not(target_os = "windows"))]
fn resolution_size(resolution: &Resolution) -> &'static str {
    match resolution {
        Resolution::R1080p => "1920x1080",
        Resolution::R720p => "1280x720",
    }
}

pub struct RecordingSpawn {
    pub child: Child,
    pub degrade_message: Option<String>,
}

fn build_recording_command(
    ffmpeg_bin: &str,
    profile: &RecordingProfile,
    output_path: &Path,
) -> (Command, Option<String>) {
    let mut command = Command::new(ffmpeg_bin);
    command.arg("-y");
    command.arg("-hide_banner");
    command.arg("-loglevel");
    command.arg("warning");
    command.stdin(Stdio::piped());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    #[cfg(target_os = "windows")]
    let degrade_message = configure_windows_capture(&mut command, profile);

    #[cfg(target_os = "macos")]
    let degrade_message = configure_macos_capture(&mut command, profile);

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let degrade_message = configure_mock_capture(&mut command, profile);

    command.arg("-pix_fmt");
    command.arg("yuv420p");
    command.arg("-c:v");
    command.arg("libx264");
    command.arg("-preset");
    command.arg("ultrafast");
    command.arg("-movflags");
    command.arg("+faststart");
    command.arg("-r");
    command.arg(profile.frame_rate.to_string());
    command.arg(output_path.as_os_str());

    (command, degrade_message)
}

fn exited_too_early(child: &mut Child) -> Result<bool, AppError> {
    std::thread::sleep(Duration::from_millis(400));
    let status = child.try_wait().map_err(|error| {
        AppError::new(
            "RECORDING_START_FAIL",
            format!("failed to query recording process status: {error}"),
            None,
        )
    })?;
    Ok(status.is_some())
}

pub fn spawn_recording_process(
    ffmpeg_bin: &str,
    profile: &RecordingProfile,
    output_path: &Path,
) -> Result<RecordingSpawn, AppError> {
    let (mut command, degrade_message) = build_recording_command(ffmpeg_bin, profile, output_path);
    let mut child = command.spawn().map_err(|error| {
        AppError::new(
            "RECORDING_START_FAIL",
            format!("failed to start recording process: {error}"),
            Some("检查录制权限和 ffmpeg 采集设备".to_string()),
        )
    })?;

    if exited_too_early(&mut child)? {
        if profile.system_audio_enabled {
            let mut fallback_profile = profile.clone();
            fallback_profile.system_audio_enabled = false;
            fallback_profile.microphone_device_id = None;

            let (mut fallback_command, _) =
                build_recording_command(ffmpeg_bin, &fallback_profile, output_path);
            let mut fallback_child = fallback_command.spawn().map_err(|error| {
                AppError::new(
                    "RECORDING_START_FAIL",
                    format!("failed to start degraded recording process: {error}"),
                    Some("请关闭系统音频后重试，或检查录制权限".to_string()),
                )
            })?;

            if exited_too_early(&mut fallback_child)? {
                return Err(AppError::new(
                    "RECORDING_START_FAIL",
                    "录制进程启动后立即退出",
                    Some("请检查录制权限、显示会话和音频设备后重试".to_string()),
                ));
            }

            return Ok(RecordingSpawn {
                child: fallback_child,
                degrade_message: Some("系统音频采集不可用，已自动降级为静音轨录制".to_string()),
            });
        }

        return Err(AppError::new(
            "RECORDING_START_FAIL",
            "录制进程启动后立即退出",
            Some("请检查录制权限、显示会话和音频设备后重试".to_string()),
        ));
    }

    Ok(RecordingSpawn {
        child,
        degrade_message,
    })
}

#[cfg(target_os = "windows")]
fn configure_windows_capture(command: &mut Command, profile: &RecordingProfile) -> Option<String> {
    command.arg("-f").arg("gdigrab");
    command
        .arg("-framerate")
        .arg(profile.frame_rate.to_string());
    match profile.capture_mode {
        CaptureMode::Fullscreen => {
            command.arg("-i").arg("desktop");
        }
        CaptureMode::Window => {
            // MVP 阶段优先保证可录制，窗口捕获未指定目标时降级为全屏。
            let window = profile.window_target.as_deref().unwrap_or("desktop");
            command.arg("-i").arg(window);
        }
    }

    let mut audio_inputs = 0usize;
    let mut degrade_message = None;
    if let Some(mic) = profile
        .microphone_device_id
        .as_deref()
        .filter(|value| !value.trim().is_empty() && *value != "default")
    {
        command.arg("-f").arg("dshow");
        command.arg("-i").arg(format!("audio={mic}"));
        audio_inputs += 1;
    }
    let system_audio_enabled = if profile.system_audio_enabled {
        if ffmpeg_supports_input_format("wasapi") {
            true
        } else {
            degrade_message = Some("当前 ffmpeg 不支持 WASAPI，已自动关闭系统音频".to_string());
            false
        }
    } else {
        false
    };
    if system_audio_enabled {
        command.arg("-f").arg("wasapi");
        command.arg("-i").arg("default");
        audio_inputs += 1;
    }

    if audio_inputs == 0 {
        command.arg("-f").arg("lavfi");
        command
            .arg("-i")
            .arg("anullsrc=channel_layout=stereo:sample_rate=48000");
        audio_inputs = 1;
    }

    if audio_inputs >= 2 {
        command.arg("-filter_complex");
        command.arg("[1:a][2:a]amix=inputs=2:duration=longest[aout]");
        command.arg("-map").arg("0:v:0");
        command.arg("-map").arg("[aout]");
    } else {
        command.arg("-map").arg("0:v:0");
        command.arg("-map").arg("1:a:0");
    }
    command.arg("-c:a").arg("aac");
    command.arg("-b:a").arg("128k");
    degrade_message
}

#[cfg(target_os = "macos")]
fn configure_macos_capture(command: &mut Command, profile: &RecordingProfile) -> Option<String> {
    command.arg("-f").arg("avfoundation");
    command
        .arg("-framerate")
        .arg(profile.frame_rate.to_string());
    command
        .arg("-video_size")
        .arg(resolution_size(&profile.resolution));
    command.arg("-i").arg("1:none");
    command
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("anullsrc=channel_layout=stereo:sample_rate=48000");
    command.arg("-map").arg("0:v:0");
    command.arg("-map").arg("1:a:0");
    command.arg("-c:a").arg("aac");
    command.arg("-b:a").arg("128k");

    if profile.system_audio_enabled {
        Some("当前环境不支持系统音频，仅录制麦克风".to_string())
    } else {
        None
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn configure_mock_capture(command: &mut Command, profile: &RecordingProfile) -> Option<String> {
    let size = resolution_size(&profile.resolution);
    command.arg("-f").arg("lavfi");
    command
        .arg("-i")
        .arg(format!("testsrc2=size={size}:rate={}", profile.frame_rate));
    command.arg("-f").arg("lavfi");
    command
        .arg("-i")
        .arg("anullsrc=channel_layout=stereo:sample_rate=48000");
    command.arg("-map").arg("0:v:0");
    command.arg("-map").arg("1:a:0");
    command.arg("-c:a").arg("aac");
    command.arg("-b:a").arg("128k");
    Some("当前平台不在 MVP 支持范围，已启用模拟录制源".to_string())
}

pub fn send_ffmpeg_stdin(child: &mut Child, payload: &[u8]) -> Result<(), AppError> {
    let stdin = child.stdin.as_mut().ok_or_else(|| {
        AppError::new(
            "RECORDING_PROCESS_IO",
            "recording process stdin not available",
            None,
        )
    })?;
    use std::io::Write;
    stdin.write_all(payload).map_err(|error| {
        AppError::new(
            "RECORDING_PROCESS_IO",
            format!("failed to write command to ffmpeg stdin: {error}"),
            None,
        )
    })?;
    stdin.flush().map_err(|error| {
        AppError::new(
            "RECORDING_PROCESS_IO",
            format!("failed to flush ffmpeg stdin: {error}"),
            None,
        )
    })
}

pub fn stop_ffmpeg_process(child: &mut Child) -> Result<(), AppError> {
    let _ = send_ffmpeg_stdin(child, b"q\n");
    for _ in 0..30 {
        if child
            .try_wait()
            .map_err(|error| {
                AppError::new(
                    "RECORDING_STOP_FAIL",
                    format!("failed to query ffmpeg process status: {error}"),
                    None,
                )
            })?
            .is_some()
        {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    child.kill().map_err(|error| {
        AppError::new(
            "RECORDING_STOP_FAIL",
            format!("failed to kill ffmpeg process: {error}"),
            None,
        )
    })?;
    Ok(())
}

pub fn build_ffmpeg_recording_debug_command(
    profile: &RecordingProfile,
    output_path: &Path,
) -> Vec<OsString> {
    let (command, _) = build_recording_command("ffmpeg", profile, output_path);
    command
        .get_args()
        .map(|arg| arg.to_os_string())
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::build_ffmpeg_recording_debug_command;
    use crate::domain::models::RecordingProfile;

    #[test]
    fn build_recording_command_includes_fps_and_output() {
        let profile = RecordingProfile::default();
        let output = std::path::Path::new("recording.mp4");
        let args = build_ffmpeg_recording_debug_command(&profile, output);
        let joined = args
            .iter()
            .map(|item| item.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("30"));
        assert!(joined.contains("recording.mp4"));
    }
}
