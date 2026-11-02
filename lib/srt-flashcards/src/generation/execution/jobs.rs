//! Bound concurrent extraction, drain children on cancellation, and count outcomes.
use super::*;

pub(super) async fn run_jobs(jobs: MediaJobs<'_>) -> Result<MediaOutcome, String> {
    let MediaJobs {
        config,
        cancel,
        progress,
        active_lines,
        media_dir,
        audio_source: media_source_arc,
        video_source: video_source_arc,
        video_encoder,
        snapshot_settings,
        mut snapshot_groups,
        crop: effective_crop,
        workers: batch_size,
    } = jobs;
    let needs_audio = config.generate_audio && media_source_arc.is_some();
    let needs_snapshots = config.generate_snapshots && video_source_arc.is_some();
    let needs_video = config.generate_video_clips && video_source_arc.is_some();
    let total_active = active_lines.len();
    let deck_sanitized = sanitize_filename(&config.deck_name);
    let ep = config.episode_number;
    let ffmpeg_cmd_arc = Arc::<str>::from(snapshot_settings.ffmpeg.as_str());
    let video_codec = config.video_codec.clone();
    let h264_preset = config.h264_preset.clone();
    let mut audio_count = 0usize;
    let mut snapshot_count = 0usize;
    let mut video_count = 0usize;

    let total_media_ops = active_lines.len()
        * (needs_audio as usize + needs_snapshots as usize + needs_video as usize);
    let mut completed_ops = 0usize;

    if total_media_ops > 0 {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(batch_size));

        let hw_video_semaphore = video_encoder
            .is_hardware()
            .then(|| Arc::new(tokio::sync::Semaphore::new(3)));
        let mut tasks: tokio::task::JoinSet<Vec<(&'static str, anyhow::Result<()>, usize)>> =
            tokio::task::JoinSet::new();

        for &(seq, line) in active_lines {
            let seq_num = seq + 1;
            let start_ms = line.subs1.start_ms;
            let end_ms = line.subs1.end_ms;

            if needs_audio {
                let source = media_source_arc.clone().unwrap();
                let output_path = media_dir.join(media_filename(
                    MediaKind::Audio(config.audio_format),
                    &deck_sanitized,
                    ep,
                    seq_num,
                ));
                let bitrate = config.audio_bitrate;
                let audio_format = config.audio_format;
                let audio_track_index = config.audio_track_index;
                let pad_s = config.audio_pad_start_ms;
                let pad_e = config.audio_pad_end_ms;
                let normalize = config.normalize_audio;
                let gain_db = if config.audio_gain_db == 0 && config.audio_boost {
                    6
                } else {
                    config.audio_gain_db
                };
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
                        audio_format,
                        normalize,
                        gain_db,
                        &ffmpeg,
                    )
                    .await;
                    vec![("audio", result, seq_num)]
                });
            }

            if let Some(requests) = snapshot_groups.remove(&seq_num) {
                let settings = snapshot_settings.clone();
                let permit = semaphore.clone();
                tasks.spawn(async move {
                    let _permit = permit.acquire_owned().await.expect("semaphore open");
                    settings.extract(requests).await
                });
            }

            if needs_video {
                let source = video_source_arc.clone().unwrap();
                let output_path = media_dir.join(media_filename(
                    MediaKind::Video(video_clip_extension(&video_codec)),
                    &deck_sanitized,
                    ep,
                    seq_num,
                ));
                let codec = video_codec.clone();
                let preset = h264_preset.clone();
                let vbr = config.video_bitrate;
                let abr = config.video_audio_bitrate;
                let audio_track_index = config.audio_track_index;
                let pad_s = config.video_pad_start_ms;
                let pad_e = config.video_pad_end_ms;
                let normalize = config.normalize_audio;
                let gain_db = if config.audio_gain_db == 0 && config.audio_boost {
                    6
                } else {
                    config.audio_gain_db
                };
                let w = config.video_width.unwrap_or(config.snapshot_width);
                let h = config.video_height.unwrap_or(config.snapshot_height);
                let crop = effective_crop;
                let ffmpeg = ffmpeg_cmd_arc.clone();
                let permit = semaphore.clone();

                let hw_permit_source = hw_video_semaphore.clone();
                tasks.spawn(async move {
                    let _permit = permit.acquire_owned().await.expect("semaphore open");
                    let _hw_permit = match hw_video_semaphore_acquire(hw_permit_source).await {
                        Ok(p) => p,
                        Err(_) => {
                            return vec![(
                                "video",
                                Err(anyhow::anyhow!("semaphore closed")),
                                seq_num,
                            )];
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
                        normalize,
                        gain_db,
                        &ffmpeg,
                    )
                    .await;
                    // Safety net: a probed GPU encoder can still fail on a
                    // specific clip (driver quirks, session limits); retry
                    // that clip once in software instead of losing it.
                    // The GPU attempt is finished. A software retry must not
                    // reserve a hardware session while doing CPU work.
                    drop(_hw_permit);
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
                            normalize,
                            gain_db,
                            &ffmpeg,
                        )
                        .await;
                    }
                    vec![("video", result, seq_num)]
                });
            }
        }

        loop {
            let joined = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    tasks.abort_all();
                    while tasks.join_next().await.is_some() {}
                    return Ok(MediaOutcome::Cancelled(MediaCounts { audio: audio_count, snapshots: snapshot_count, video: video_count }));
                }
                joined = tasks.join_next() => joined,
            };
            let Some(joined) = joined else {
                break;
            };
            if cancel.is_cancelled() {
                tasks.abort_all();
                while tasks.join_next().await.is_some() {}
                return Ok(MediaOutcome::Cancelled(MediaCounts {
                    audio: audio_count,
                    snapshots: snapshot_count,
                    video: video_count,
                }));
            }

            let Ok(results) = joined else {
                continue;
            };
            for (kind, result, seq_num) in results {
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
    }

    // Report aggregate media failures. Never export dangling media references.
    let mut failed_media = 0usize;
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
            failed_media += total_active - count;
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

    if cancel.is_cancelled() {
        return Ok(MediaOutcome::Cancelled(MediaCounts {
            audio: audio_count,
            snapshots: snapshot_count,
            video: video_count,
        }));
    }
    if failed_media > 0 {
        return Err(format!(
            "Failed to generate {failed_media} media file(s); no deck was exported."
        ));
    }

    Ok(MediaOutcome::Completed(MediaCounts {
        audio: audio_count,
        snapshots: snapshot_count,
        video: video_count,
    }))
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    #[tokio::test]
    async fn software_retries_release_gpu_sessions_for_pending_clips() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.srt");
        std::fs::write(
            &source,
            (0..4)
                .map(|i| {
                    format!(
                        "{}\n00:00:0{},000 --> 00:00:0{},000\nLine {}\n\n",
                        i + 1,
                        i,
                        i + 1,
                        i + 1
                    )
                })
                .collect::<String>(),
        )
        .unwrap();
        let ffmpeg = directory.path().join("ffmpeg");
        let marker = directory.path().join("all-gpu-attempts-started");
        std::fs::write(
            &ffmpeg,
            format!(
                r#"#!/bin/sh
previous=
codec=
for arg do
  [ "$previous" = "-c:v" ] && codec="$arg"
  previous="$arg"
  out="$arg"
done
if [ "$codec" != libx264 ]; then
  touch "$out.gpu-attempt"
  set -- '{root}'/*.gpu-attempt
  [ "$#" -ge 4 ] && touch '{marker}'
  exit 1
fi
while [ ! -e '{marker}' ]; do sleep 0.01; done
printf clip > "$out"
"#,
                root = directory.path().display(),
                marker = marker.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&ffmpeg, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = FlashcardConfig {
            target_subs_path: source.to_string_lossy().into(),
            generate_audio: false,
            generate_snapshots: false,
            generate_video_clips: true,
            ..FlashcardConfig::default()
        };
        let matched = build_matched_lines(&config).unwrap();
        let active: Vec<_> = matched.iter().enumerate().collect();
        let cancel = CancellationToken::new();
        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            run_jobs(MediaJobs {
                config: &config,
                cancel: &cancel,
                progress: &|_| {},
                active_lines: &active,
                media_dir: directory.path(),
                audio_source: None,
                video_source: Some(Arc::from("unused.mp4")),
                video_encoder: H264Encoder::Nvenc,
                snapshot_settings: snapshot_batch::SnapshotSettings {
                    source: String::new(),
                    ffmpeg: ffmpeg.to_string_lossy().into(),
                    ffprobe: String::new(),
                    width: 320,
                    height: 180,
                    crop: 0,
                    format: SnapshotFormat::Webp,
                    quality: 80,
                    batch_source: None,
                    cancel: cancel.clone(),
                },
                snapshot_groups: HashMap::new(),
                crop: 0,
                // Independent of host CPU count: exercise four workers and three
                // hardware sessions with real, failing child processes.
                workers: 4,
            }),
        )
        .await
        .expect("CPU fallbacks must not block the fourth GPU attempt")
        .unwrap();
        match outcome {
            MediaOutcome::Completed(counts) => assert_eq!(counts.video, 4),
            MediaOutcome::Cancelled(_) => panic!("Unexpected cancellation"),
        }
        assert!(marker.exists());
    }
}
