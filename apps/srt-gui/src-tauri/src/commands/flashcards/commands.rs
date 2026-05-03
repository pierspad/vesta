use crate::state::AppFlashcardState;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use super::export_apkg::generate_apkg;
use super::export_tsv::{generate_tsv, sanitize_filename};
use super::filters::{apply_filters, apply_span, combine_sentences, compute_context};
use super::matcher::match_subtitles;
use super::media::{
    check_ffmpeg, extract_audio_clip, extract_snapshot, extract_video_clip, normalize_audio,
};
use super::parser::parse_subtitle_file;
use super::types::{
    AudioTrackInfo, FlashcardConfig, FlashcardProgressEvent, FlashcardResult, MatchedLine,
    PreviewLine, SubFileInfo,
};

// ─── Tauri Commands ──────────────────────────────────────────────────────────

/// Load a subtitle file and return info
#[tauri::command]
pub async fn flashcard_load_subs(path: String) -> Result<SubFileInfo, String> {
    let (entries, format) = parse_subtitle_file(&path).map_err(|e| e.to_string())?;

    let mut actors: Vec<String> = entries
        .iter()
        .filter_map(|e| e.actor.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    actors.sort();

    let first_text = entries.first().map(|e| e.text.clone()).unwrap_or_default();
    let last_text = entries.last().map(|e| e.text.clone()).unwrap_or_default();
    let duration_ms = entries.last().map(|e| e.end_ms).unwrap_or(0);

    Ok(SubFileInfo {
        path,
        format: format.to_string(),
        count: entries.len(),
        first_text,
        last_text,
        actors,
        duration_ms,
    })
}

/// Probe a media file and return its audio streams in ffmpeg 0:a:N order.
#[tauri::command]
pub async fn flashcard_list_audio_tracks(
    app: AppHandle,
    path: String,
) -> Result<Vec<AudioTrackInfo>, String> {
    let ffprobe_cmd = super::media::resolve_ffprobe_path(Some(&app)).await;
    let output = tokio::process::Command::new(ffprobe_cmd)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=index,codec_name,channels:stream_tags=language,title",
            "-of",
            "json",
            &path,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse ffprobe output: {}", e))?;
    let streams = value
        .get("streams")
        .and_then(|streams| streams.as_array())
        .ok_or_else(|| "ffprobe did not return stream data".to_string())?;

    let tracks = streams
        .iter()
        .enumerate()
        .map(|(audio_index, stream)| {
            let tags = stream.get("tags");
            AudioTrackInfo {
                index: audio_index,
                stream_index: stream
                    .get("index")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(audio_index as u64) as usize,
                codec: stream
                    .get("codec_name")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                language: tags
                    .and_then(|tags| tags.get("language"))
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string),
                title: tags
                    .and_then(|tags| tags.get("title"))
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string),
                channels: stream
                    .get("channels")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as u32),
            }
        })
        .collect();

    Ok(tracks)
}

