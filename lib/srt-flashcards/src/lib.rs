//! # srt-flashcards
//!
//! Headless, GUI-agnostic engine that turns a subtitle file (optionally a pair
//! of *target* + *native* subtitles) plus a media file into Anki flashcards,
//! exported either as a subs2srs-style TSV + media folder or as a self-contained
//! `.apkg` package.
//!
//! The crate is deliberately **free of any Tauri / GUI coupling** (see
//! `.cursorrules` §9): progress is reported through a plain callback and work is
//! cancelled through a [`CancellationToken`], so the exact same code powers the
//! Vesta desktop app, the `srt-flashcards` headless CLI and the benchmark
//! harness.
//!
//! ## Pipeline
//!
//! ```text
//! parse → time-shift → match (dual subs) → span → filter → combine → context
//!       → parallel ffmpeg media extraction → TSV / APKG export
//! ```
//!
//! ## Quick start
//!
//! ```no_run
//! use srt_flashcards::{generate, FlashcardConfig, MediaTools};
//! use tokio_util::sync::CancellationToken;
//!
//! # async fn run(config: FlashcardConfig) -> Result<(), String> {
//! let result = generate(
//!     config,
//!     MediaTools::default(),          // resolve `ffmpeg`/`ffprobe` from PATH
//!     CancellationToken::new(),
//!     &|p| eprintln!("[{:>3.0}%] {}", p.percentage, p.stage), // progress sink
//! )
//! .await?;
//! println!("Generated {} cards", result.cards_generated);
//! # Ok(())
//! # }
//! ```

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

mod export_apkg;
mod export_tsv;
mod filters;
mod matcher;
mod parser;
mod types;

pub mod media;

pub use media::{H264Encoder, check_ffmpeg, detect_h264_encoder, video_has_audio};
pub use types::*;

use export_apkg::generate_apkg;
use export_tsv::{generate_tsv, sanitize_filename};
use filters::{apply_filters, apply_span, combine_sentences, compute_context};
use matcher::match_subtitles;
use media::{extract_audio_clip, extract_snapshot, extract_video_clip, normalize_audio};
use parser::parse_subtitle_file;

// ─── Public support types ────────────────────────────────────────────────────

/// External media tools used for extraction.
///
/// Each field is the command the engine will spawn — a bare name resolved via
/// `PATH` (the [`Default`]) or an absolute path to a bundled binary (used by the
/// desktop app, which can download ffmpeg into its data directory).
#[derive(Debug, Clone)]
pub struct MediaTools {
    pub ffmpeg: String,
    pub ffprobe: String,
}

impl Default for MediaTools {
    fn default() -> Self {
        Self {
            ffmpeg: "ffmpeg".to_string(),
            ffprobe: "ffprobe".to_string(),
        }
    }
}

impl MediaTools {
    /// Build from explicit commands/paths (empty values fall back to PATH).
    pub fn new(ffmpeg: impl Into<String>, ffprobe: impl Into<String>) -> Self {
        let ffmpeg = ffmpeg.into();
        let ffprobe = ffprobe.into();
        Self {
            ffmpeg: if ffmpeg.is_empty() {
                "ffmpeg".into()
            } else {
                ffmpeg
            },
            ffprobe: if ffprobe.is_empty() {
                "ffprobe".into()
            } else {
                ffprobe
            },
        }
    }
}

/// Progress callback signature.
///
/// The engine invokes this from the orchestrating task (never from the parallel
/// ffmpeg workers); it must be `Send + Sync` so the generation future stays
/// `Send` on multi-threaded runtimes (e.g. Tauri).
pub type ProgressCallback<'a> = &'a (dyn Fn(FlashcardProgressEvent) + Send + Sync);

fn emit(
    progress: ProgressCallback<'_>,
    stage: &str,
    message: &str,
    current: usize,
    total: usize,
    pct: f64,
    params: HashMap<String, String>,
) {
    progress(FlashcardProgressEvent {
        stage: stage.to_string(),
        message: message.to_string(),
        current,
        total,
        percentage: pct,
        params,
    });
}

