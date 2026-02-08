use crate::domain::models::{
    AppError, AspectRatio, CameraIntensity, ExportProfile, ProjectManifest, Resolution,
};
use crate::infra::ffmpeg::command::{ffprobe_bin, run_ffmpeg, CommandOutput};
use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct ExportAttemptResult {
    pub success: bool,
    pub used_codec: String,
    pub stderr: String,
    pub output_path: String,
}

pub fn export_with_fallback(
    manifest: &ProjectManifest,
    input_path: &Path,
    output_path: &Path,
    profile: &ExportProfile,
) -> Result<ExportAttemptResult, AppError> {
    let primary_codec = hardware_codec();
    let mut first = run_export_once(manifest, input_path, output_path, profile, primary_codec)?;
    if first.status.success() {
        return Ok(ExportAttemptResult {
            success: true,
            used_codec: primary_codec.to_string(),
            stderr: first.stderr,
            output_path: output_path.to_string_lossy().to_string(),
        });
    }

    let fallback_codec = "libx264";
    let second = run_export_once(manifest, input_path, output_path, profile, fallback_codec)?;
    if second.status.success() {
        let mut stderr = first.stderr;
        if !stderr.is_empty() {
            stderr.push_str("\n---- fallback ----\n");
        }
        stderr.push_str(&second.stderr);
        return Ok(ExportAttemptResult {
            success: true,
            used_codec: fallback_codec.to_string(),
            stderr,
            output_path: output_path.to_string_lossy().to_string(),
        });
    }

    first.stderr.push_str("\n---- fallback ----\n");
    first.stderr.push_str(&second.stderr);
    Ok(ExportAttemptResult {
        success: false,
        used_codec: fallback_codec.to_string(),
        stderr: first.stderr,
        output_path: output_path.to_string_lossy().to_string(),
    })
}

fn run_export_once(
    manifest: &ProjectManifest,
    input_path: &Path,
    output_path: &Path,
    profile: &ExportProfile,
    codec: &str,
) -> Result<CommandOutput, AppError> {
    let (target_w, target_h) = output_resolution(
        profile.resolution.clone(),
        manifest.timeline.aspect_ratio.clone(),
    );
    let mut args: Vec<String> = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "info".to_string(),
        "-stats".to_string(),
    ];

    if manifest.timeline.trim_start_ms > 0 {
        args.push("-ss".to_string());
        args.push(format!(
            "{:.3}",
            manifest.timeline.trim_start_ms as f64 / 1000.0
        ));
    }
    if manifest.timeline.trim_end_ms > manifest.timeline.trim_start_ms {
        args.push("-to".to_string());
        args.push(format!(
            "{:.3}",
            manifest.timeline.trim_end_ms as f64 / 1000.0
        ));
    }

    args.push("-i".to_string());
    args.push(input_path.to_string_lossy().to_string());

    let vf = build_video_filters(manifest, profile, input_path);
    args.push("-vf".to_string());
    args.push(vf);

    args.push("-r".to_string());
    args.push(profile.fps.to_string());
    args.push("-c:v".to_string());
    args.push(codec.to_string());
    args.push("-b:v".to_string());
    args.push(format!("{}M", profile.bitrate_mbps));
    args.push("-pix_fmt".to_string());
    args.push("yuv420p".to_string());
    args.push("-c:a".to_string());
    args.push("aac".to_string());
    args.push("-b:a".to_string());
    args.push("128k".to_string());
    args.push("-movflags".to_string());
    args.push("+faststart".to_string());
    args.push("-metadata:s:v:0".to_string());
    args.push("rotate=0".to_string());
    args.push("-aspect".to_string());
    args.push(format!("{target_w}:{target_h}"));
    args.push(output_path.to_string_lossy().to_string());

    run_ffmpeg(args)
}

pub fn classify_export_error(stderr: &str) -> AppError {
    let lower = stderr.to_lowercase();
    if lower.contains("permission denied") || lower.contains("access is denied") {
        return AppError::new(
            "NO_PERMISSION",
            "导出路径无权限，无法写入目标文件",
            Some("请切换到有写入权限的路径后重试".to_string()),
        );
    }
    if lower.contains("no space left on device") || lower.contains("there is not enough space") {
        return AppError::new(
            "NO_SPACE",
            "磁盘空间不足，导出失败",
            Some("释放空间后重试导出".to_string()),
        );
    }
    if lower.contains("unknown encoder")
        || lower.contains("error while opening encoder")
        || lower.contains("cannot open encoder")
    {
        return AppError::new(
            "ENCODER_FAIL",
            "编码器初始化失败",
            Some("将自动回退软件编码，或检查本机编码器驱动".to_string()),
        );
    }
    AppError::new(
        "IO_FAIL",
        "导出失败",
        Some("请查看导出日志并重试".to_string()),
    )
}