/// Generate preview data: parse, match, filter, and return all lines
#[tauri::command]
pub async fn flashcard_preview(config: FlashcardConfig) -> Result<Vec<PreviewLine>, String> {
    let (mut subs1, _) =
        parse_subtitle_file(&config.target_subs_path).map_err(|e| e.to_string())?;

    let subs2 = if let Some(ref path) = config.native_subs_path {
        let (entries, _) = parse_subtitle_file(path).map_err(|e| e.to_string())?;
        Some(entries)
    } else {
        None
    };

    // Apply time shifts to raw subs
    for s in subs1.iter_mut() {
        s.start_ms += config.time_shift_target_ms;
        s.end_ms += config.time_shift_target_ms;
    }

    // Match dual subtitles
    let mut matched = if let Some(ref s2) = subs2 {
        let mut s2_shifted = s2.clone();
        for s in s2_shifted.iter_mut() {
            s.start_ms += config.time_shift_native_ms;
            s.end_ms += config.time_shift_native_ms;
        }
        match_subtitles(&subs1, &s2_shifted)
    } else {
        subs1
            .iter()
            .enumerate()
            .map(|(i, s)| MatchedLine {
                index: i,
                subs1: s.clone(),
                subs2: None,
                active: true,
                leading_context: Vec::new(),
                trailing_context: Vec::new(),
            })
            .collect()
    };

    // Apply span filter
    apply_span(&mut matched, config.span_start_ms, config.span_end_ms);

    // Apply filters
    apply_filters(&mut matched, &config.filters);

    // Sentence combining
    if config.combine_sentences {
        combine_sentences(&mut matched, &config.continuation_chars);
    }

    // Context lines
    compute_context(&mut matched, &config.context);

    // Convert to preview lines
    let preview: Vec<PreviewLine> = matched
        .iter()
        .map(|m| PreviewLine {
            index: m.index,
            subs1_text: m.subs1.text.clone(),
            subs2_text: m.subs2.as_ref().map(|s| s.text.clone()),
            start_ms: m.subs1.start_ms,
            end_ms: m.subs1.end_ms,
            duration_ms: m.subs1.end_ms - m.subs1.start_ms,
            active: m.active,
            actor: m.subs1.actor.clone(),
            leading_context: m.leading_context.clone(),
            trailing_context: m.trailing_context.clone(),
        })
        .collect();

    Ok(preview)
}

/// Main generation command - processes everything with parallel FFmpeg calls
#[tauri::command]
pub async fn flashcard_generate(
    app: AppHandle,
    state: State<'_, AppFlashcardState>,
    config: FlashcardConfig,
) -> Result<FlashcardResult, String> {
    // Check if already processing
    {
        let mut fc_state = state.lock().map_err(|e| e.to_string())?;
        if fc_state.is_processing {
            return Err("Already processing flashcards".to_string());
        }
        fc_state.is_processing = true;
        fc_state.cancellation_token = Some(CancellationToken::new());
    }

    let cancel_token = {
        let fc_state = state.lock().map_err(|e| e.to_string())?;
        fc_state.cancellation_token.clone().unwrap()
    };

    let result = perform_generation(Some(app.clone()), config, cancel_token).await;

    // Reset state
    {
        if let Ok(mut fc_state) = state.lock() {
            fc_state.is_processing = false;
            fc_state.cancellation_token = None;
        }
    }

    result
}

#[allow(dead_code)]
pub async fn flashcard_generate_headless(
    config: FlashcardConfig,
) -> Result<FlashcardResult, String> {
    perform_generation(None, config, CancellationToken::new()).await
}

