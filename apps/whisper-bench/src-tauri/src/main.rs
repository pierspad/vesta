// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

use srt_transcribe::pipeline::{PipelineCallbacks, TranscriptionConfig, transcribe_to_srt};
use srt_transcribe::{gpu_backend_name, gpu_supported, model};

// ─── Event helpers ──────────────────────────────────────────────────────────

fn log(app: &AppHandle, line: impl Into<String>) {
    let line = line.into();
    let _ = app.emit("bench-log", line);
}

// ─── Info ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct BenchInfo {
    app_version: String,
    gpu_backend: &'static str,
    gpu_supported: bool,
    default_threads: usize,
    physical_cores: usize,
    models: Vec<model::WhisperModelInfo>,
    vad_installed: bool,
    os: &'static str,
}

#[tauri::command]
fn bench_info() -> Result<BenchInfo, String> {
    Ok(BenchInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        gpu_backend: gpu_backend_name(),
        gpu_supported: gpu_supported(),
        default_threads: srt_transcribe::transcribe::default_n_threads(),
        physical_cores: num_cpus::get_physical(),
        models: model::list_models().map_err(|e| e.to_string())?,
        vad_installed: model::vad_model_installed(model::DEFAULT_VAD_MODEL_ID),
        os: std::env::consts::OS,
    })
}

// ─── File pickers ───────────────────────────────────────────────────────────

#[tauri::command]
async fn pick_media(app: AppHandle) -> Result<Option<String>, String> {
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter(
                "Media",
                &[
                    "mp4", "mkv", "avi", "webm", "mp3", "wav", "m4a", "flac", "ogg", "opus",
                ],
            )
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(file.map(|f| f.to_string()))
}

#[tauri::command]
async fn save_csv(app: AppHandle, csv: String) -> Result<Option<String>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name("whisper-bench-results.csv")
            .add_filter("CSV", &["csv"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.to_string();
    std::fs::write(&path, csv).map_err(|e| format!("Failed to write CSV: {e}"))?;
    Ok(Some(path))
}

// ─── Asset preparation (whisper model + VAD model + ffmpeg) ─────────────────

async fn command_works(cmd: &str) -> bool {
    tokio::process::Command::new(cmd)
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn hoist_binary(dir: &std::path::Path, name: &str) -> Result<(), String> {
    let file_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let target = dir.join(&file_name);
    if target.exists() {
        return Ok(());
    }

    fn find(dir: &std::path::Path, file_name: &str, depth: u8) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().is_some_and(|n| n == file_name) {
                return Some(path);
            }
            if depth > 0
                && path.is_dir()
                && let Some(found) = find(&path, file_name, depth - 1)
            {
                return Some(found);
            }
        }
        None
    }

    let found = find(dir, &file_name, 3)
        .ok_or_else(|| format!("{file_name} not found in the downloaded ffmpeg archive"))?;
    std::fs::rename(&found, &target)
        .or_else(|_| std::fs::copy(&found, &target).map(|_| ()))
        .map_err(|e| format!("Failed to move {file_name} into place: {e}"))?;
    Ok(())
}

/// Resolve an ffmpeg command: system PATH first, then a static build
/// downloaded into the app's data dir (same approach as the main vesta app).
async fn ensure_ffmpeg(app: &AppHandle) -> Result<String, String> {
    if command_works("ffmpeg").await {
        return Ok("ffmpeg".to_string());
    }

    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let dest = app_data.join("ffmpeg_bin");
    let mut local = dest.join("ffmpeg");
    if cfg!(windows) {
        local.set_extension("exe");
    }
    if local.exists() {
        return Ok(local.to_string_lossy().into_owned());
    }

    log(
        app,
        "ffmpeg not found in PATH, downloading a static build...",
    );
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let dest_task = dest.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        use ffmpeg_sidecar::download::{
            download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg,
        };
        let url =
            ffmpeg_download_url().map_err(|e| format!("Could not determine download URL: {e}"))?;
        let archive = download_ffmpeg_package(url, &dest_task)
            .map_err(|e| format!("ffmpeg download failed: {e}"))?;
        unpack_ffmpeg(&archive, &dest_task).map_err(|e| format!("ffmpeg unpack failed: {e}"))?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    hoist_binary(&dest, "ffmpeg")?;
    let _ = hoist_binary(&dest, "ffprobe");
    Ok(local.to_string_lossy().into_owned())
}

