use serde::{Deserialize, Serialize};
use std::process::Command;

#[tauri::command]
pub fn read_subtitle_file(path: String) -> Result<String, String> {
    srt_parser::encoding::read_text_auto(&path).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub license: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "vesta".to_string(),
        license: env!("CARGO_PKG_LICENSE").to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemDiagnostics {
    pub os: String,
    pub arch: String,
    pub compute_backend: String,
    pub gpu_compiled: bool,
    pub gpu_device: Option<String>,
    pub ffmpeg_available: bool,
    pub video_encoder: String,
    pub hardware_video_encoder: bool,
    pub gstreamer_available: bool,
    pub gstreamer_h264: bool,
    pub gstreamer_h265: bool,
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn vulkan_device_name(summary: &str) -> Option<String> {
    summary.lines().find_map(|line| {
        let (_, value) = line.split_once("deviceName")?;
        let value = value.trim().trim_start_matches('=').trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn detect_gpu_device(backend: &str) -> Option<String> {
    if backend != "vulkan" {
        return None;
    }
    let output = Command::new("vulkaninfo").arg("--summary").output().ok()?;
    output
        .status
        .success()
        .then(|| vulkan_device_name(&String::from_utf8_lossy(&output.stdout)))
        .flatten()
}

#[tauri::command]
pub async fn get_system_diagnostics() -> SystemDiagnostics {
    let gpu_compiled = srt_transcribe::gpu_supported();
    let gpu_backend = srt_transcribe::gpu_backend_name();
    // Process probes run off the async executor and independently of FFmpeg.
    let gpu = tokio::task::spawn_blocking(move || detect_gpu_device(gpu_backend));
    let gst = tokio::task::spawn_blocking(|| command_succeeds("gst-inspect-1.0", &["--version"]));
    let h264 = tokio::task::spawn_blocking(|| command_succeeds("gst-inspect-1.0", &["avdec_h264"]));
    let h265 = tokio::task::spawn_blocking(|| command_succeeds("gst-inspect-1.0", &["avdec_h265"]));
    let media = async {
        let available = srt_flashcards::media::check_ffmpeg("ffmpeg").await;
        let encoder = if available {
            srt_flashcards::media::detect_h264_encoder("ffmpeg").await
        } else {
            srt_flashcards::media::H264Encoder::Libx264
        };
        (available, encoder)
    };
    let (gpu_device, gst, h264, h265, (ffmpeg_available, encoder)) =
        tokio::join!(gpu, gst, h264, h265, media);
    let gpu_device = gpu_device.unwrap_or(None);

    SystemDiagnostics {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        compute_backend: if gpu_compiled {
            gpu_backend.to_string()
        } else {
            "cpu".to_string()
        },
        gpu_compiled,
        gpu_device,
        ffmpeg_available,
        video_encoder: encoder.label().to_string(),
        hardware_video_encoder: ffmpeg_available && encoder.is_hardware(),
        gstreamer_available: gst.unwrap_or(false),
        gstreamer_h264: h264.unwrap_or(false),
        gstreamer_h265: h265.unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::vulkan_device_name;

    #[test]
    fn parses_vulkan_device_name() {
        let summary = "GPU0:\n\tdeviceName         = AMD Radeon RX 7800 XT\n";
        assert_eq!(
            vulkan_device_name(summary).as_deref(),
            Some("AMD Radeon RX 7800 XT")
        );
    }

    #[test]
    fn ignores_summaries_without_a_device() {
        assert_eq!(vulkan_device_name("Vulkan Instance Version: 1.3"), None);
    }
}