async fn perform_generation(
    app: Option<AppHandle>,
    config: FlashcardConfig,
    cancel_token: CancellationToken,
) -> Result<FlashcardResult, String> {
    // --- Stage 1: Parse subtitles ---
    emit_progress(
        app.as_ref(),
        "parsing",
        "flashcards.progress.parsing",
        0,
        100,
        0.0,
        HashMap::new(),
    );

    let (mut subs1, _) =
        parse_subtitle_file(&config.target_subs_path).map_err(|e| e.to_string())?;

    let subs2 = if let Some(ref path) = config.native_subs_path {
        let (entries, _) = parse_subtitle_file(path).map_err(|e| e.to_string())?;
        Some(entries)
    } else {
        None
    };

    if cancel_token.is_cancelled() {
        return Ok(FlashcardResult {
            success: false,
            message: "Cancelled".to_string(),
            cards_generated: 0,
            audio_clips: 0,
            snapshots: 0,
            video_clips: 0,
            tsv_path: None,
            apkg_path: None,
        });
    }

    // Apply time shifts
    for s in subs1.iter_mut() {
        s.start_ms += config.time_shift_target_ms;
        s.end_ms += config.time_shift_target_ms;
    }

    // --- Stage 2: Match subtitles ---
    emit_progress(
        app.as_ref(),
        "matching",
        "flashcards.progress.matching",
        5,
        100,
        5.0,
        HashMap::new(),
    );

    let mut matched = if let Some(ref s2) = subs2 {
        let mut s2_shifted = s2.clone();
        for s in s2_shifted.iter_mut() {
            s.start_ms += config.time_shift_native_ms;
            s.end_ms += config.time_shift_native_ms;
        }
        match_subtitles(&subs1, &s2_shifted)
    } else {
        subs1
            .iter()
            .enumerate()
            .map(|(i, s)| MatchedLine {
                index: i,
                subs1: s.clone(),
                subs2: None,
                active: true,
                leading_context: Vec::new(),
                trailing_context: Vec::new(),
            })
            .collect()
    };

    // --- Stage 3: Filter ---
    emit_progress(
        app.as_ref(),
        "filtering",
        "flashcards.progress.filtering",
        10,
        100,
        10.0,
        HashMap::new(),
    );

    apply_span(&mut matched, config.span_start_ms, config.span_end_ms);
    apply_filters(&mut matched, &config.filters);

    if config.combine_sentences {
        combine_sentences(&mut matched, &config.continuation_chars);
    }

    compute_context(&mut matched, &config.context);

    let active_count = matched.iter().filter(|m| m.active).count();
    let total_active = active_count;

    if active_count == 0 {
        return Ok(FlashcardResult {
            success: false,
            message: "No active subtitle lines after filtering".to_string(),
            cards_generated: 0,
            audio_clips: 0,
            snapshots: 0,
            video_clips: 0,
            tsv_path: None,
            apkg_path: None,
        });
    }

    // Create output directories
    let output_dir = PathBuf::from(&config.output_dir);
    let export_format = config.export_format.as_deref().unwrap_or("tsv");

    // For APKG: use a temp directory for Anki package media.
    // For TSV: match classic subs2srs output, <deck_name>.media.
    let apkg_temp_dir = if export_format == "apkg" {
        Some(tempfile::tempdir().map_err(|e| format!("Cannot create temp dir for media: {}", e))?)
    } else {
        None
    };

    let media_dir = if let Some(ref tmp) = apkg_temp_dir {
        tmp.path().join("collection.media")
    } else {
        output_dir.join(format!("{}.media", sanitize_filename(&config.deck_name)))
    };
    // Clean existing media directory to prevent stale files from prior runs
    if media_dir.exists() {
        let _ = std::fs::remove_dir_all(&media_dir);
    }
    std::fs::create_dir_all(&media_dir).map_err(|e| format!("Cannot create output dir: {}", e))?;

    // Determine media source
    let media_source = config
        .audio_path
        .as_deref()
        .or(config.video_path.as_deref());

    let video_source = config.video_path.as_deref();

    // --- Stage 4: Generate media (parallelized) ---
    let mut audio_count = 0usize;
    let mut snapshot_count = 0usize;
    let mut video_count = 0usize;

    let active_lines: Vec<(usize, &MatchedLine)> =
        matched.iter().filter(|m| m.active).enumerate().collect();

    let needs_audio = config.generate_audio && media_source.is_some();
    let needs_snapshots = config.generate_snapshots && video_source.is_some();
    let needs_video = config.generate_video_clips && video_source.is_some();

    // Check ffmpeg availability
    if needs_audio || needs_snapshots || needs_video {
        let has_ffmpeg = match app.as_ref() {
            Some(app) => flashcard_check_deps(app.clone()).await.unwrap_or(false),
            None => super::media::check_ffmpeg().await.unwrap_or(false),
        };
        if !has_ffmpeg {
            return Err("ffmpeg not found. Install ffmpeg to extract media.".to_string());
        }
    }

    let deck_sanitized = sanitize_filename(&config.deck_name);
    let ep = config.episode_number;

    let ffmpeg_cmd = super::media::resolve_ffmpeg_path(app.as_ref()).await;
    let ffmpeg_cmd_arc = std::sync::Arc::<str>::from(ffmpeg_cmd.as_str());

    // Pre-calculate media source strings (avoid allocating per-line)
    let media_source_arc = media_source.map(|s| std::sync::Arc::<str>::from(s));
    let video_source_arc = video_source.map(|s| std::sync::Arc::<str>::from(s));
    let video_codec = config.video_codec.clone();
    let h264_preset = config.h264_preset.clone();

    // Use configured CPU cores, or default to 3/4 of available cores
    let num_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let batch_size = if let Some(user_cores) = config.cpu_cores {
        // Ensure user value is within safe bounds: min 2, max num_cores - 1 (leave at least 1 free)
        user_cores.max(2).min(num_cores.saturating_sub(1).max(2))
    } else {
        (num_cores / 4 * 3).max(2)
    };
    let total_media_ops = active_lines.len()
        * (needs_audio as usize + needs_snapshots as usize + needs_video as usize);
    let mut completed_ops = 0usize;

    for chunk in active_lines.chunks(batch_size) {
        if cancel_token.is_cancelled() {
            return Ok(FlashcardResult {
                success: false,
                message: "Cancelled by user".to_string(),
                cards_generated: 0,
                audio_clips: audio_count,
                snapshots: snapshot_count,
                video_clips: video_count,
                tsv_path: None,
                apkg_path: None,
            });
        }

        let mut handles = Vec::new();

        for &(seq, line) in chunk {
            let seq_num = seq + 1;
            let start_ms = line.subs1.start_ms;
            let end_ms = line.subs1.end_ms;
            let line_seq = seq_num; // capture for error reporting

            // Audio extraction
            if needs_audio {
                let source = media_source_arc.clone().unwrap();
                let output_path =
                    media_dir.join(format!("{}_{:03}_{:04}.mp3", deck_sanitized, ep, seq_num));
                let bitrate = config.audio_bitrate;
                let audio_track_index = config.audio_track_index;
                let pad_s = config.audio_pad_start_ms;
                let pad_e = config.audio_pad_end_ms;
                let normalize = config.normalize_audio;
                let ffmpeg = ffmpeg_cmd_arc.clone();

                handles.push(tokio::spawn(async move {
                    let result = extract_audio_clip(
                        &source,
                        &output_path,
                        start_ms,
                        end_ms,
                        pad_s,
                        pad_e,
                        bitrate,
                        audio_track_index,
                        &ffmpeg,
                    )
                    .await;
                    if result.is_ok() && normalize {
                        let _ = normalize_audio(&output_path, &ffmpeg).await;
                    }
                    ("audio", result, line_seq)
                }));
            }

            // Snapshot extraction
            if needs_snapshots {
                let source = video_source_arc.clone().unwrap();
                let output_path =
                    media_dir.join(format!("{}_{:03}_{:04}.jpg", deck_sanitized, ep, seq_num));
                let w = config.snapshot_width;
                let h = config.snapshot_height;
                let crop = config.crop_bottom;
                let ffmpeg = ffmpeg_cmd_arc.clone();

                handles.push(tokio::spawn(async move {
                    let result = extract_snapshot(
                        &source,
                        &output_path,
                        start_ms,
                        end_ms,
                        w,
                        h,
                        crop,
                        &ffmpeg,
                    )
                    .await;
                    ("snapshot", result, line_seq)
                }));
            }

            // Video clip extraction
            if needs_video {
                let source = video_source_arc.clone().unwrap();
                let ext = if video_codec == "h264" { "mp4" } else { "avi" };
                let output_path = media_dir.join(format!(
                    "{}_{:03}_{:04}.{}",
                    deck_sanitized, ep, seq_num, ext
                ));
                let codec = video_codec.clone();
                let preset = h264_preset.clone();
                let vbr = config.video_bitrate;
                let abr = config.video_audio_bitrate;
                let audio_track_index = config.audio_track_index;
                let pad_s = config.video_pad_start_ms;
                let pad_e = config.video_pad_end_ms;
                let w = config.snapshot_width;
                let h = config.snapshot_height;
                let crop = config.crop_bottom;
                let ffmpeg = ffmpeg_cmd_arc.clone();

                handles.push(tokio::spawn(async move {
                    let result = extract_video_clip(
                        &source,
                        &output_path,
                        start_ms,
                        end_ms,
                        pad_s,
                        pad_e,
                        &codec,
                        &preset,
                        vbr,
                        abr,
                        audio_track_index,
                        w,
                        h,
                        crop,
                        &ffmpeg,
                    )
                    .await;
                    ("video", result, line_seq)
                }));
            }
        }

        // Await all handles in this batch
        for handle in handles {
            if let Ok((kind, result, seq_num)) = handle.await {
                completed_ops += 1;
                let pct = 15.0 + (completed_ops as f64 / total_media_ops.max(1) as f64) * 75.0;

                match kind {
                    "audio" => {
                        if let Err(ref e) = result {
                            eprintln!("Audio extraction failed for line {}: {}", seq_num, e);
                            emit_progress(
                                app.as_ref(),
                                "media",
                                "flashcards.progress.audioFailed",
                                completed_ops,
                                total_media_ops,
                                pct,
                                HashMap::from([
                                    ("line".to_string(), seq_num.to_string()),
                                    ("error".to_string(), e.to_string()),
                                ]),
                            );
                        } else {
                            audio_count += 1;
                        }
                    }
                    "snapshot" => {
                        if let Err(ref e) = result {
                            eprintln!("Snapshot extraction failed for line {}: {}", seq_num, e);
                            emit_progress(
                                app.as_ref(),
                                "media",
                                "flashcards.progress.snapshotFailed",
                                completed_ops,
                                total_media_ops,
                                pct,
                                HashMap::from([
                                    ("line".to_string(), seq_num.to_string()),
                                    ("error".to_string(), e.to_string()),
                                ]),
                            );
                        } else {
                            snapshot_count += 1;
                        }
                    }
                    "video" => {
                        if let Err(ref e) = result {
                            eprintln!("Video clip extraction failed for line {}: {}", seq_num, e);
                            emit_progress(
                                app.as_ref(),
                                "media",
                                "flashcards.progress.videoFailed",
                                completed_ops,
                                total_media_ops,
                                pct,
                                HashMap::from([
                                    ("line".to_string(), seq_num.to_string()),
                                    ("error".to_string(), e.to_string()),
                                ]),
                            );
                        } else {
                            video_count += 1;
                        }
                    }
                    _ => {}
                }

                if result.is_ok() {
                    emit_progress(
                        app.as_ref(),
                        "media",
                        "flashcards.progress.extractingMedia",
                        completed_ops,
                        total_media_ops,
                        pct,
                        HashMap::from([
                            ("current".to_string(), completed_ops.to_string()),
                            ("total".to_string(), total_media_ops.to_string()),
                        ]),
                    );
                }
            }
        }
    }

    // Report media extraction failures
    if needs_audio && audio_count < total_active {
        let failed = total_active - audio_count;
        emit_progress(
            app.as_ref(),
            "media",
            "flashcards.progress.audioExtractionsFailed",
            completed_ops,
            total_media_ops,
            90.0,
            HashMap::from([("count".to_string(), failed.to_string())]),
        );
    }
    if needs_snapshots && snapshot_count < total_active {
        let failed = total_active - snapshot_count;
        emit_progress(
            app.as_ref(),
            "media",
            "flashcards.progress.snapshotExtractionsFailed",
            completed_ops,
            total_media_ops,
            90.0,
            HashMap::from([("count".to_string(), failed.to_string())]),
        );
    }
    if needs_video && video_count < total_active {
        let failed = total_active - video_count;
        emit_progress(
            app.as_ref(),
            "media",
            "flashcards.progress.videoExtractionsFailed",
            completed_ops,
            total_media_ops,
            90.0,
            HashMap::from([("count".to_string(), failed.to_string())]),
        );
    }

    // --- Stage 5: Generate export file ---
    let mut tsv_path_result: Option<String> = None;
    let mut apkg_path_result: Option<String> = None;

    if export_format == "apkg" {
        emit_progress(
            app.as_ref(),
            "tsv",
            "flashcards.progress.generatingApkg",
            90,
            100,
            90.0,
            HashMap::new(),
        );

        let apkg_filename = format!("{}.apkg", sanitize_filename(&config.deck_name));
        let apkg_path = output_dir.join(&apkg_filename);

        generate_apkg(&matched, &config, &media_dir, &apkg_path)?;

        apkg_path_result = Some(apkg_path.to_string_lossy().to_string());
    } else {
        emit_progress(
            app.as_ref(),
            "tsv",
            "flashcards.progress.generatingTsv",
            90,
            100,
            90.0,
            HashMap::new(),
        );

        let tsv_content = generate_tsv(
            &matched,
            &config,
            media_dir.to_str().unwrap_or(""),
            media_dir.to_str().unwrap_or(""),
            media_dir.to_str().unwrap_or(""),
        );

        let tsv_filename = format!("{}.tsv", sanitize_filename(&config.deck_name));
        let tsv_path = output_dir.join(&tsv_filename);
        std::fs::write(&tsv_path, tsv_content.as_bytes())
            .map_err(|e| format!("Cannot write TSV: {}", e))?;

        tsv_path_result = Some(tsv_path.to_string_lossy().to_string());
    }

    // --- Done ---
    emit_progress(
        app.as_ref(),
        "done",
        "flashcards.progress.complete",
        100,
        100,
        100.0,
        HashMap::new(),
    );

    Ok(FlashcardResult {
        success: true,
        message: format!(
            "Generated {} cards ({} audio, {} snapshots, {} video clips)",
            total_active, audio_count, snapshot_count, video_count
        ),
        cards_generated: total_active,
        audio_clips: audio_count,
        snapshots: snapshot_count,
        video_clips: video_count,
        tsv_path: tsv_path_result,
        apkg_path: apkg_path_result,
    })
}