#[tauri::command]
async fn prepare_assets(app: AppHandle, model_id: String) -> Result<(), String> {
    let ffmpeg = ensure_ffmpeg(&app).await?;
    log(&app, format!("ffmpeg: {ffmpeg}"));

    let model_path = model::model_file_path(&model_id).map_err(|e| e.to_string())?;
    if !model_path.exists() {
        log(&app, format!("Downloading Whisper model '{model_id}'..."));
        let app2 = app.clone();
        let mid = model_id.clone();
        model::download_model(
            &model_id,
            move |pct| {
                if pct % 10 == 0 {
                    log(&app2, format!("  model {mid}: {pct}%"));
                }
            },
            None,
        )
        .await
        .map_err(|e| format!("Whisper model download failed: {e}"))?;
    }
    log(
        &app,
        format!("Whisper model ready: {}", model_path.display()),
    );

    if !model::vad_model_installed(model::DEFAULT_VAD_MODEL_ID) {
        log(&app, "Downloading Silero VAD model...");
        let app2 = app.clone();
        model::download_vad_model(
            model::DEFAULT_VAD_MODEL_ID,
            move |pct| {
                if pct % 25 == 0 {
                    log(&app2, format!("  vad: {pct}%"));
                }
            },
            None,
        )
        .await
        .map_err(|e| format!("VAD model download failed: {e}"))?;
    }
    log(&app, "VAD model ready.");
    Ok(())
}

// ─── Benchmark ──────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
struct BenchRow {
    variant: String,
    backend: String,
    vad: bool,
    model: String,
    wall_seconds: f64,
    audio_seconds: Option<f64>,
    realtime_factor: Option<f64>,
    subtitle_count: usize,
    detected_language: Option<String>,
    status: String,
    error: Option<String>,
}

async fn media_duration_seconds(input: &str) -> Option<f64> {
    let out = tokio::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            input,
        ])
        .output()
        .await
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn variant_configs(
    input: &str,
    model_id: &str,
    language: &str,
    out_dir: &std::path::Path,
) -> Vec<(String, TranscriptionConfig)> {
    // (label, use_gpu, vad)
    let mut variants: Vec<(&str, bool, bool)> =
        vec![("cpu", false, false), ("cpu+vad", false, true)];
    if gpu_supported() {
        variants.push((gpu_backend_name(), true, false));
        // Owned label like "vulkan+vad" needs a String; build below instead.
    }

    let mut out = Vec::new();
    for (label, use_gpu, vad) in variants {
        out.push((
            label.to_string(),
            make_config(input, model_id, language, out_dir, use_gpu, vad, label),
        ));
    }
    if gpu_supported() {
        let label = format!("{}+vad", gpu_backend_name());
        out.push((
            label.clone(),
            make_config(input, model_id, language, out_dir, true, true, &label),
        ));
    }
    out
}

fn make_config(
    input: &str,
    model_id: &str,
    language: &str,
    out_dir: &std::path::Path,
    use_gpu: bool,
    vad: bool,
    label: &str,
) -> TranscriptionConfig {
    TranscriptionConfig {
        input_path: input.to_string(),
        output_path: out_dir
            .join(format!("bench-{}.srt", label.replace('+', "-")))
            .to_string_lossy()
            .into_owned(),
        model: model_id.to_string(),
        language: language.to_string(),
        translate_to_english: false,
        word_timestamps: false,
        max_segment_length: 0,
        provider: None,
        api_key: None,
        api_url: None,
        quality: false,
        vad,
        vad_model_id: None,
        vad_custom_path: None,
        use_gpu,
    }
}

