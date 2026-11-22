//! Build media requests from matched cues and prepare sources before scheduling.
mod jobs;
mod video;
use super::*;
use jobs::run_jobs;
use video::prepare_video;

pub(super) async fn execute_media(
    config: &FlashcardConfig,
    tools: &MediaTools,
    cancel: &CancellationToken,
    progress: ProgressCallback<'_>,
    matched: &[MatchedLine],
    media_dir: &Path,
) -> Result<MediaOutcome, String> {
    let media_source = config
        .audio_path
        .as_deref()
        .or(config.video_path.as_deref());
    let video_source = config.video_path.as_deref();

    let needs_audio = config.generate_audio && media_source.is_some();
    let needs_snapshots = config.generate_snapshots && video_source.is_some();
    let needs_video = config.generate_video_clips && video_source.is_some();

    if (needs_audio || needs_snapshots || needs_video) && !check_ffmpeg(&tools.ffmpeg).await {
        return Err("ffmpeg not found. Install ffmpeg to extract media.".to_string());
    }

    let deck_sanitized = sanitize_filename(&config.deck_name);
    let ep = config.episode_number;

    let media_source_arc = media_source.map(Arc::<str>::from);
    let video_codec = config.video_codec.clone();

    let (detected_gpu_encoder, opt_video) =
        prepare_video(config, tools, cancel, progress, needs_video, video_source).await;

    let (effective_video_path, effective_crop) = match &opt_video {
        Some(opt) if !opt.original_used => (
            opt.path
                .to_str()
                .unwrap_or_else(|| video_source.unwrap_or_default()),
            if opt.crop_applied {
                0
            } else {
                config.crop_bottom
            },
        ),
        _ => (video_source.unwrap_or_default(), config.crop_bottom),
    };
    let video_source_arc = video_source
        .is_some()
        .then(|| Arc::<str>::from(effective_video_path));

    let video_encoder = if needs_video
        && video_codec == "h264"
        && detected_gpu_encoder.is_hardware()
        && opt_video.as_ref().is_none_or(|o| o.original_used)
    {
        detected_gpu_encoder
    } else {
        H264Encoder::Libx264
    };

    let batch_size = resolve_worker_count(config.cpu_cores);

    let active_lines: Vec<(usize, &MatchedLine)> =
        matched.iter().filter(|m| m.active).enumerate().collect();

    let snapshot_requests: Vec<_> = if needs_snapshots {
        active_lines
            .iter()
            .map(|&(seq, line)| snapshot_batch::SnapshotRequest {
                seq: seq + 1,
                start_ms: line.subs1.start_ms,
                end_ms: line.subs1.end_ms,
                output: media_dir.join(media_filename(
                    MediaKind::Snapshot(config.snapshot_format),
                    &deck_sanitized,
                    ep,
                    seq + 1,
                )),
            })
            .collect()
    } else {
        Vec::new()
    };
    let candidate_groups = snapshot_batch::groups(snapshot_requests.clone());
    let batch_source = if config.optimize_video
        && config.snapshot_format == SnapshotFormat::Webp
        && candidate_groups.iter().any(|group| group.len() > 1)
    {
        snapshot_batch::probe(effective_video_path, &tools.ffprobe, cancel).await
    } else {
        None
    };
    let grouped = if batch_source.is_some() {
        candidate_groups
    } else {
        snapshot_requests
            .into_iter()
            .map(|request| vec![request])
            .collect()
    };
    let snapshot_groups: HashMap<_, _> = grouped
        .into_iter()
        .map(|group| {
            // Schedule a group at its earliest card, retaining interleaved audio work.
            (
                group.iter().map(|request| request.seq).min().unwrap(),
                group,
            )
        })
        .collect();
    let snapshot_settings = snapshot_batch::SnapshotSettings {
        source: effective_video_path.to_string(),
        ffmpeg: tools.ffmpeg.clone(),
        ffprobe: tools.ffprobe.clone(),
        width: config.snapshot_width,
        height: config.snapshot_height,
        crop: effective_crop,
        format: config.snapshot_format,
        quality: config.snapshot_quality,
        batch_source,
        cancel: cancel.clone(),
    };
    if cancel.is_cancelled() {
        return Ok(MediaOutcome::Cancelled(MediaCounts::default()));
    }

    run_jobs(MediaJobs {
        config,
        cancel,
        progress,
        active_lines: &active_lines,
        media_dir,
        audio_source: media_source_arc,
        video_source: video_source_arc,
        video_encoder,
        snapshot_settings,
        snapshot_groups,
        crop: effective_crop,
        workers: batch_size,
    })
    .await
}

struct MediaJobs<'a> {
    config: &'a FlashcardConfig,
    cancel: &'a CancellationToken,
    progress: ProgressCallback<'a>,
    active_lines: &'a [(usize, &'a MatchedLine)],
    media_dir: &'a Path,
    audio_source: Option<Arc<str>>,
    video_source: Option<Arc<str>>,
    video_encoder: H264Encoder,
    snapshot_settings: snapshot_batch::SnapshotSettings,
    snapshot_groups: HashMap<usize, Vec<snapshot_batch::SnapshotRequest>>,
    crop: u32,
    workers: usize,
}
