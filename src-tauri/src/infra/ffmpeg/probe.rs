use crate::domain::models::AppError;
use crate::infra::ffmpeg::command::ffprobe_bin;
use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    streams: Vec<ProbeStream>,
    format: ProbeFormat,
}

pub struct ProbeSummary {
    pub container_duration_ms: u64,
    pub video_duration_ms: Option<u64>,
    pub audio_duration_ms: Option<u64>,
}

pub fn probe_media(path: &Path) -> Result<ProbeSummary, AppError> {
    let output = Command::new(ffprobe_bin())
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("stream=codec_type,duration:format=duration")
        .arg("-of")
        .arg("json")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            AppError::new(
                "FFPROBE_EXEC_ERROR",
                format!("failed to run ffprobe: {error}"),
                Some("请安装 ffprobe 并加入 PATH".to_string()),
            )
        })?;
    if !output.status.success() {
        return Err(AppError::new(
            "FFPROBE_EXEC_ERROR",
            String::from_utf8_lossy(&output.stderr).to_string(),
            Some("检查输入媒体文件是否完整".to_string()),
        ));
    }

    let parsed: ProbeOutput = serde_json::from_slice(&output.stdout).map_err(|error| {
        AppError::new(
            "FFPROBE_PARSE_ERROR",
            format!("failed to parse ffprobe output: {error}"),
            None,
        )
    })?;

    let video_duration_ms = parsed
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"))
        .and_then(|stream| parse_duration_ms(stream.duration.as_deref()));
    let audio_duration_ms = parsed
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"))
        .and_then(|stream| parse_duration_ms(stream.duration.as_deref()));
    let container_duration_ms = parse_duration_ms(parsed.format.duration.as_deref()).unwrap_or(0);

    Ok(ProbeSummary {
        container_duration_ms,
        video_duration_ms,
        audio_duration_ms,
    })
}

fn parse_duration_ms(raw: Option<&str>) -> Option<u64> {
    raw.and_then(|value| value.parse::<f64>().ok())
        .map(|seconds| (seconds * 1000.0).max(0.0) as u64)
}

pub fn calc_av_offset_ms(video_duration_ms: Option<u64>, audio_duration_ms: Option<u64>) -> i64 {
    match (video_duration_ms, audio_duration_ms) {
        (Some(video), Some(audio)) => video as i64 - audio as i64,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::calc_av_offset_ms;

    #[test]
    fn av_offset_positive() {
        assert_eq!(calc_av_offset_ms(Some(30_100), Some(30_000)), 100);
    }

    #[test]
    fn av_offset_zero_when_missing() {
        assert_eq!(calc_av_offset_ms(Some(30_000), None), 0);
    }
}