#[tauri::command]
async fn run_bench(
    app: AppHandle,
    input_path: String,
    model_id: String,
    language: String,
) -> Result<Vec<BenchRow>, String> {
    if !std::path::Path::new(&input_path).exists() {
        return Err(format!("Input file not found: {input_path}"));
    }
    let ffmpeg = ensure_ffmpeg(&app).await?;
    let out_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let audio_seconds = media_duration_seconds(&input_path).await;

    log(
        &app,
        format!(
            "Build backend: {} | gpu_supported: {} | threads: {}",
            gpu_backend_name(),
            gpu_supported(),
            srt_transcribe::transcribe::default_n_threads()
        ),
    );
    if let Some(s) = audio_seconds {
        log(&app, format!("Media duration: {s:.1}s"));
    }

    let variants = variant_configs(&input_path, &model_id, &language, out_dir.path());
    let total = variants.len();
    let mut rows = Vec::with_capacity(total);

    for (i, (label, config)) in variants.into_iter().enumerate() {
        log(
            &app,
            format!("── [{}/{}] variant '{label}' starting...", i + 1, total),
        );
        let _ = app.emit(
            "bench-progress",
            serde_json::json!({ "current": i + 1, "total": total, "variant": label }),
        );

        let cb_app = app.clone();
        let cb_label = label.clone();
        let callbacks = PipelineCallbacks {
            on_progress: Some(std::sync::Arc::new(move |p| {
                if p.percentage >= 0.0 {
                    let _ = cb_app.emit(
                        "bench-stage",
                        serde_json::json!({ "variant": cb_label, "stage": p.stage, "pct": p.percentage }),
                    );
                }
            })),
            on_segment: None,
        };

        let start = Instant::now();
        let result =
            transcribe_to_srt(&config, &ffmpeg, callbacks, &CancellationToken::new()).await;
        let wall = start.elapsed().as_secs_f64();

        let row = match result {
            Ok(outcome) => {
                log(
                    &app,
                    format!(
                        "   '{label}' OK in {wall:.1}s — {} subtitles{}",
                        outcome.subtitle_count,
                        outcome
                            .detected_language
                            .as_deref()
                            .map(|l| format!(", language: {l}"))
                            .unwrap_or_default()
                    ),
                );
                BenchRow {
                    variant: label.clone(),
                    backend: if config.use_gpu {
                        gpu_backend_name().into()
                    } else {
                        "cpu".into()
                    },
                    vad: config.vad,
                    model: model_id.clone(),
                    wall_seconds: wall,
                    audio_seconds,
                    realtime_factor: audio_seconds.map(|a| a / wall),
                    subtitle_count: outcome.subtitle_count,
                    detected_language: outcome.detected_language,
                    status: "ok".into(),
                    error: None,
                }
            }
            Err(e) => {
                log(&app, format!("   '{label}' FAILED after {wall:.1}s: {e:#}"));
                BenchRow {
                    variant: label.clone(),
                    backend: if config.use_gpu {
                        gpu_backend_name().into()
                    } else {
                        "cpu".into()
                    },
                    vad: config.vad,
                    model: model_id.clone(),
                    wall_seconds: wall,
                    audio_seconds,
                    realtime_factor: None,
                    subtitle_count: 0,
                    detected_language: None,
                    status: "error".into(),
                    error: Some(format!("{e:#}")),
                }
            }
        };
        rows.push(row);
    }

    log(&app, "Benchmark complete.");
    Ok(rows)
}

// ─── Entrypoint ─────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            bench_info,
            pick_media,
            save_csv,
            prepare_assets,
            run_bench
        ])
        .run(tauri::generate_context!())
        .expect("error while running whisper-bench");
}
