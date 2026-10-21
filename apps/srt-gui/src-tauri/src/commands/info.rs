use serde::{Deserialize, Serialize};
use std::{process::Stdio, time::Duration};
use tokio::process::Command;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

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

async fn command_succeeds(program: &str, args: &[&str]) -> bool {
    let mut command = Command::new(program);
    command
        .args(args)
        .kill_on_drop(true)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    tokio::time::timeout(PROBE_TIMEOUT, command.status())
        .await
        .is_ok_and(|result| result.is_ok_and(|status| status.success()))
}

fn vulkan_device_name(summary: &str) -> Option<String> {
    summary.lines().find_map(|line| {
        let (_, value) = line.split_once("deviceName")?;
        let value = value.trim().trim_start_matches('=').trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

async fn detect_gpu_device(backend: &str) -> Option<String> {
    if backend != "vulkan" {
        return None;
    }
    let mut command = Command::new("vulkaninfo");
    command
        .arg("--summary")
        .kill_on_drop(true)
        .stderr(Stdio::null());
    let output = tokio::time::timeout(PROBE_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()?;
    output
        .status
        .success()
        .then(|| vulkan_device_name(&String::from_utf8_lossy(&output.stdout)))
        .flatten()
}

#[tauri::command]
pub async fn get_system_diagnostics(app: tauri::AppHandle) -> SystemDiagnostics {
    let gpu_compiled = srt_transcribe::gpu_supported();
    let gpu_backend = srt_transcribe::gpu_backend_name();
    // Async probes have bounded lifetimes and kill their child when dropped.
    let gpu = detect_gpu_device(gpu_backend);
    let gst = command_succeeds("gst-inspect-1.0", &["--version"]);
    let h264 = command_succeeds("gst-inspect-1.0", &["avdec_h264"]);
    let h265 = command_succeeds("gst-inspect-1.0", &["avdec_h265"]);
    let ffmpeg_cmd = crate::commands::media::resolve_ffmpeg_path(Some(&app)).await;
    let media = async move {
        let available = srt_flashcards::media::check_ffmpeg(&ffmpeg_cmd).await;
        let encoder = if available {
            srt_flashcards::media::detect_h264_encoder(&ffmpeg_cmd).await
        } else {
            srt_flashcards::media::H264Encoder::Libx264
        };
        (available, encoder)
    };
    let (gpu_device, gst, h264, h265, (ffmpeg_available, encoder)) =
        tokio::join!(gpu, gst, h264, h265, media);

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
        gstreamer_available: gst,
        gstreamer_h264: h264,
        gstreamer_h265: h265,
    }
}

#[cfg(test)]
mod tests {
    use super::vulkan_device_name;
    #[cfg(target_os = "linux")]
    use super::{Duration, command_succeeds};

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn probes_handle_success_failure_and_kill_timed_out_children() {
        assert!(command_succeeds("sh", &["-c", "exit 0"]).await);
        assert!(!command_succeeds("sh", &["-c", "exit 1"]).await);
        assert!(!command_succeeds("/nonexistent-vesta-probe", &[]).await);
        let directory = tempfile::tempdir().unwrap();
        let pid_file = directory.path().join("pid");
        let path = pid_file.to_str().unwrap();
        // exec keeps the PID written by the shell, so the timeout must kill sleep.
        let result = tokio::time::timeout(
            Duration::from_secs(7),
            command_succeeds(
                "sh",
                &["-c", "echo $$ > \"$1\"; exec sleep 30", "probe", path],
            ),
        )
        .await
        .expect("probe did not respect its deadline");
        assert!(!result);
        let pid = std::fs::read_to_string(pid_file).unwrap();
        for _ in 0..100 {
            if !std::path::Path::new(&format!("/proc/{}", pid.trim())).exists() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out child is still alive");
    }

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
