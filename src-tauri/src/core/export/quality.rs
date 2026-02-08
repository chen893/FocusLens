#[derive(Debug, Clone)]
pub struct QualityGateResult {
    pub passed: bool,
    pub reasons: Vec<String>,
}

pub fn validate_mvp_quality(
    av_offset_ms: i64,
    avg_drop_rate: f32,
    peak_drop_rate: f32,
) -> QualityGateResult {
    let mut reasons = Vec::new();
    if !avg_drop_rate.is_finite()
        || !peak_drop_rate.is_finite()
        || avg_drop_rate < 0.0
        || peak_drop_rate < 0.0
    {
        reasons.push("缺少有效掉帧率数据，请检查导出日志采集".to_string());
    }
    if av_offset_ms.abs() > 100 {
        reasons.push(format!("A/V 偏移超标: {av_offset_ms}ms (阈值 <=100ms)"));
    }
    if avg_drop_rate > 2.0 {
        reasons.push(format!("平均掉帧率超标: {avg_drop_rate:.2}% (阈值 <=2%)"));
    }
    if peak_drop_rate > 5.0 {
        reasons.push(format!("峰值掉帧率超标: {peak_drop_rate:.2}% (阈值 <=5%)"));
    }
    QualityGateResult {
        passed: reasons.is_empty(),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_mvp_quality;

    #[test]
    fn quality_gate_passes_when_all_metrics_in_range() {
        let result = validate_mvp_quality(80, 1.8, 4.9);
        assert!(result.passed);
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn quality_gate_fails_when_drop_metrics_missing() {
        let result = validate_mvp_quality(80, -1.0, -1.0);
        assert!(!result.passed);
        assert_eq!(result.reasons.len(), 1);
    }

    #[test]
    fn quality_gate_fails_when_metrics_out_of_range() {
        let result = validate_mvp_quality(180, 2.3, 6.2);
        assert!(!result.passed);
        assert_eq!(result.reasons.len(), 3);
    }
}
