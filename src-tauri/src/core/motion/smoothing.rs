#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionPoint {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct MotionConfig {
    pub smoothing: f32,
    pub max_speed_px: f32,
    pub max_zoom_step: f32,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            smoothing: 0.68,
            max_speed_px: 80.0,
            max_zoom_step: 0.08,
        }
    }
}

pub fn smooth_motion(prev: MotionPoint, target: MotionPoint, cfg: MotionConfig) -> MotionPoint {
    let alpha = cfg.smoothing.clamp(0.0, 1.0);
    let mut dx = (target.x - prev.x) * (1.0 - alpha);
    let mut dy = (target.y - prev.y) * (1.0 - alpha);
    let distance = (dx * dx + dy * dy).sqrt();
    if distance > cfg.max_speed_px {
        let ratio = cfg.max_speed_px / distance;
        dx *= ratio;
        dy *= ratio;
    }

    let mut dz = (target.zoom - prev.zoom) * (1.0 - alpha);
    if dz.abs() > cfg.max_zoom_step {
        dz = cfg.max_zoom_step * dz.signum();
    }

    MotionPoint {
        x: prev.x + dx,
        y: prev.y + dy,
        zoom: (prev.zoom + dz).clamp(1.0, 2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::{smooth_motion, MotionConfig, MotionPoint};

    #[test]
    fn large_jump_is_clamped() {
        let prev = MotionPoint {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        };
        let target = MotionPoint {
            x: 1000.0,
            y: 0.0,
            zoom: 2.0,
        };
        let cfg = MotionConfig {
            smoothing: 0.5,
            max_speed_px: 60.0,
            max_zoom_step: 0.1,
        };
        let next = smooth_motion(prev, target, cfg);
        assert!(next.x <= 60.0);
        assert_eq!(next.y, 0.0);
        assert!(next.zoom <= 1.1);
    }

    #[test]
    fn idle_point_stays_stable() {
        let prev = MotionPoint {
            x: 400.0,
            y: 300.0,
            zoom: 1.3,
        };
        let next = smooth_motion(prev, prev, MotionConfig::default());
        assert!((next.x - prev.x).abs() < f32::EPSILON);
        assert!((next.y - prev.y).abs() < f32::EPSILON);
        assert!((next.zoom - prev.zoom).abs() < f32::EPSILON);
    }
}
