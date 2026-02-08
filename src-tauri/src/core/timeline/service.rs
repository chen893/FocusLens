use crate::domain::models::{ProjectManifest, TimelinePatch};
use chrono::Utc;

pub fn apply_timeline_patch(manifest: &mut ProjectManifest, patch: TimelinePatch) {
    if let Some(trim_start_ms) = patch.trim_start_ms {
        manifest.timeline.trim_start_ms = trim_start_ms;
    }
    if let Some(trim_end_ms) = patch.trim_end_ms {
        manifest.timeline.trim_end_ms = trim_end_ms;
    }
    if let Some(aspect_ratio) = patch.aspect_ratio {
        manifest.timeline.aspect_ratio = aspect_ratio;
    }
    if let Some(cursor_highlight_enabled) = patch.cursor_highlight_enabled {
        manifest.timeline.cursor_highlight_enabled = cursor_highlight_enabled;
    }
    manifest.updated_at = Utc::now();
}