fn hardware_codec() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "h264_nvenc"
    }
    #[cfg(target_os = "macos")]
    {
        "h264_videotoolbox"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "libx264"
    }
}

fn build_video_filters(
    manifest: &ProjectManifest,
    profile: &ExportProfile,
    input_path: &Path,
) -> String {
    let (target_w, target_h) = output_resolution(
        profile.resolution.clone(),
        manifest.timeline.aspect_ratio.clone(),
    );
    let (source_w, source_h) = probe_input_dimensions(input_path).unwrap_or((target_w, target_h));
    let target_ar = target_w as f64 / target_h as f64;
    let mut filters: Vec<String> = Vec::new();

    filters.push(build_crop_filter(
        manifest,
        target_ar,
        source_w as f64,
        source_h as f64,
    ));

    if manifest.timeline.cursor_highlight_enabled {
        // MVP 使用轻量视觉增强替代复杂光标合成，避免引入轨道级渲染依赖。
        filters.push("eq=contrast=1.03:saturation=1.06".to_string());
    }

    filters.push(format!("scale={target_w}:{target_h}"));
    filters.push("setsar=1".to_string());
    filters.push(format!("setdar={target_w}/{target_h}"));
    filters.join(",")
}

fn build_crop_filter(
    manifest: &ProjectManifest,
    target_ar: f64,
    source_w: f64,
    source_h: f64,
) -> String {
    let zoom = camera_zoom(manifest);
    let crop_w = format!(
        "if(gt(iw/ih,{target_ar:.6}),trunc((ih*{target_ar:.6})/{zoom:.6}/2)*2,trunc(iw/{zoom:.6}/2)*2)"
    );
    let crop_h = format!(
        "if(gt(iw/ih,{target_ar:.6}),trunc(ih/{zoom:.6}/2)*2,trunc((iw/{target_ar:.6})/{zoom:.6}/2)*2)"
    );

    if manifest.camera_motion.enabled {
        let cursor_track = load_cursor_track(manifest);
        if let Some((nx_expr, ny_expr)) = build_cursor_position_expr(
            &cursor_track,
            source_w,
            source_h,
            manifest.camera_motion.smoothing as f64,
            manifest.camera_motion.idle_threshold_ms as f64,
            manifest.camera_motion.intensity.clone(),
        ) {
            let x = format!("max(0,min(iw-ow,iw*({nx_expr})-ow/2))");
            let y = format!("max(0,min(ih-oh,ih*({ny_expr})-oh/2))");
            return format!("crop=w='{crop_w}':h='{crop_h}':x='{x}':y='{y}'");
        }
    }

    format!("crop=w='{crop_w}':h='{crop_h}':x='(iw-ow)/2':y='(ih-oh)/2'")
}

fn camera_zoom(manifest: &ProjectManifest) -> f64 {
    if !manifest.camera_motion.enabled {
        return 1.0;
    }
    let base = match manifest.camera_motion.intensity {
        CameraIntensity::Low => 1.03,
        CameraIntensity::Medium => 1.08,
        CameraIntensity::High => 1.14,
    };
    let responsiveness = (1.0 - manifest.camera_motion.smoothing as f64).clamp(0.0, 1.0); // 低平滑=更激进
    let cap = (manifest.camera_motion.max_zoom as f64).clamp(1.0, 2.0);
    let room = (cap - base).max(0.0);
    let adaptive = base + room * (0.35 + responsiveness * 0.65);
    adaptive.min(cap).clamp(1.0, 2.0)
}

#[derive(Debug, Clone, Copy)]
struct CursorPoint {
    t_sec: f64,
    x: f64,
    y: f64,
}

fn load_cursor_track(manifest: &ProjectManifest) -> Vec<CursorPoint> {
    let Some(path) = manifest.artifacts.cursor_track_path.as_ref() else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| {
            Some(CursorPoint {
                t_sec: value.get("tMs")?.as_u64()? as f64 / 1000.0,
                x: value.get("x")?.as_f64()?,
                y: value.get("y")?.as_f64()?,
            })
        })
        .collect::<Vec<_>>()
}