/// Acquire the optional hardware-encode permit (no-op when encoding on CPU).
async fn hw_video_semaphore_acquire(
    semaphore: Option<Arc<tokio::sync::Semaphore>>,
) -> Result<Option<tokio::sync::OwnedSemaphorePermit>, tokio::sync::AcquireError> {
    match semaphore {
        Some(s) => Ok(Some(s.acquire_owned().await?)),
        None => Ok(None),
    }
}

fn cancelled_result(
    message: &str,
    audio: usize,
    snapshots: usize,
    video: usize,
) -> FlashcardResult {
    FlashcardResult {
        success: false,
        message: message.to_string(),
        cards_generated: 0,
        audio_clips: audio,
        snapshots,
        video_clips: video,
        tsv_path: None,
        apkg_path: None,
    }
}

// ─── Subtitle pipeline (shared by `preview` and `generate`) ──────────────────

/// Run the full deterministic subtitle pipeline:
/// parse → time-shift → match → span → filter → combine → context.
///
/// This single source of truth is shared by [`preview`] and [`generate`] so the
/// two can never drift apart (previously this block was copy-pasted between the
/// preview command and the generation routine).
pub fn build_matched_lines(config: &FlashcardConfig) -> Result<Vec<MatchedLine>, String> {
    // Target subtitles + time shift.
    let (mut subs1, _) =
        parse_subtitle_file(&config.target_subs_path).map_err(|e| e.to_string())?;
    if config.time_shift_target_ms != 0 {
        for s in subs1.iter_mut() {
            s.start_ms += config.time_shift_target_ms;
            s.end_ms += config.time_shift_target_ms;
        }
    }

    // Optional native subtitles + their own time shift.
    let subs2 = match config.native_subs_path.as_ref() {
        Some(path) => {
            let (mut entries, _) = parse_subtitle_file(path).map_err(|e| e.to_string())?;
            if config.time_shift_native_ms != 0 {
                for s in entries.iter_mut() {
                    s.start_ms += config.time_shift_native_ms;
                    s.end_ms += config.time_shift_native_ms;
                }
            }
            Some(entries)
        }
        None => None,
    };

    // Match dual subtitles, or build a 1:1 list when there is no native track.
    let mut matched = match subs2.as_ref() {
        Some(s2) => match_subtitles(&subs1, s2),
        None => subs1
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
            .collect(),
    };

    apply_span(&mut matched, config.span_start_ms, config.span_end_ms);
    apply_filters(&mut matched, &config.filters);
    if config.combine_sentences {
        combine_sentences(&mut matched, &config.continuation_chars);
    }
    compute_context(&mut matched, &config.context);

    Ok(matched)
}

/// Parse + match + filter and return every line (active and inactive) as a
/// [`PreviewLine`], ready to render in a preview table. Pure CPU work, no media.
pub fn preview(config: &FlashcardConfig) -> Result<Vec<PreviewLine>, String> {
    let matched = build_matched_lines(config)?;
    Ok(matched
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
        .collect())
}

