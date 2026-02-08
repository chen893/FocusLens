#[derive(Debug, Clone)]
pub struct HardwareEncoderAvailability {
    pub available: bool,
    pub detail: String,
    pub codec: String,
}

pub fn detect_hardware_encoder() -> HardwareEncoderAvailability {
    let codec = preferred_codec().to_string();
    let output = std::process::Command::new(
        std::env::var("FOCUSLENS_FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string()),
    )
    .arg("-hide_banner")
    .arg("-encoders")
    .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        let available = output.status.success() && stdout.contains(&codec.to_lowercase());
        if available {
            return HardwareEncoderAvailability {
                available: true,
                detail: format!("detected hardware encoder: {codec}"),
                codec,
            };
        }
    }

    #[cfg(target_os = "windows")]
    {
        HardwareEncoderAvailability {
            available: false,
            detail: "windows: hardware encoder unavailable, fallback to software".to_string(),
            codec,
        }
    }
    #[cfg(target_os = "macos")]
    {
        HardwareEncoderAvailability {
            available: false,
            detail: "macos: hardware encoder unavailable, fallback to software".to_string(),
            codec,
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        HardwareEncoderAvailability {
            available: false,
            detail: "当前平台不在 MVP 支持范围，使用软件编码".to_string(),
            codec,
        }
    }
}

fn preferred_codec() -> &'static str {
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
