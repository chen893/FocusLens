use crate::core::motion::smoothing::{smooth_motion, MotionConfig, MotionPoint};
use crate::domain::models::{CameraIntensity, CameraMotionProfile};

#[derive(Debug, Clone, Copy)]
pub struct CursorSample {
    pub t_ms: u64,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct MotionMetrics {
    pub transition_latency_ms: u64,
    pub idle_jitter_ratio: f32,
}

pub fn compute_motion_path(
    samples: &[CursorSample],
    profile: &CameraMotionProfile,
) -> Vec<MotionPoint> {
    if samples.is_empty() {
        return Vec::new();
    }
    let config = profile_to_config(profile);
    let mut output = Vec::with_capacity(samples.len());
    let mut current = MotionPoint {
        x: samples[0].x,
        y: samples[0].y,
        zoom: 1.0,
    };
    output.push(current);
    for sample in samples.iter().skip(1) {
        let target = MotionPoint {
            x: sample.x,
            y: sample.y,
            zoom: intensity_zoom(profile.intensity.clone()).min(profile.max_zoom.clamp(1.0, 2.0)),
        };
        current = smooth_motion(current, target, config);
        output.push(current);
    }
    output
}

pub fn evaluate_metrics(samples: &[CursorSample], path: &[MotionPoint]) -> MotionMetrics {
    if samples.len() < 2 || path.len() < 2 {
        return MotionMetrics {
            transition_latency_ms: 0,
            idle_jitter_ratio: 0.0,
        };
    }

    let mut transition_latency_ms = 0;
    let start = samples[0];
    let target = samples.last().copied().unwrap_or(start);
    let total_dx = (target.x - start.x).abs();
    let total_dy = (target.y - start.y).abs();
    let has_x_motion = total_dx >= 1.0;
    let has_y_motion = total_dy >= 1.0;
    let axis_count = usize::from(has_x_motion) + usize::from(has_y_motion);
    for (index, point) in path.iter().enumerate() {
        let progress_x = if has_x_motion {
            ((point.x - start.x).abs() / total_dx).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let progress_y = if has_y_motion {
            ((point.y - start.y).abs() / total_dy).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let weighted_progress = if axis_count == 0 {
            0.0
        } else {
            (progress_x + progress_y) / axis_count as f32
        };
        if weighted_progress >= 0.75 {
            transition_latency_ms = samples[index].t_ms.saturating_sub(start.t_ms);
            break;
        }
    }
    if transition_latency_ms == 0 && (has_x_motion || has_y_motion) {
        transition_latency_ms = target.t_ms.saturating_sub(start.t_ms);
    }

    let tail = path.iter().rev().take(10).copied().collect::<Vec<_>>();
    let center_x = tail.iter().map(|point| point.x).sum::<f32>() / tail.len() as f32;
    let jitter = tail
        .iter()
        .map(|point| (point.x - center_x).abs())
        .sum::<f32>()
        / tail.len() as f32;
    let idle_jitter_ratio = if center_x.abs() < f32::EPSILON {
        0.0
    } else {
        jitter / center_x.abs()
    };

    MotionMetrics {
        transition_latency_ms,
        idle_jitter_ratio,
    }
}

fn profile_to_config(profile: &CameraMotionProfile) -> MotionConfig {
    let base = match profile.intensity {
        CameraIntensity::Low => MotionConfig {
            smoothing: 0.72,
            max_speed_px: 120.0,
            max_zoom_step: 0.05,
        },
        CameraIntensity::Medium => MotionConfig {
            smoothing: 0.56,
            max_speed_px: 260.0,
            max_zoom_step: 0.1,
        },
        CameraIntensity::High => MotionConfig {
            smoothing: 0.42,
            max_speed_px: 360.0,
            max_zoom_step: 0.14,
        },
    };
    MotionConfig {
        // 保留 intensity 的默认手感，同时让评估能真实反映用户 smoothing 调整。
        smoothing: ((base.smoothing + profile.smoothing) * 0.5).clamp(0.0, 1.0),
        max_speed_px: base.max_speed_px,
        max_zoom_step: base.max_zoom_step,
    }
}

fn intensity_zoom(intensity: CameraIntensity) -> f32 {
    match intensity {
        CameraIntensity::Low => 1.03,
        CameraIntensity::Medium => 1.08,
        CameraIntensity::High => 1.14,
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_motion_path, evaluate_metrics, CursorSample};
    use crate::domain::models::{CameraIntensity, CameraMotionProfile};

    fn profile(intensity: CameraIntensity) -> CameraMotionProfile {
        CameraMotionProfile {
            enabled: true,
            intensity,
            smoothing: 0.56,
            max_zoom: 1.35,
            idle_threshold_ms: 500,
        }
    }

    #[test]
    fn motion_path_reacts_within_mvp_latency_band() {
        let samples = vec![
            CursorSample {
                t_ms: 0,
                x: 100.0,
                y: 100.0,
            },
            CursorSample {
                t_ms: 120,
                x: 900.0,
                y: 520.0,
            },
            CursorSample {
                t_ms: 240,
                x: 900.0,
                y: 520.0,
            },
            CursorSample {
                t_ms: 360,
                x: 900.0,
                y: 520.0,
            },
            CursorSample {
                t_ms: 480,
                x: 900.0,
                y: 520.0,
            },
        ];
        let path = compute_motion_path(&samples, &profile(CameraIntensity::Medium));
        let metrics = evaluate_metrics(&samples, &path);
        assert!(metrics.transition_latency_ms <= 450);
    }

    #[test]
    fn idle_jitter_is_small() {
        let samples = (0..20)
            .map(|idx| CursorSample {
                t_ms: idx * 50,
                x: 500.0,
                y: 300.0,
            })
            .collect::<Vec<_>>();
        let path = compute_motion_path(&samples, &profile(CameraIntensity::Low));
        let metrics = evaluate_metrics(&samples, &path);
        assert!(metrics.idle_jitter_ratio <= 0.01);
    }

    #[test]
    fn transition_latency_should_handle_single_axis_movement() {
        let samples = vec![
            CursorSample {
                t_ms: 0,
                x: 100.0,
                y: 320.0,
            },
            CursorSample {
                t_ms: 120,
                x: 860.0,
                y: 320.0,
            },
            CursorSample {
                t_ms: 240,
                x: 860.0,
                y: 320.0,
            },
            CursorSample {
                t_ms: 360,
                x: 860.0,
                y: 320.0,
            },
        ];
        let path = compute_motion_path(&samples, &profile(CameraIntensity::Medium));
        let metrics = evaluate_metrics(&samples, &path);
        assert!(metrics.transition_latency_ms > 0);
    }

    #[test]
    fn higher_smoothing_should_move_slower() {
        let samples = vec![
            CursorSample {
                t_ms: 0,
                x: 100.0,
                y: 100.0,
            },
            CursorSample {
                t_ms: 120,
                x: 900.0,
                y: 520.0,
            },
        ];
        let mut slow = profile(CameraIntensity::Medium);
        slow.smoothing = 0.9;
        let mut fast = profile(CameraIntensity::Medium);
        fast.smoothing = 0.2;
        let slow_path = compute_motion_path(&samples, &slow);
        let fast_path = compute_motion_path(&samples, &fast);
        let slow_dx = (slow_path[1].x - samples[0].x).abs();
        let fast_dx = (fast_path[1].x - samples[0].x).abs();
        assert!(fast_dx > slow_dx);
    }
}
