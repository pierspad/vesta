//! On-demand "worker" binaries for GPU backends that aren't linked into the
//! launcher itself.
//!
//! The launcher only ships CPU+Vulkan in-process (see `Cargo.toml`). CUDA,
//! ROCm and SYCL each need their own toolchain at *compile* time and — per
//! a known whisper.cpp/ggml build-system limitation — ROCm can't safely
//! share a compile with CUDA in the same binary (it overrides the whole
//! project's C/C++ compiler). Rather than gamble on a single binary that
//! links all of them, each backend stays a separate build, exactly like
//! today: what changes is that build is `cli/srt-transcribe-cli` (the exact
//! CLI/pipeline the main Vesta app uses, so the benchmark can never drift
//! from what users actually get) instead of a whole extra Tauri app bundle.
//!
//! At runtime the launcher detects matching hardware (`hardware.rs`),
//! downloads the matching worker binary from the same GitHub release the
//! launcher itself was published from, sha256-verifies it, caches it in the
//! app data dir, and runs it as a subprocess with `--json` so its output
//! slots into the same progress/result events as the in-process variants.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use futures::StreamExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use srt_transcribe::pipeline::{ProgressUpdate, TranscriptionConfig, TranscriptionOutcome};

const RELEASE_BASE: &str =
    "https://github.com/pierspad/vesta/releases/download/whisper-bench-latest";

#[derive(Clone, Copy)]
pub struct Backend {
    pub id: &'static str,
    #[allow(dead_code)]
    pub display: &'static str,
}

/// GPU backends the launcher can fetch a worker for. Order is also the
/// order they're attempted in.
pub const BACKENDS: &[Backend] = &[
    Backend {
        id: "cuda",
        display: "CUDA",
    },
    Backend {
        id: "rocm",
        display: "ROCm",
    },
    Backend {
        id: "sycl",
        display: "SYCL (Intel)",
    },
];

/// ROCm on Windows is a known upstream build limitation (whisper.cpp#2202:
/// ROCm 5.7's hipBLAS doesn't link on Windows) — CI never publishes a
/// Windows ROCm worker, so don't try to fetch one.
pub fn applies_to_this_os(backend_id: &str) -> bool {
    !(backend_id == "rocm" && cfg!(windows))
}

pub fn hardware_wants(backend_id: &str, hw: &crate::hardware::DetectedHardware) -> bool {
    match backend_id {
        "cuda" => hw.nvidia,
        "rocm" => hw.amd,
        "sycl" => hw.intel_gpu,
        _ => false,
    }
}

fn os_slug() -> &'static str {
    if cfg!(windows) { "windows" } else { "linux" }
}

fn asset_name(backend_id: &str) -> String {
    let ext = if cfg!(windows) { ".exe" } else { "" };
    format!("srt-transcribe-{backend_id}-{}{ext}", os_slug())
}

fn cached_path(app: &AppHandle, backend_id: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?
        .join("workers");
    let file = if cfg!(windows) {
        format!("{backend_id}.exe")
    } else {
        backend_id.to_string()
    };
    Ok(dir.join(file))
}

/// Ensure the worker binary for `backend_id` is present locally, downloading
/// and sha256-verifying it from the release on first use.
pub async fn ensure_worker(
    app: &AppHandle,
    backend_id: &str,
    log: &impl Fn(String),
) -> Result<PathBuf, String> {
    let dest = cached_path(app, backend_id)?;
    if dest.exists() {
        return Ok(dest);
    }
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;

    let asset = asset_name(backend_id);
    let bin_url = format!("{RELEASE_BASE}/{asset}");
    let sha_url = format!("{bin_url}.sha256");

    log(format!("Downloading {backend_id} worker ({asset})..."));
    let expected_sha = fetch_text(&sha_url)
        .await
        .ok()
        .and_then(|s| s.split_whitespace().next().map(str::to_string));

    let bytes = download_with_progress(&bin_url, log).await?;

    if let Some(expected) = &expected_sha {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        // `finalize()`'s output type doesn't implement `LowerHex` in the
        // resolved sha2/crypto-common version, so hex-encode by hand.
        let actual = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if &actual != expected {
            return Err(format!(
                "Checksum mismatch for {asset}: expected {expected}, got {actual}. \
                 The download may be corrupt or the release incomplete — try again."
            ));
        }
        log("Checksum OK.".to_string());
    } else {
        log("No published checksum for this asset, skipping verification.".to_string());
    }

    let tmp = dest.with_extension("part");
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp, perms).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    log(format!("{backend_id} worker ready: {}", dest.display()));
    Ok(dest)
}