/// Cancel flashcard generation
#[tauri::command]
pub async fn flashcard_cancel(state: State<'_, AppFlashcardState>) -> Result<bool, String> {
    let mut fc_state = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref token) = fc_state.cancellation_token {
        token.cancel();
    }
    fc_state.is_processing = false;
    fc_state.cancellation_token = None;
    Ok(true)
}

/// Check if ffmpeg is available
#[tauri::command]
pub async fn flashcard_check_deps(app: AppHandle) -> Result<bool, String> {
    // 1. First check if it's in system PATH
    if check_ffmpeg().await.unwrap_or(false) {
        return Ok(true);
    }
    // 2. Then check if we downloaded it into AppData
    if let Ok(app_data) = app.path().app_local_data_dir() {
        let ffmpeg_ext = if cfg!(windows) { "exe" } else { "" };
        let mut ffmpeg_path = app_data.join("ffmpeg_bin").join("ffmpeg");
        if cfg!(windows) {
            ffmpeg_path.set_extension(ffmpeg_ext);
        }
        if ffmpeg_path.exists() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Automate FFmpeg download
#[tauri::command]
pub async fn flashcard_download_ffmpeg(app: AppHandle) -> Result<bool, String> {
    use ffmpeg_sidecar::download::{download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg};

    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let dest = app_data.join("ffmpeg_bin");
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    tokio::task::spawn_blocking(move || {
        let url = ffmpeg_download_url().map_err(|e| e.to_string())?;
        let archive = download_ffmpeg_package(url, &dest).map_err(|e| e.to_string())?;
        unpack_ffmpeg(&archive, &dest).map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Check if a directory exists
#[tauri::command]
pub async fn flashcard_check_dir_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).is_dir())
}

/// Get the number of available CPU cores
#[tauri::command]
pub async fn flashcard_get_cpu_count() -> Result<usize, String> {
    Ok(std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4))
}

fn emit_progress(
    app: Option<&AppHandle>,
    stage: &str,
    message: &str,
    current: usize,
    total: usize,
    pct: f64,
    params: HashMap<String, String>,
) {
    let Some(app) = app else {
        return;
    };
    let _ = app.emit(
        "flashcard-progress",
        FlashcardProgressEvent {
            stage: stage.to_string(),
            message: message.to_string(),
            current,
            total,
            percentage: pct,
            params,
        },
    );
}