#[derive(Debug, Deserialize)]
struct InputSizeStream {
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct InputSizeProbe {
    streams: Vec<InputSizeStream>,
}

fn probe_input_dimensions(path: &Path) -> Option<(u32, u32)> {
    let output = Command::new(ffprobe_bin())
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=width,height")
        .arg("-of")
        .arg("json")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let parsed: InputSizeProbe = serde_json::from_slice(&output.stdout).ok()?;
    let stream = parsed.streams.first()?;
    let (Some(width), Some(height)) = (stream.width, stream.height) else {
        return None;
    };
    if width == 0 || height == 0 {
        None
    } else {
        Some((width, height))
    }
}

#[derive(Debug, Clone, Copy)]
struct HybridSettings {
    dead_zone: f64,
    follow_gain: f64,
    recenter_gain: f64,
    micro_follow_gain: f64,
    movement_epsilon: f64,
}

fn hybrid_settings(intensity: CameraIntensity, smoothing: f64) -> HybridSettings {
    let responsiveness = (1.0 - smoothing).clamp(0.0, 1.0);
    let (base_dead_zone, base_follow) = match intensity {
        CameraIntensity::Low => (0.05, 0.22),
        CameraIntensity::Medium => (0.036, 0.30),
        CameraIntensity::High => (0.026, 0.38),
    };
    let base_micro = match intensity {
        CameraIntensity::Low => 0.05,
        CameraIntensity::Medium => 0.07,
        CameraIntensity::High => 0.10,
    };
    let follow_gain = (base_follow + responsiveness * 0.30).clamp(0.18, 0.72);
    let dead_zone = (base_dead_zone - responsiveness * 0.012).clamp(0.012, 0.08);
    let recenter_gain = (follow_gain * 0.56).clamp(0.10, 0.36);
    let micro_follow_gain = (base_micro + responsiveness * 0.06).clamp(0.03, 0.22);
    let movement_epsilon = (dead_zone * 0.10).clamp(0.003, 0.018);
    HybridSettings {
        dead_zone,
        follow_gain,
        recenter_gain,
        micro_follow_gain,
        movement_epsilon,
    }
}

fn follow_with_dead_zone(center: f64, target: f64, settings: HybridSettings) -> f64 {
    let delta = target - center;
    if delta.abs() <= settings.dead_zone {
        // 在死区内保留少量跟随，避免焦点“粘住”导致明显迟滞。
        return (center + delta * settings.micro_follow_gain).clamp(0.03, 0.97);
    }
    let overshoot = delta.signum() * (delta.abs() - settings.dead_zone);
    (center + overshoot * settings.follow_gain).clamp(0.03, 0.97)
}

fn build_cursor_position_expr(
    points: &[CursorPoint],
    source_w: f64,
    source_h: f64,
    smoothing: f64,
    idle_threshold_ms: f64,
    intensity: CameraIntensity,
) -> Option<(String, String)> {
    if points.is_empty() {
        return None;
    }
    let safe_w = source_w.max(1.0);
    let safe_h = source_h.max(1.0);
    let settings = hybrid_settings(intensity, smoothing);
    let effective_idle_threshold_ms = (idle_threshold_ms.clamp(120.0, 900.0)
        * (0.65 + smoothing.clamp(0.0, 1.0) * 0.20))
        .clamp(120.0, 900.0);

    // FFmpeg 表达式嵌套层数有限，分段过多会导致 crop 表达式解析失败。
    const MAX_SEGMENTS: usize = 64;
    let normalized = points
        .iter()
        .map(|point| {
            let nx = (point.x / safe_w).clamp(0.02, 0.98);
            let ny = (point.y / safe_h).clamp(0.02, 0.98);
            (point.t_sec, nx, ny)
        })
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        return None;
    }

    // 先在完整光标轨迹上平滑，再降采样构造表达式，避免“先抽样后平滑”带来的跟随迟滞。
    let mut center_x = normalized[0].1;
    let mut center_y = normalized[0].2;
    let mut full_smooth_points = Vec::with_capacity(normalized.len());
    full_smooth_points.push((normalized[0].0, center_x, center_y));
    let mut prev_time = normalized[0].0;
    let mut prev_cursor_x = normalized[0].1;
    let mut prev_cursor_y = normalized[0].2;
    let mut idle_acc_ms = 0.0;
    for (t_sec, nx, ny) in normalized.into_iter().skip(1) {
        let dt_ms = ((t_sec - prev_time).max(0.0)) * 1000.0;
        let movement = ((nx - prev_cursor_x).powi(2) + (ny - prev_cursor_y).powi(2)).sqrt();
        if movement <= settings.movement_epsilon {
            idle_acc_ms += dt_ms;
        } else {
            idle_acc_ms = 0.0;
        }

        if idle_acc_ms >= effective_idle_threshold_ms {
            center_x += (0.5 - center_x) * settings.recenter_gain;
            center_y += (0.5 - center_y) * settings.recenter_gain;
        } else {
            center_x = follow_with_dead_zone(center_x, nx, settings);
            center_y = follow_with_dead_zone(center_y, ny, settings);
        }
        full_smooth_points.push((t_sec, center_x, center_y));
        prev_time = t_sec;
        prev_cursor_x = nx;
        prev_cursor_y = ny;
    }

