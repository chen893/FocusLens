pub fn parse_drop_rates(ffmpeg_stderr: &str) -> (f32, f32) {
    let mut rates = Vec::new();
    for line in ffmpeg_stderr.lines() {
        let frame = extract_numeric(line, "frame=");
        let drop = extract_numeric(line, "drop=");
        let Some(drop) = drop else {
            continue;
        };
        let rate = if let Some(frame) = frame {
            if frame > 0.0 {
                (drop / frame) * 100.0
            } else {
                0.0
            }
        } else {
            // 部分 ffmpeg 日志直接给出 drop 百分比，保留原值。
            drop
        };
        rates.push(rate);
    }

    if rates.is_empty() {
        return (0.0, 0.0);
    }
    let sum: f32 = rates.iter().sum();
    let avg = sum / rates.len() as f32;
    let peak = rates.iter().copied().fold(0.0, f32::max);
    (avg, peak)
}

fn extract_numeric(line: &str, key: &str) -> Option<f32> {
    let index = line.find(key)?;
    let segment = &line[index + key.len()..];
    let token = segment.split_whitespace().next()?;
    token.parse::<f32>().ok()
}

#[cfg(test)]
mod tests {
    use super::parse_drop_rates;

    #[test]
    fn parse_drop_rate_from_frame_and_drop_count() {
        let log = "frame=100 fps=30 drop=1\nframe=200 fps=30 drop=4";
        let (avg, peak) = parse_drop_rates(log);
        assert!((avg - 1.5).abs() < 0.01);
        assert!((peak - 2.0).abs() < 0.01);
    }

    #[test]
    fn parse_direct_drop_percentage() {
        let log = "drop=1.2\n drop=3.4";
        let (avg, peak) = parse_drop_rates(log);
        assert!((avg - 2.3).abs() < 0.01);
        assert!((peak - 3.4).abs() < 0.01);
    }
}