async fn fetch_text(url: &str) -> Result<String, String> {
    reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

async fn download_with_progress(url: &str, log: &impl Fn(String)) -> Result<Vec<u8>, String> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to reach {url}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Download failed for {url}: {e}"))?;
    let total = response.content_length();
    let mut downloaded: u64 = 0;
    let mut last_decile = -1i64;
    let mut buf = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        buf.extend_from_slice(&chunk);
        if let Some(total) = total.filter(|t| *t > 0) {
            let decile = (downloaded * 10 / total) as i64;
            if decile != last_decile {
                last_decile = decile;
                log(format!("  {}%", (decile * 10).min(100)));
            }
        }
    }
    Ok(buf)
}

/// One JSON line from a worker's stdout: either a progress update or the
/// final result (see `run_json` in `cli/srt-transcribe-cli`).
#[derive(Deserialize)]
#[serde(untagged)]
enum WorkerLine {
    Progress(ProgressUpdate),
    Result {
        ok: bool,
        outcome: Option<TranscriptionOutcome>,
        error: Option<String>,
    },
}

/// Run one variant through a worker binary, streaming its progress into
/// `on_progress` exactly like the in-process pipeline would.
pub async fn run_worker_variant(
    worker_path: &Path,
    ffmpeg: &str,
    config: &TranscriptionConfig,
    on_progress: impl Fn(ProgressUpdate),
) -> Result<TranscriptionOutcome, String> {
    let mut cmd = tokio::process::Command::new(worker_path);
    cmd.arg("run")
        .arg(&config.input_path)
        .arg("--output")
        .arg(&config.output_path)
        .arg("--model")
        .arg(&config.model)
        .arg("--language")
        .arg(&config.language)
        .arg("--ffmpeg")
        .arg(ffmpeg)
        .arg("--gpu")
        .arg("--json")
        .stdout(Stdio::piped())
        // Discarded rather than piped-and-ignored: an unread stderr pipe
        // can fill its OS buffer and deadlock the child if whisper.cpp's
        // own (stderr) logging gets chatty, since nothing here would be
        // draining it while we block reading stdout lines.
        .stderr(Stdio::null());
    if config.vad {
        cmd.arg("--vad");
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to launch worker {}: {e}", worker_path.display()))?;
    let stdout = child.stdout.take().expect("piped stdout");
    let mut lines = BufReader::new(stdout).lines();

    let mut final_result: Option<Result<TranscriptionOutcome, String>> = None;
    while let Ok(Some(line)) = lines.next_line().await {
        match serde_json::from_str::<WorkerLine>(&line) {
            Ok(WorkerLine::Progress(update)) => on_progress(update),
            Ok(WorkerLine::Result { ok, outcome, error }) => {
                final_result = Some(if ok {
                    outcome.ok_or_else(|| "worker reported ok with no outcome".to_string())
                } else {
                    Err(error.unwrap_or_else(|| "worker reported an unspecified error".into()))
                });
            }
            Err(_) => {
                // Non-JSON noise on stdout (shouldn't normally happen in
                // --json mode) — ignore rather than fail the whole variant.
            }
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Worker process error: {e}"))?;

    match final_result {
        Some(result) => result,
        None if status.success() => Err("Worker exited without reporting a result".to_string()),
        None => Err(format!("Worker exited with {status} and no result line")),
    }
}
