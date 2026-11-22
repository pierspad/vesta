use serde::{Deserialize, Serialize};
use std::{process::Stdio, time::Duration};
use tokio::process::Command;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[tauri::command]
pub fn read_subtitle_file(path: String) -> Result<String, String> {
    vesta_parser::encoding::read_text_auto(&path).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubtitleEntry {
    pub id: u32,
    pub start: String,
    pub end: String,
    pub text: String,
}

#[tauri::command]
pub async fn parse_subtitle_file(path: String) -> Result<Vec<SubtitleEntry>, String> {
    tokio::task::spawn_blocking(move || {
        let map = vesta_parser::SrtParser::parse_file(&path).map_err(|e| e.to_string())?;
        let mut entries: Vec<SubtitleEntry> = map
            .into_values()
            .map(|sub| SubtitleEntry {
                id: sub.id,
                start: sub.start.to_srt_string(),
                end: sub.end.to_srt_string(),
                text: sub.text,
            })
            .collect();
        entries.sort_by_key(|entry| entry.id);
        Ok(entries)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_subtitle_file(path: String, entries: Vec<SubtitleEntry>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let mut map = std::collections::HashMap::with_capacity(entries.len());
        for entry in entries {
            let start = vesta_parser::Timestamp::from_srt_string(&entry.start)
                .map_err(|e| format!("Invalid start timestamp for cue {}: {e}", entry.id))?;
            let end = vesta_parser::Timestamp::from_srt_string(&entry.end)
                .map_err(|e| format!("Invalid end timestamp for cue {}: {e}", entry.id))?;
            map.insert(
                entry.id,
                vesta_parser::Subtitle {
                    id: entry.id,
                    start,
                    end,
                    text: entry.text,
                },
            );
        }
        vesta_parser::SrtParser::save_file(&path, &map).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpCliInfo {
    pub command: String,
    pub bundled: bool,
    pub default_read_root: String,
    pub default_output_root: String,
}

fn default_mcp_roots() -> (String, String) {
    let read_fallback = if cfg!(windows) {
        r"C:\Users\Public\Videos"
    } else {
        "/path/to/media"
    };
    let output_fallback = if cfg!(windows) {
        r"C:\Users\Public\Documents"
    } else {
        "/path/to/exports"
    };
    let read_root = dirs::video_dir()
        .or_else(dirs::home_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| read_fallback.to_string());
    let output_root = dirs::document_dir()
        .or_else(dirs::home_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| output_fallback.to_string());
    (read_root, output_root)
}

pub(crate) fn resolve_mcp_cli_info_from(
    exe_path: Option<&std::path::Path>,
    path_env: Option<&std::ffi::OsStr>,
) -> McpCliInfo {
    let bin_name = if cfg!(windows) {
        "vesta-cli.exe"
    } else {
        "vesta-cli"
    };
    let (default_read_root, default_output_root) = default_mcp_roots();

    if let Some(sibling) = exe_path
        .and_then(|p| p.parent())
        .map(|dir| dir.join(bin_name))
        .filter(|candidate| candidate.is_file())
    {
        return McpCliInfo {
            command: sibling.to_string_lossy().into_owned(),
            bundled: true,
            default_read_root,
            default_output_root,
        };
    }

    if let Some(paths) = path_env {
        for dir in std::env::split_paths(paths) {
            let candidate = dir.join(bin_name);
            if candidate.is_file() {
                return McpCliInfo {
                    command: candidate.to_string_lossy().into_owned(),
                    bundled: true,
                    default_read_root,
                    default_output_root,
                };
            }
        }
    }

    McpCliInfo {
        command: bin_name.to_string(),
        bundled: false,
        default_read_root,
        default_output_root,
    }
}

#[tauri::command]
pub fn get_mcp_cli_info() -> McpCliInfo {
    let exe_path = std::env::current_exe().ok();
    let path_env = std::env::var_os("PATH");
    resolve_mcp_cli_info_from(exe_path.as_deref(), path_env.as_deref())
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
    let gpu_compiled = vesta_transcribe::gpu_supported();
    let gpu_backend = vesta_transcribe::gpu_backend_name();
    // Async probes have bounded lifetimes and kill their child when dropped.
    let gpu = detect_gpu_device(gpu_backend);
    let gst = command_succeeds("gst-inspect-1.0", &["--version"]);
    let h264 = command_succeeds("gst-inspect-1.0", &["avdec_h264"]);
    let h265 = command_succeeds("gst-inspect-1.0", &["avdec_h265"]);
    let ffmpeg_cmd = crate::commands::media::resolve_ffmpeg_path(Some(&app)).await;
    let media = async move {
        let available = vesta_flashcards::media::check_ffmpeg(&ffmpeg_cmd).await;
        let encoder = if available {
            vesta_flashcards::media::detect_h264_encoder(&ffmpeg_cmd).await
        } else {
            vesta_flashcards::media::H264Encoder::Libx264
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

    #[tokio::test]
    async fn parse_and_save_subtitle_file_roundtrip_and_validate_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        let srt_path = dir.path().join("sample.srt");
        let entries = vec![
            super::SubtitleEntry {
                id: 1,
                start: "00:00:01,000".to_string(),
                end: "00:00:02,500".to_string(),
                text: "First line\nSecond line".to_string(),
            },
            super::SubtitleEntry {
                id: 2,
                start: "00:00:03,000".to_string(),
                end: "00:00:04,250".to_string(),
                text: "Third line".to_string(),
            },
        ];

        super::save_subtitle_file(srt_path.to_string_lossy().into_owned(), entries.clone())
            .await
            .unwrap();
        let parsed = super::parse_subtitle_file(srt_path.to_string_lossy().into_owned())
            .await
            .unwrap();
        assert_eq!(parsed, entries);

        let err = super::save_subtitle_file(
            srt_path.to_string_lossy().into_owned(),
            vec![super::SubtitleEntry {
                id: 1,
                start: "not-a-timestamp".to_string(),
                end: "00:00:02,000".to_string(),
                text: "Bad".to_string(),
            }],
        )
        .await
        .unwrap_err();
        assert!(err.contains("Invalid start timestamp"));
    }

    #[test]
    fn mcp_cli_info_prefers_sibling_then_path_then_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let app_dir = dir.path().join("app");
        let path_dir = dir.path().join("path");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::create_dir_all(&path_dir).unwrap();

        let bin_name = if cfg!(windows) {
            "vesta-cli.exe"
        } else {
            "vesta-cli"
        };
        let app_exe = app_dir.join(if cfg!(windows) { "vesta.exe" } else { "vesta" });
        let sibling_cli = app_dir.join(bin_name);
        let path_cli = path_dir.join(bin_name);

        let fallback = super::resolve_mcp_cli_info_from(Some(&app_exe), Some(path_dir.as_os_str()));
        assert!(!fallback.bundled);
        assert_eq!(fallback.command, bin_name);
        assert!(!fallback.default_read_root.is_empty());
        assert!(!fallback.default_output_root.is_empty());

        std::fs::write(&path_cli, b"path-cli").unwrap();
        let from_path =
            super::resolve_mcp_cli_info_from(Some(&app_exe), Some(path_dir.as_os_str()));
        assert!(from_path.bundled);
        assert_eq!(from_path.command, path_cli.to_string_lossy());

        std::fs::write(&sibling_cli, b"sibling-cli").unwrap();
        let from_sibling =
            super::resolve_mcp_cli_info_from(Some(&app_exe), Some(path_dir.as_os_str()));
        assert!(from_sibling.bundled);
        assert_eq!(from_sibling.command, sibling_cli.to_string_lossy());
    }
}
