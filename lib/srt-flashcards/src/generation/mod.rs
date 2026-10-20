//! Coordinate subtitle processing, media execution, and export publication.
//! Staging remains owned until jobs finish or cancellation drains them.
mod execution;
mod publication;
use super::*;
use crate::export_apkg::generate_apkg;
use crate::export_tsv::{generate_tsv, sanitize_filename};
use crate::media::{
    MediaKind, extract_audio_clip, extract_video_clip, media_filename,
    optimize_video_source_with_progress, video_clip_extension,
};
use execution::execute_media;
use publication::{ExportWorkspace, publish_export};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_util::sync::CancellationToken;

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
        output_size_bytes: 0,
    }
}

/// Clamp the requested worker count into a safe range.
///
/// `None` defaults to ~3/4 of the logical cores and keeps one core free when
/// possible. An explicit request is honored up to the machine's logical core
/// count; `1` is allowed for apples-to-apples single-worker benchmarks.
fn resolve_worker_count(requested: Option<usize>) -> usize {
    let num_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    resolve_worker_count_for(requested, num_cores)
}

fn resolve_worker_count_for(requested: Option<usize>, num_cores: usize) -> usize {
    let num_cores = num_cores.max(1);
    match requested {
        Some(user_cores) => user_cores.clamp(1, num_cores),
        None => (num_cores * 3)
            .div_ceil(4)
            .clamp(1, num_cores.saturating_sub(1).max(1)),
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
            output_size_bytes: 0,
        });
    }

    let workspace = ExportWorkspace::new(&config)?;

    let counts = match execute_media(
        &config,
        &tools,
        &cancel,
        progress,
        &matched,
        &workspace.media,
    )
    .await?
    {
        MediaOutcome::Completed(counts) => counts,
        MediaOutcome::Cancelled(counts) => {
            return Ok(cancelled_result(
                "Cancelled by user",
                counts.audio,
                counts.snapshots,
                counts.video,
            ));
        }
    };
    let MediaCounts {
        audio: audio_count,
        snapshots: snapshot_count,
        video: video_count,
    } = counts;

    let Some(published) = publish_export(&config, &matched, workspace, &cancel, progress)? else {
        return Ok(cancelled_result(
            "Cancelled by user",
            audio_count,
            snapshot_count,
            video_count,
        ));
    };
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
        tsv_path: published.tsv_path,
        apkg_path: published.apkg_path,
        output_size_bytes: published.size_bytes,
    })
}

#[cfg(test)]
mod worker_count_tests {
    use super::resolve_worker_count_for;

    #[test]
    fn defaults_reserve_one_core_but_explicit_counts_use_the_machine() {
        assert_eq!(resolve_worker_count_for(Some(99), 16), 16);
        assert_eq!(resolve_worker_count_for(None, 16), 12);
        assert_eq!(resolve_worker_count_for(None, 4), 3);
    }

    #[test]
    fn never_invents_workers_on_small_machines() {
        assert_eq!(resolve_worker_count_for(None, 1), 1);
        assert_eq!(resolve_worker_count_for(Some(8), 1), 1);
        assert_eq!(resolve_worker_count_for(None, 2), 1);
        assert_eq!(resolve_worker_count_for(Some(0), 2), 1);
    }
}

#[derive(Default)]
struct MediaCounts {
    audio: usize,
    snapshots: usize,
    video: usize,
}
enum MediaOutcome {
    Completed(MediaCounts),
    Cancelled(MediaCounts),
}
