use crate::domain::models::AppError;
use std::ffi::OsStr;
use std::process::{Command, ExitStatus, Stdio};

pub struct CommandOutput {
    pub status: ExitStatus,
    pub stderr: String,
    pub stdout: String,
}

pub fn ffmpeg_bin() -> String {
    std::env::var("FOCUSLENS_FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string())
}

pub fn ffprobe_bin() -> String {
    std::env::var("FOCUSLENS_FFPROBE_PATH").unwrap_or_else(|_| "ffprobe".to_string())
}

pub fn ffmpeg_supports_input_format(format_name: &str) -> bool {
    let output = Command::new(ffmpeg_bin())
        .arg("-hide_banner")
        .arg("-formats")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    let needle = format_name.to_lowercase();
    let body = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    body.lines().any(|line| {
        let lowered = line.trim().to_lowercase();
        lowered.contains(&needle) && (lowered.starts_with('d') || lowered.starts_with("de"))
    })
}

pub fn ensure_ffmpeg_available() -> Result<(), AppError> {
    let output = Command::new(ffmpeg_bin())
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| {
            AppError::new(
                "FFMPEG_NOT_FOUND",
                format!("failed to execute ffmpeg: {error}"),
                Some("请安装 ffmpeg 并加入 PATH，或设置 FOCUSLENS_FFMPEG_PATH".to_string()),
            )
        })?;
    if !output.status.success() {
        return Err(AppError::new(
            "FFMPEG_NOT_FOUND",
            "ffmpeg command exists but returns non-zero on -version",
            Some("确认 ffmpeg 可正常运行".to_string()),
        ));
    }
    Ok(())
}

pub fn run_ffmpeg<I, S>(args: I) -> Result<CommandOutput, AppError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(ffmpeg_bin())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            AppError::new(
                "FFMPEG_EXEC_ERROR",
                format!("failed to run ffmpeg: {error}"),
                Some("确认 ffmpeg 安装状态并检查导出参数".to_string()),
            )
        })?;
    Ok(CommandOutput {
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
    })
}
