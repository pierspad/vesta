// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hardware;
mod sample;
mod workers;

use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

use hardware::DetectedHardware;
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
    hardware: DetectedHardware,
    /// Variant labels the app plans to attempt on this machine, in order —
    /// computed the same way `run_bench` will, so the UI can show it before
    /// the user even picks a file.
    planned_variants: Vec<String>,
}

#[tauri::command]
async fn bench_info() -> Result<BenchInfo, String> {
    let hw = hardware::detect().await;
    let planned_variants = variant_plan(&hw)
        .into_iter()
        .map(|(label, _)| label)
        .collect();
    Ok(BenchInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        gpu_backend: gpu_backend_name(),
        gpu_supported: gpu_supported(),
        default_threads: srt_transcribe::transcribe::default_n_threads(),
        physical_cores: num_cpus::get_physical(),
        models: model::list_models().map_err(|e| e.to_string())?,
        vad_installed: model::vad_model_installed(model::DEFAULT_VAD_MODEL_ID),
        os: std::env::consts::OS,
        hardware: hw,
        planned_variants,
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

/// Fetches (once) and extracts the bundled sample episode + reference SRTs,
/// for testers who don't have media of their own handy.
#[tauri::command]
async fn use_sample_media(app: AppHandle) -> Result<String, String> {
    let app2 = app.clone();
    let path = sample::ensure_sample_media(&app, move |line| log(&app2, line)).await?;
    Ok(path.to_string_lossy().into_owned())
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
    let progress_app = app.clone();
    let started = Instant::now();
    let heartbeat = tokio::spawn(async move {
        // The ffmpeg-sidecar download has no progress callback, so without
        // this the log would just sit on "downloading..." for however long
        // the connection takes — indistinguishable from a hang. A periodic
        // heartbeat at least proves the app is still alive and working.
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            log(
                &progress_app,
                format!(
                    "  ...still downloading ffmpeg ({}s elapsed)",
                    started.elapsed().as_secs()
                ),
            );
        }
    });
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
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
    .map_err(|e| e.to_string());
    heartbeat.abort();
    result??;

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

/// Which backend runs a variant: in-process (compiled into this launcher —
/// currently just CPU and, if a loader was detected, Vulkan) or a fetched
/// worker binary keyed by backend id ("cuda" | "rocm" | "sycl").
#[derive(Clone, Copy)]
enum VariantKind {
    InProcess,
    Worker(&'static str),
}

/// The list of variants this machine will attempt, independent of any
/// specific input file — same detection logic `bench_info` previews and
/// `run_bench` executes.
fn variant_plan(hw: &DetectedHardware) -> Vec<(String, VariantKind)> {
    let mut out = vec![
        ("cpu".to_string(), VariantKind::InProcess),
        ("cpu+vad".to_string(), VariantKind::InProcess),
    ];

    // Vulkan is compiled into every launcher build; still gate on an actual
    // loader being present so a GPU-less machine doesn't get a confusing
    // guaranteed-to-fail "vulkan" row.
    if gpu_supported() && hw.vulkan_loader {
        let name = gpu_backend_name();
        out.push((name.to_string(), VariantKind::InProcess));
        out.push((format!("{name}+vad"), VariantKind::InProcess));
    }

    // CUDA/ROCm/SYCL are never compiled into the launcher (see Cargo.toml) —
    // each is a separate `srt-transcribe` worker binary fetched on demand
    // only when this OS supports it and matching hardware was detected.
    for backend in workers::BACKENDS {
        if !workers::applies_to_this_os(backend.id) || !workers::hardware_wants(backend.id, hw) {
            continue;
        }
        out.push((backend.id.to_string(), VariantKind::Worker(backend.id)));
        out.push((
            format!("{}+vad", backend.id),
            VariantKind::Worker(backend.id),
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

fn variant_configs(
    input: &str,
    model_id: &str,
    language: &str,
    out_dir: &std::path::Path,
    hw: &DetectedHardware,
) -> Vec<(String, VariantKind, TranscriptionConfig)> {
    variant_plan(hw)
        .into_iter()
        .map(|(label, kind)| {
            let use_gpu = label != "cpu" && label != "cpu+vad";
            let vad = label.ends_with("+vad");
            let config = make_config(input, model_id, language, out_dir, use_gpu, vad, &label);
            (label, kind, config)
        })
        .collect()
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

    log(&app, "Detecting hardware...");
    let hw = hardware::detect().await;
    log(
        &app,
        format!(
            "Hardware: nvidia={} amd={} intel_gpu={} vulkan_loader={} | threads: {}",
            hw.nvidia,
            hw.amd,
            hw.intel_gpu,
            hw.vulkan_loader,
            srt_transcribe::transcribe::default_n_threads()
        ),
    );
    if let Some(s) = audio_seconds {
        log(&app, format!("Media duration: {s:.1}s"));
    }

    let variants = variant_configs(&input_path, &model_id, &language, out_dir.path(), &hw);
    let total = variants.len();
    log(
        &app,
        format!(
            "Planned {total} variant(s): {}",
            variants
                .iter()
                .map(|(label, ..)| label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    let mut rows = Vec::with_capacity(total);

    for (i, (label, kind, config)) in variants.into_iter().enumerate() {
        log(
            &app,
            format!("── [{}/{}] variant '{label}' starting...", i + 1, total),
        );
        let _ = app.emit(
            "bench-progress",
            serde_json::json!({ "current": i + 1, "total": total, "variant": label }),
        );

        let backend_label = match kind {
            VariantKind::InProcess if config.use_gpu => gpu_backend_name().to_string(),
            VariantKind::InProcess => "cpu".to_string(),
            VariantKind::Worker(id) => id.to_string(),
        };

        let start = Instant::now();
        let result = match kind {
            VariantKind::InProcess => {
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
                transcribe_to_srt(&config, &ffmpeg, callbacks, &CancellationToken::new())
                    .await
                    .map_err(|e| format!("{e:#}"))
            }
            VariantKind::Worker(backend_id) => {
                let log_app = app.clone();
                let worker_log = move |line: String| {
                    log(&log_app, format!("   [{backend_id}] {line}"));
                };
                match workers::ensure_worker(&app, backend_id, &worker_log).await {
                    Ok(worker_path) => {
                        let cb_app = app.clone();
                        let cb_label = label.clone();
                        workers::run_worker_variant(&worker_path, &ffmpeg, &config, move |update| {
                            let _ = cb_app.emit(
                                "bench-stage",
                                serde_json::json!({ "variant": cb_label, "stage": update.stage, "pct": update.percentage }),
                            );
                        })
                        .await
                    }
                    Err(e) => Err(e),
                }
            }
        };
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
                    backend: backend_label,
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
                log(&app, format!("   '{label}' FAILED after {wall:.1}s: {e}"));
                BenchRow {
                    variant: label.clone(),
                    backend: backend_label,
                    vad: config.vad,
                    model: model_id.clone(),
                    wall_seconds: wall,
                    audio_seconds,
                    realtime_factor: None,
                    subtitle_count: 0,
                    detected_language: None,
                    status: "error".into(),
                    error: Some(e),
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
            use_sample_media,
            prepare_assets,
            run_bench
        ])
        .run(tauri::generate_context!())
        .expect("error while running whisper-bench");
}
