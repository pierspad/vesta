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

            match optimize_video_source_with_encoder(
                src,
                target_w,
                target_h,
                config.crop_bottom,
                needs_video,
                Some(detected_gpu_encoder),
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn generation_probes_hardware_once_and_keeps_the_original_source() {
        let directory = tempfile::tempdir().unwrap();
        let log = directory.path().join("encoder-probes");
        let ffmpeg = directory.path().join("ffmpeg");
        let ffprobe = directory.path().join("ffprobe");
        std::fs::write(
            &ffmpeg,
            format!(
                "#!/bin/sh\nprintf 'probe\\n' >> '{}'\nexit 0\n",
                log.display()
            ),
        )
        .unwrap();
        std::fs::write(&ffprobe, "#!/bin/sh\nprintf '%s' '{\"streams\":[{\"width\":320,\"height\":240,\"codec_name\":\"h264\"}],\"format\":{\"duration\":\"1\"}}'\n").unwrap();
        for path in [&ffmpeg, &ffprobe] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let tools = MediaTools::new(ffmpeg.to_string_lossy(), ffprobe.to_string_lossy());
        let mut config = FlashcardConfig {
            optimize_video: true,
            video_hw_accel: "auto".into(),
            snapshot_width: 320,
            snapshot_height: 240,
            ..FlashcardConfig::default()
        };
        let cancel = CancellationToken::new();
        let (encoder, optimized) =
            prepare_video(&config, &tools, &cancel, &|_| {}, true, Some("source.mp4")).await;
        assert!(encoder.is_hardware());
        assert!(optimized.unwrap().original_used);
        assert_eq!(std::fs::read_to_string(&log).unwrap().lines().count(), 1);

        // Explicit CPU mode must not probe or initialize a GPU.
        std::fs::remove_file(&log).unwrap();
        config.video_hw_accel = "off".into();
        let (encoder, optimized) =
            prepare_video(&config, &tools, &cancel, &|_| {}, true, Some("source.mp4")).await;
        assert_eq!(encoder, H264Encoder::Libx264);
        assert!(optimized.unwrap().original_used);
        assert!(!log.exists());
    }
}
