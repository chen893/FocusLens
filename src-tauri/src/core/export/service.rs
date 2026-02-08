use crate::domain::models::ExportProgressEvent;
use crate::infra::ffmpeg::capabilities::HardwareEncoderAvailability;

pub fn planned_progress(
    task_id: &str,
    hw_encoder: HardwareEncoderAvailability,
) -> Vec<ExportProgressEvent> {
    let mut events = vec![
        ExportProgressEvent {
            task_id: task_id.to_string(),
            status: "queued".to_string(),
            progress: 0,
            detail: "导出任务排队中".to_string(),
        },
        ExportProgressEvent {
            task_id: task_id.to_string(),
            status: "running".to_string(),
            progress: 20,
            detail: "正在解析项目配置".to_string(),
        },
        ExportProgressEvent {
            task_id: task_id.to_string(),
            status: "running".to_string(),
            progress: 50,
            detail: "正在编码视频流".to_string(),
        },
    ];

    if !hw_encoder.available {
        events.push(ExportProgressEvent {
            task_id: task_id.to_string(),
            status: "fallback".to_string(),
            progress: 62,
            detail: format!("硬件编码({})不可用，已自动回退到软件编码", hw_encoder.codec),
        });
    }

    events.extend([
        ExportProgressEvent {
            task_id: task_id.to_string(),
            status: "running".to_string(),
            progress: 85,
            detail: "正在封装 MP4".to_string(),
        },
        ExportProgressEvent {
            task_id: task_id.to_string(),
            status: "success".to_string(),
            progress: 100,
            detail: "导出完成".to_string(),
        },
    ]);
    events
}