    let step = full_smooth_points.len().div_ceil(MAX_SEGMENTS).max(1);
    let mut smooth_points = full_smooth_points
        .iter()
        .step_by(step)
        .copied()
        .collect::<Vec<_>>();
    if let Some(last) = full_smooth_points.last().copied() {
        if smooth_points
            .last()
            .map(|item| (item.0 - last.0).abs() > 0.001)
            .unwrap_or(true)
        {
            smooth_points.push(last);
        }
    }

    let x_points = smooth_points
        .iter()
        .map(|(t, x, _)| (*t, *x))
        .collect::<Vec<_>>();
    let y_points = smooth_points
        .iter()
        .map(|(t, _, y)| (*t, *y))
        .collect::<Vec<_>>();
    Some((piecewise_expr(&x_points), piecewise_expr(&y_points)))
}

fn piecewise_expr(points: &[(f64, f64)]) -> String {
    if points.is_empty() {
        return "0.5".to_string();
    }
    if points.len() == 1 {
        return format!("{:.6}", points[0].1);
    }
    let mut expr = format!(
        "{:.6}",
        points.last().map(|(_, value)| *value).unwrap_or(0.5)
    );
    for index in (0..points.len() - 1).rev() {
        let (t0, v0) = points[index];
        let (t1, v1) = points[index + 1];
        let dt = (t1 - t0).max(0.001);
        let seg = format!("({v0:.6}+((t-{t0:.3})/{dt:.3})*{:.6})", v1 - v0);
        expr = format!("if(lt(t,{t1:.3}),{seg},{expr})");
    }
    let (first_t, first_v) = points[0];
    format!("if(lt(t,{first_t:.3}),{first_v:.6},{expr})")
}

fn output_resolution(resolution: Resolution, aspect_ratio: AspectRatio) -> (u32, u32) {
    match (resolution, aspect_ratio) {
        (Resolution::R1080p, AspectRatio::Widescreen) => (1920, 1080),
        (Resolution::R1080p, AspectRatio::Vertical) => (1080, 1920),
        (Resolution::R1080p, AspectRatio::Square) => (1080, 1080),
        (Resolution::R720p, AspectRatio::Widescreen) => (1280, 720),
        (Resolution::R720p, AspectRatio::Vertical) => (720, 1280),
        (Resolution::R720p, AspectRatio::Square) => (720, 720),
    }
}

#[cfg(test)]
mod tests {
    use super::{camera_zoom, classify_export_error};
    use crate::domain::models::{CameraIntensity, ProjectManifest};

    #[test]
    fn classify_permission_error() {
        let err = classify_export_error("Permission denied");
        assert_eq!(err.code, "NO_PERMISSION");
    }

    #[test]
    fn classify_space_error() {
        let err = classify_export_error("No space left on device");
        assert_eq!(err.code, "NO_SPACE");
    }

    #[test]
    fn camera_zoom_should_respect_user_cap() {
        let mut manifest = ProjectManifest::default();
        manifest.camera_motion.enabled = true;
        manifest.camera_motion.intensity = CameraIntensity::High;
        manifest.camera_motion.smoothing = 0.0;
        manifest.camera_motion.max_zoom = 1.10;
        let zoom = camera_zoom(&manifest);
        assert!(zoom <= 1.10 + 1e-6);
    }

    #[test]
    fn camera_zoom_should_use_high_cap_when_responsive() {
        let mut manifest = ProjectManifest::default();
        manifest.camera_motion.enabled = true;
        manifest.camera_motion.intensity = CameraIntensity::High;
        manifest.camera_motion.smoothing = 0.0;
        manifest.camera_motion.max_zoom = 1.50;
        let zoom = camera_zoom(&manifest);
        assert!(zoom > 1.35);
    }
}