/// Parse a subtitle file and summarise it (count, format, actors, duration).
pub fn load_sub_file_info(path: &str) -> Result<SubFileInfo, String> {
    let (entries, format) = parse_subtitle_file(path).map_err(|e| e.to_string())?;

    let mut actors: Vec<String> = entries
        .iter()
        .filter_map(|e| e.actor.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    actors.sort();

    Ok(SubFileInfo {
        path: path.to_string(),
        format: format.to_string(),
        count: entries.len(),
        first_text: entries.first().map(|e| e.text.clone()).unwrap_or_default(),
        last_text: entries.last().map(|e| e.text.clone()).unwrap_or_default(),
        actors,
        duration_ms: entries.last().map(|e| e.end_ms).unwrap_or(0),
    })
}

/// Probe a media file and return its audio streams in ffmpeg `0:a:N` order.
pub async fn list_audio_tracks(
    path: &str,
    ffprobe_cmd: &str,
) -> Result<Vec<AudioTrackInfo>, String> {
    let output = media::media_command(ffprobe_cmd)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=index,codec_name,channels:stream_tags=language,title",
            "-of",
            "json",
            path,
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

    Ok(streams
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
        .collect())
}

/// Clamp the requested worker count into a safe range.
///
/// `None` defaults to ~3/4 of the logical cores. An explicit request is bounded
/// to `[1, cores-1]`: the upper bound keeps a core free for the OS / UI, while
/// `1` is allowed so a run can be pinned to a single ffmpeg worker (e.g. an
/// apples-to-apples comparison against single-threaded tools).
fn resolve_worker_count(requested: Option<usize>) -> usize {
    let num_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    match requested {
        Some(user_cores) => user_cores.max(1).min(num_cores.saturating_sub(1).max(2)),
        None => (num_cores / 4 * 3).max(2),
    }
}

// ─── Generation ──────────────────────────────────────────────────────────────

/// Generate flashcards: run the subtitle pipeline, extract media in parallel via
/// ffmpeg, and write the chosen export (`tsv` or `apkg`).
///
/// * `tools` — the ffmpeg/ffprobe executables to invoke.
/// * `cancel` — cooperative cancellation; checked between stages and media batches.
/// * `progress` — called with [`FlashcardProgressEvent`]s as the run advances.
pub async fn generate(
    config: FlashcardConfig,
    tools: MediaTools,
    cancel: CancellationToken,
    progress: ProgressCallback<'_>,
) -> Result<FlashcardResult, String> {
    // --- Stage 1: Parse + match + filter (shared pipeline) ---
    emit(
        progress,
        "parsing",
        "flashcards.progress.parsing",
        0,
        100,
        0.0,
        HashMap::new(),
    );

    if cancel.is_cancelled() {
        return Ok(cancelled_result("Cancelled", 0, 0, 0));
    }

    emit(
        progress,
        "matching",
        "flashcards.progress.matching",
        5,
        100,
        5.0,
        HashMap::new(),
    );
    emit(
        progress,
        "filtering",
        "flashcards.progress.filtering",
        10,
        100,
        10.0,
        HashMap::new(),
    );

    let matched = build_matched_lines(&config)?;

    let total_active = matched.iter().filter(|m| m.active).count();
    if total_active == 0 {
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

    // --- Output directories ---
    let output_dir = PathBuf::from(&config.output_dir);
    // Make sure the destination exists up front. TSV gets this for free (its
    // `<deck>.media` folder lives inside `output_dir`), but APKG packs media in a
    // temp dir, so without this an APKG export into a not-yet-existing folder
    // (e.g. `~/Downloads/decks/`) would fail when the `.apkg` file is created.
    std::fs::create_dir_all(&output_dir).map_err(|e| {
        format!(
            "Cannot create output directory '{}': {e}",
            output_dir.display()
        )
    })?;
    let export_format = config.export_format.as_deref().unwrap_or("tsv");

    // APKG packs media from a temp dir; TSV writes a sibling `<deck>.media` folder
    // matching the classic subs2srs layout.
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
    // Clean any stale media from a prior run.
    if media_dir.exists() {
        let _ = std::fs::remove_dir_all(&media_dir);
    }
    std::fs::create_dir_all(&media_dir).map_err(|e| format!("Cannot create output dir: {}", e))?;

    // Media sources
    let media_source = config
        .audio_path
        .as_deref()
        .or(config.video_path.as_deref());
    let video_source = config.video_path.as_deref();

    let needs_audio = config.generate_audio && media_source.is_some();
    let needs_snapshots = config.generate_snapshots && video_source.is_some();
    let needs_video = config.generate_video_clips && video_source.is_some();

    // ffmpeg must exist before we spawn hundreds of workers.
    if (needs_audio || needs_snapshots || needs_video) && !check_ffmpeg(&tools.ffmpeg).await {
        return Err("ffmpeg not found. Install ffmpeg to extract media.".to_string());
    }

    let deck_sanitized = sanitize_filename(&config.deck_name);
    let ep = config.episode_number;

    let ffmpeg_cmd_arc = Arc::<str>::from(tools.ffmpeg.as_str());
    let media_source_arc = media_source.map(Arc::<str>::from);
    let video_source_arc = video_source.map(Arc::<str>::from);
    let video_codec = config.video_codec.clone();
    let h264_preset = config.h264_preset.clone();

    // H.264 encoder: probe the GPU once per run ("auto", the default), unless
    // the user forced software encoding ("off", expert mode).
    let video_encoder = if needs_video && video_codec == "h264" && config.video_hw_accel != "off" {
        let encoder = detect_h264_encoder(&tools.ffmpeg).await;
        if encoder.is_hardware() {
            emit(
                progress,
                "media",
                &format!("Video encoder: {}", encoder.label()),
                0,
                0,
                0.0,
                HashMap::from([("encoder".to_string(), encoder.ffmpeg_name().to_string())]),
            );
        }
        encoder
    } else {
        H264Encoder::Libx264
    };

    let batch_size = resolve_worker_count(config.cpu_cores);

    // --- Stage 4: Parallel media extraction (streaming, bounded concurrency) ---
    //
    // A semaphore caps in-flight ffmpeg processes at `batch_size` (max cores-1).
    // Unlike a fixed batch barrier, work streams continuously: the instant one
    // ffmpeg job finishes the next starts, so a single slow clip never stalls a
    // whole batch and every worker stays saturated for the full run.
    let active_lines: Vec<(usize, &MatchedLine)> =
        matched.iter().filter(|m| m.active).enumerate().collect();

    let mut audio_count = 0usize;
    let mut snapshot_count = 0usize;
    let mut video_count = 0usize;

    let total_media_ops = active_lines.len()
        * (needs_audio as usize + needs_snapshots as usize + needs_video as usize);
    let mut completed_ops = 0usize;

    if total_media_ops > 0 {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(batch_size));
        // GPU encoders have concurrent-session limits (NVENC on GeForce: 3-8
        // depending on driver) and saturate with few sessions anyway: cap
        // hardware encodes separately from the CPU-bound audio/snapshot jobs.
        let hw_video_semaphore = video_encoder
            .is_hardware()
            .then(|| Arc::new(tokio::sync::Semaphore::new(3)));
        let mut tasks: tokio::task::JoinSet<(&'static str, anyhow::Result<()>, usize)> =
            tokio::task::JoinSet::new();

        for &(seq, line) in &active_lines {
            let seq_num = seq + 1;
            let start_ms = line.subs1.start_ms;
            let end_ms = line.subs1.end_ms;

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
                let permit = semaphore.clone();

                tasks.spawn(async move {
                    let _permit = permit.acquire_owned().await.expect("semaphore open");
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
                    ("audio", result, seq_num)
                });
            }

            if needs_snapshots {
                let source = video_source_arc.clone().unwrap();
                let output_path =
                    media_dir.join(format!("{}_{:03}_{:04}.jpg", deck_sanitized, ep, seq_num));
                let w = config.snapshot_width;
                let h = config.snapshot_height;
                let crop = config.crop_bottom;
                let ffmpeg = ffmpeg_cmd_arc.clone();
                let permit = semaphore.clone();

                tasks.spawn(async move {
                    let _permit = permit.acquire_owned().await.expect("semaphore open");
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
                    ("snapshot", result, seq_num)
                });
            }

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
                let permit = semaphore.clone();

                let hw_permit_source = hw_video_semaphore.clone();
                tasks.spawn(async move {
                    let _permit = permit.acquire_owned().await.expect("semaphore open");
                    let _hw_permit = match hw_video_semaphore_acquire(hw_permit_source).await {
                        Ok(p) => p,
                        Err(_) => {
                            return ("video", Err(anyhow::anyhow!("semaphore closed")), seq_num);
                        }
                    };
                    let mut result = extract_video_clip(
                        &source,
                        &output_path,
                        start_ms,
                        end_ms,
                        pad_s,
                        pad_e,
                        &codec,
                        &preset,
                        video_encoder,
                        vbr,
                        abr,
                        audio_track_index,
                        w,
                        h,
                        crop,
                        &ffmpeg,
                    )
                    .await;
                    // Safety net: a probed GPU encoder can still fail on a
                    // specific clip (driver quirks, session limits); retry
                    // that clip once in software instead of losing it.
                    if result.is_err() && video_encoder.is_hardware() {
                        result = extract_video_clip(
                            &source,
                            &output_path,
                            start_ms,
                            end_ms,
                            pad_s,
                            pad_e,
                            &codec,
                            &preset,
                            media::H264Encoder::Libx264,
                            vbr,
                            abr,
                            audio_track_index,
                            w,
                            h,
                            crop,
                            &ffmpeg,
                        )
                        .await;
                    }
                    ("video", result, seq_num)
                });
            }
        }

        while let Some(joined) = tasks.join_next().await {
            if cancel.is_cancelled() {
                tasks.abort_all();
                return Ok(cancelled_result(
                    "Cancelled by user",
                    audio_count,
                    snapshot_count,
                    video_count,
                ));
            }

            let Ok((kind, result, seq_num)) = joined else {
                continue;
            };

            completed_ops += 1;
            let pct = 15.0 + (completed_ops as f64 / total_media_ops.max(1) as f64) * 75.0;

            match (kind, &result) {
                ("audio", Ok(())) => audio_count += 1,
                ("snapshot", Ok(())) => snapshot_count += 1,
                ("video", Ok(())) => video_count += 1,
                (kind, Err(e)) => {
                    let (msg_key, label) = match kind {
                        "audio" => ("flashcards.progress.audioFailed", "Audio"),
                        "snapshot" => ("flashcards.progress.snapshotFailed", "Snapshot"),
                        _ => ("flashcards.progress.videoFailed", "Video"),
                    };
                    eprintln!("{} extraction failed for line {}: {}", label, seq_num, e);
                    emit(
                        progress,
                        "media",
                        msg_key,
                        completed_ops,
                        total_media_ops,
                        pct,
                        HashMap::from([
                            ("line".to_string(), seq_num.to_string()),
                            ("error".to_string(), e.to_string()),
                        ]),
                    );
                }
                _ => {}
            }

            if result.is_ok() {
                emit(
                    progress,
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

    // Report aggregate media failures.
    for (needs, count, msg_key) in [
        (
            needs_audio,
            audio_count,
            "flashcards.progress.audioExtractionsFailed",
        ),
        (
            needs_snapshots,
            snapshot_count,
            "flashcards.progress.snapshotExtractionsFailed",
        ),
        (
            needs_video,
            video_count,
            "flashcards.progress.videoExtractionsFailed",
        ),
    ] {
        if needs && count < total_active {
            emit(
                progress,
                "media",
                msg_key,
                completed_ops,
                total_media_ops,
                90.0,
                HashMap::from([("count".to_string(), (total_active - count).to_string())]),
            );
        }
    }

    // --- Stage 5: Export ---
    let mut tsv_path_result: Option<String> = None;
    let mut apkg_path_result: Option<String> = None;

    if export_format == "apkg" {
        emit(
            progress,
            "tsv",
            "flashcards.progress.generatingApkg",
            90,
            100,
            90.0,
            HashMap::new(),
        );
        let apkg_path = output_dir.join(format!("{}.apkg", sanitize_filename(&config.deck_name)));
        generate_apkg(&matched, &config, &media_dir, &apkg_path)?;
        apkg_path_result = Some(apkg_path.to_string_lossy().to_string());
    } else {
        emit(
            progress,
            "tsv",
            "flashcards.progress.generatingTsv",
            90,
            100,
            90.0,
            HashMap::new(),
        );
        let media_str = media_dir.to_str().unwrap_or("");
        let tsv_content = generate_tsv(&matched, &config, media_str, media_str, media_str);
        let tsv_path = output_dir.join(format!("{}.tsv", sanitize_filename(&config.deck_name)));
        std::fs::write(&tsv_path, tsv_content.as_bytes())
            .map_err(|e| format!("Cannot write TSV: {}", e))?;
        tsv_path_result = Some(tsv_path.to_string_lossy().to_string());
    }

    emit(
        progress,
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
