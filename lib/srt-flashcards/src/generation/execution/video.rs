//! Select an encoder and optionally prepare the source for video clips.
use super::*;

pub(super) async fn prepare_video(
    config: &FlashcardConfig,
    tools: &MediaTools,
    cancel: &CancellationToken,
    progress: ProgressCallback<'_>,
    needs_video: bool,
    video_source: Option<&str>,
) -> (H264Encoder, Option<OptimizedVideo>) {
    // Transparent GPU auto-detection: if available, used automatically.
    let detected_gpu_encoder = if needs_video && config.video_hw_accel != "off" {
        let enc = detect_h264_encoder(&tools.ffmpeg).await;
        if enc.is_hardware() {
            emit(
                progress,
                "media",
                &format!("Hardware acceleration: {}", enc.label()),
                0,
                0,
                0.0,
                HashMap::from([("encoder".to_string(), enc.ffmpeg_name().to_string())]),
            );
        }
        enc
    } else {
        H264Encoder::Libx264
    };

    // Full-film preparation is reserved for video clips. Snapshots use the
    // original source directly, avoiding a lossy H.264 intermediate and startup delay.
    let opt_video = if needs_video && config.optimize_video {
        if let Some(src) = video_source {
            let target_w = config
                .video_width
                .unwrap_or(config.snapshot_width)
                .max(config.snapshot_width);
            let target_h = config
                .video_height
                .unwrap_or(config.snapshot_height)
                .max(config.snapshot_height);
            let hw_enabled = config.video_hw_accel != "off";

            emit(
                progress,
                "optimizing",
                &format!(
                    "Optimizing video stream with {}...",
                    detected_gpu_encoder.label()
                ),
                12,
                100,
                12.0,
                HashMap::from([(
                    "encoder".to_string(),
                    detected_gpu_encoder.ffmpeg_name().to_string(),
                )]),
            );

            match optimize_video_source_with_progress(
                src,
                target_w,
                target_h,
                config.crop_bottom,
                needs_video,
                hw_enabled,
                &tools.ffmpeg,
                &tools.ffprobe,
                cancel,
                &|percent, encoder| {
                    emit(
                        progress,
                        "optimizing",
                        "flashcards.progress.optimizingVideo",
                        percent.round() as usize,
                        100,
                        12.0 + percent * 0.03,
                        HashMap::from([
                            ("percent".into(), format!("{percent:.0}")),
                            ("encoder".into(), encoder.into()),
                        ]),
                    );
                },
            )
            .await
            {
                Ok(opt) => {
                    emit(
                        progress,
                        "optimizing",
                        if opt.original_used {
                            "Video stream ready (original source used)"
                        } else {
                            "Video stream optimized"
                        },
                        15,
                        100,
                        15.0,
                        HashMap::new(),
                    );
                    Some(opt)
                }
                Err(e) => {
                    eprintln!("Video optimization warning: {e}");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    (detected_gpu_encoder, opt_video)
}
