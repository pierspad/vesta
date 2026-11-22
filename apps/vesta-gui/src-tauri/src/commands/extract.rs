use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use super::media::{resolve_ffmpeg_path, resolve_ffprobe_path};
use crate::state::{AppOcrState, OperationGuard};

#[tauri::command]
pub async fn embedded_subtitle_tracks(
    app: AppHandle,
    path: String,
) -> Result<Vec<vesta_extract::embedded::EmbeddedSubtitleTrack>, String> {
    let ffprobe = resolve_ffprobe_path(Some(&app)).await;
    vesta_extract::embedded::list_embedded_subtitles(&ffprobe, std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_embedded_subtitle(
    app: AppHandle,
    path: String,
    index: u32,
    output_path: String,
) -> Result<(), String> {
    let (ffmpeg, ffprobe) = resolve_media_tools(&app).await;
    vesta_extract::embedded::extract_embedded_subtitle(
        &ffmpeg,
        &ffprobe,
        std::path::Path::new(&path),
        index,
        std::path::Path::new(&output_path),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_output_path(app: AppHandle, path: String, folder: bool) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = std::path::PathBuf::from(path);
    let path = if folder {
        path.parent().ok_or("Output has no parent folder")?
    } else {
        path.as_path()
    };
    if !path.exists() {
        return Err("Output no longer exists".into());
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Preview uses a temporary extraction and caps returned text; no user output is written.
#[tauri::command]
pub async fn preview_embedded_subtitle(
    app: AppHandle,
    path: String,
    index: u32,
) -> Result<String, String> {
    let (ffmpeg, ffprobe) = resolve_media_tools(&app).await;
    vesta_extract::embedded::preview_embedded_subtitle(
        &ffmpeg,
        &ffprobe,
        std::path::Path::new(&path),
        index,
    )
    .await
    .map_err(|error| error.to_string())
}

/// Scan only the requested portable output filenames in the chosen directory.
#[tauri::command]
pub async fn existing_subtitle_outputs(
    directory: String,
    names: Vec<String>,
) -> Result<Vec<String>, String> {
    let directory = std::path::PathBuf::from(directory);
    let mut existing = Vec::new();
    for name in names {
        if !name.ends_with(".srt") || name.contains(['/', '\\']) {
            return Err("Invalid subtitle filename".into());
        }
        match tokio::fs::metadata(directory.join(&name)).await {
            Ok(metadata) if metadata.is_file() => existing.push(name),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(existing)
}

async fn resolve_media_tools(app: &AppHandle) -> (String, String) {
    let ffprobe_path = resolve_ffprobe_path(Some(app)).await;
    let ffmpeg_path = resolve_ffmpeg_path(Some(app)).await;
    (ffmpeg_path, ffprobe_path)
}

fn ocr_progress_callbacks(app: AppHandle) -> vesta_ocr::OcrCallbacks {
    vesta_ocr::OcrCallbacks {
        on_progress: Some(Arc::new(move |progress: vesta_ocr::OcrProgress| {
            let _ = app.emit("ocr-progress", progress);
        })),
        on_segment: None,
    }
}

fn validate_hardcoded_params(
    roi: vesta_ocr::VideoRoi,
    sample_fps: Option<f64>,
) -> Result<f64, String> {
    roi.validate().map_err(|e| e.to_string())?;
    let fps = sample_fps.unwrap_or(2.5);
    if !fps.is_finite() || !(0.5..=15.0).contains(&fps) {
        return Err(format!(
            "Sample FPS must be finite and within 0.5..=15.0 (got {fps})"
        ));
    }
    Ok(fps)
}

async fn build_ocr_config(
    app: &AppHandle,
    path: String,
    index: u32,
    output_path: std::path::PathBuf,
    language: Option<String>,
    tiers: Vec<Vec<vesta_translate::TierEntry>>,
    preview_limit: Option<usize>,
) -> vesta_ocr::OcrConfig {
    let (ffmpeg_path, ffprobe_path) = resolve_media_tools(app).await;
    vesta_ocr::OcrConfig {
        input_path: std::path::PathBuf::from(path),
        output_path,
        track_index: index,
        language,
        tiers,
        preview_limit,
        ffmpeg_path,
        ffprobe_path,
    }
}

#[allow(clippy::too_many_arguments)]
async fn build_hardcoded_ocr_config(
    app: &AppHandle,
    path: String,
    output_path: std::path::PathBuf,
    roi: vesta_ocr::VideoRoi,
    sample_fps: Option<f64>,
    language: Option<String>,
    tiers: Vec<Vec<vesta_translate::TierEntry>>,
    preview_limit: Option<usize>,
) -> Result<vesta_ocr::HardcodedOcrConfig, String> {
    let sample_fps = validate_hardcoded_params(roi, sample_fps)?;
    let (ffmpeg_path, ffprobe_path) = resolve_media_tools(app).await;
    Ok(vesta_ocr::HardcodedOcrConfig {
        input_path: std::path::PathBuf::from(path),
        output_path,
        roi,
        sample_fps,
        language,
        tiers,
        preview_limit,
        ffmpeg_path,
        ffprobe_path,
    })
}

/// Runs bitmap subtitle extraction and Vision-LLM or local OCR, writing an `.srt` file.
#[tauri::command]
pub async fn ocr_embedded_subtitle(
    app: AppHandle,
    state: State<'_, AppOcrState>,
    path: String,
    index: u32,
    output_path: String,
    language: Option<String>,
    tiers: Vec<Vec<vesta_translate::TierEntry>>,
) -> Result<(), String> {
    let guard = OperationGuard::begin(&state, "ERR_ALREADY_RUNNING")?;
    let config = build_ocr_config(
        &app,
        path,
        index,
        std::path::PathBuf::from(output_path),
        language,
        tiers,
        None,
    )
    .await;
    let callbacks = ocr_progress_callbacks(app);
    let cancel_token = guard.token();
    vesta_ocr::ocr_bitmap_track_to_srt(&config, callbacks, &cancel_token)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Previews up to 15 bitmap subtitle cues via OCR without writing to disk.
#[tauri::command]
pub async fn preview_ocr_embedded_subtitle(
    app: AppHandle,
    state: State<'_, AppOcrState>,
    path: String,
    index: u32,
    language: Option<String>,
    tiers: Vec<Vec<vesta_translate::TierEntry>>,
) -> Result<String, String> {
    let guard = OperationGuard::begin(&state, "ERR_ALREADY_RUNNING")?;
    let config = build_ocr_config(
        &app,
        path,
        index,
        std::path::PathBuf::new(),
        language,
        tiers,
        Some(15),
    )
    .await;
    let cancel_token = guard.token();
    vesta_ocr::preview_ocr_bitmap_track(&config, &cancel_token)
        .await
        .map_err(|e| e.to_string())
}

/// Extracts a PNG video frame preview for burned-in subtitle ROI selection.
#[tauri::command]
pub async fn ocr_video_preview_frame(
    app: AppHandle,
    path: String,
    time_ms: Option<u64>,
) -> Result<vesta_ocr::VideoFramePreview, String> {
    let (ffmpeg, ffprobe) = resolve_media_tools(&app).await;
    vesta_ocr::extract_video_preview_frame(
        &ffmpeg,
        &ffprobe,
        std::path::Path::new(&path),
        time_ms,
        None,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Previews up to 15 burned-in subtitle cues within `roi` via OCR without writing to disk.
#[tauri::command]
pub async fn preview_ocr_hardcoded_subtitle(
    app: AppHandle,
    state: State<'_, AppOcrState>,
    path: String,
    roi: vesta_ocr::VideoRoi,
    sample_fps: Option<f64>,
    language: Option<String>,
    tiers: Vec<Vec<vesta_translate::TierEntry>>,
) -> Result<String, String> {
    let guard = OperationGuard::begin(&state, "ERR_ALREADY_RUNNING")?;
    let config = build_hardcoded_ocr_config(
        &app,
        path,
        std::path::PathBuf::new(),
        roi,
        sample_fps,
        language,
        tiers,
        Some(15),
    )
    .await?;
    let cancel_token = guard.token();
    vesta_ocr::preview_ocr_hardcoded(&config, &cancel_token)
        .await
        .map_err(|e| e.to_string())
}

/// Runs burned-in subtitle ROI extraction and OCR, writing an `.srt` file while emitting `ocr-progress`.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn ocr_hardcoded_subtitle(
    app: AppHandle,
    state: State<'_, AppOcrState>,
    path: String,
    output_path: String,
    roi: vesta_ocr::VideoRoi,
    sample_fps: Option<f64>,
    language: Option<String>,
    tiers: Vec<Vec<vesta_translate::TierEntry>>,
) -> Result<(), String> {
    let guard = OperationGuard::begin(&state, "ERR_ALREADY_RUNNING")?;
    let config = build_hardcoded_ocr_config(
        &app,
        path,
        std::path::PathBuf::from(output_path),
        roi,
        sample_fps,
        language,
        tiers,
        None,
    )
    .await?;
    let callbacks = ocr_progress_callbacks(app);
    let cancel_token = guard.token();
    vesta_ocr::ocr_hardcoded_to_srt(&config, callbacks, &cancel_token)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Cancels any active OCR extraction or model download operation.
#[tauri::command]
pub fn ocr_cancel(state: State<'_, AppOcrState>) -> Result<bool, String> {
    let current = state.lock().map_err(|e| e.to_string())?;
    Ok(current.operation.cancel())
}

/// Lists cataloged local OCR models and their installation status.
#[tauri::command]
pub async fn ocr_list_models() -> Result<Vec<vesta_ocr::OcrModelInfo>, String> {
    vesta_ocr::model::list_models().map_err(|e| e.to_string())
}

/// Downloads a local OCR model while emitting progress on `ocr-progress`.
#[tauri::command]
pub async fn ocr_download_model(
    app: AppHandle,
    state: State<'_, AppOcrState>,
    model_id: String,
) -> Result<bool, String> {
    let guard = OperationGuard::begin(&state, "ERR_ALREADY_RUNNING")?;
    let cancel_token = guard.token();
    let app_progress = app.clone();
    let model_id_progress = model_id.clone();

    vesta_ocr::model::download_model(
        &model_id,
        move |pct| {
            let _ = app_progress.emit(
                "ocr-progress",
                vesta_ocr::OcrProgress {
                    stage: "download".into(),
                    message: format!("Downloading OCR model {model_id_progress} ({pct}%)..."),
                    percentage: pct as f64,
                    current: pct as usize,
                    total: 100,
                },
            );
        },
        Some(&cancel_token),
    )
    .await
    .map(|_| true)
    .map_err(|e| e.to_string())
}

/// Uninstalls a cached local OCR model from disk.
#[tauri::command]
pub async fn ocr_uninstall_model(
    state: State<'_, AppOcrState>,
    model_id: String,
) -> Result<bool, String> {
    let _guard = OperationGuard::begin(&state, "OCR or model download already in progress")?;
    tokio::task::spawn_blocking(move || vesta_ocr::model::uninstall_model(&model_id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod output_tests {
    use super::*;
    #[tokio::test]
    async fn scan_distinguishes_variants_and_ignores_directories() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("film_fr_1.srt"), "subtitle").unwrap();
        std::fs::create_dir(directory.path().join("film_fr_3.srt")).unwrap();
        let found = existing_subtitle_outputs(
            directory.path().to_string_lossy().into(),
            vec![
                "film_fr_1.srt".into(),
                "film_fr_2.srt".into(),
                "film_fr_3.srt".into(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(found, vec!["film_fr_1.srt"]);
        assert!(
            existing_subtitle_outputs(
                directory.path().to_string_lossy().into(),
                vec!["../outside.srt".into()]
            )
            .await
            .is_err()
        );
        std::fs::remove_file(directory.path().join("film_fr_1.srt")).unwrap();
        assert!(
            existing_subtitle_outputs(
                directory.path().to_string_lossy().into(),
                vec!["film_fr_1.srt".into()]
            )
            .await
            .unwrap()
            .is_empty()
        );
    }
}

#[cfg(test)]
mod ocr_tests {
    use super::*;

    #[test]
    fn ocr_state_guard_rejects_overlap_and_cancels_on_drop() {
        let state = AppOcrState::default();
        let guard = OperationGuard::begin(&state, "ERR_ALREADY_RUNNING").unwrap();
        let token = guard.token();
        assert_eq!(
            OperationGuard::begin(&state, "ERR_ALREADY_RUNNING")
                .err()
                .as_deref(),
            Some("ERR_ALREADY_RUNNING")
        );
        assert!(state.lock().unwrap().operation.cancel());
        assert!(token.is_cancelled());
        drop(guard);
        assert!(!state.lock().unwrap().operation.cancel());
        assert!(OperationGuard::begin(&state, "ERR_ALREADY_RUNNING").is_ok());
    }

    #[tokio::test]
    async fn ocr_list_models_returns_catalog() {
        let models = ocr_list_models().await.unwrap();
        assert_eq!(models.len(), vesta_ocr::model::OCR_MODELS.len());
    }

    #[test]
    fn hardcoded_params_validation_checks_roi_bounds_and_fps() {
        let valid_roi = vesta_ocr::VideoRoi {
            x: 0.08,
            y: 0.74,
            width: 0.84,
            height: 0.20,
        };
        assert_eq!(validate_hardcoded_params(valid_roi, None).unwrap(), 2.5);
        assert_eq!(
            validate_hardcoded_params(valid_roi, Some(5.0)).unwrap(),
            5.0
        );
        assert!(validate_hardcoded_params(valid_roi, Some(0.2)).is_err());
        assert!(validate_hardcoded_params(valid_roi, Some(20.0)).is_err());
        assert!(validate_hardcoded_params(valid_roi, Some(f64::NAN)).is_err());

        let invalid_roi = vesta_ocr::VideoRoi {
            x: 0.5,
            y: 0.7,
            width: 0.6,
            height: 0.2,
        };
        assert!(validate_hardcoded_params(invalid_roi, Some(2.5)).is_err());
    }
}
