//! Cancellation must not wait for the next completed media operation.
#![cfg(unix)]
use srt_flashcards::{FlashcardConfig, MediaTools, generate};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn script(path: &Path, content: &str) {
    std::fs::write(path, content).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[tokio::test]
async fn cancelling_individual_media_batch_probe_and_metadata_stops_children() {
    for stage in ["individual", "batch", "metadata"] {
        let dir = tempfile::tempdir().unwrap();
        let subtitles = dir.path().join("source.srt");
        std::fs::write(&subtitles, "1\n00:00:00,000 --> 00:00:01,000\nFirst.\n\n2\n00:00:01,000 --> 00:00:02,000\nSecond.\n").unwrap();
        let video = dir.path().join("video.mp4");
        std::fs::write(&video, []).unwrap();
        let pid_file = dir.path().join("pid");
        let ffmpeg = dir.path().join("ffmpeg");
        script(
            &ffmpeg,
            &format!(
                "#!/bin/sh\nif [ \"$1\" = \"-version\" ]; then exit 0; fi\nprintf '%s' \"$$\" > '{}'\nexec sleep 60\n",
                pid_file.display()
            ),
        );
        let ffprobe = dir.path().join("ffprobe");
        let metadata = r#"{"streams":[{"codec_name":"h264","width":1280,"height":720,"time_base":"1/24000","r_frame_rate":"24/1","avg_frame_rate":"24/1"}],"format":{"start_time":"0.000000"}}"#;
        let when = if stage == "metadata" {
            "*"
        } else {
            "*show_packets*"
        };
        script(
            &ffprobe,
            &format!(
                "#!/bin/sh\ncase \"$*\" in\n{when}) printf '%s' \"$$\" > '{}'\nexec sleep 60;;\n*) printf '%s' '{metadata}';;\nesac\n",
                pid_file.display()
            ),
        );
        let config = FlashcardConfig {
            target_subs_path: subtitles.to_string_lossy().into(),
            video_path: Some(video.to_string_lossy().into()),
            output_dir: dir.path().join("output").to_string_lossy().into(),
            deck_name: "Cancellation".into(),
            export_format: Some("apkg".into()),
            cpu_cores: Some(1),
            generate_snapshots: true,
            optimize_video: stage != "individual",
            ..FlashcardConfig::default()
        };
        let cancel = CancellationToken::new();
        let signal = cancel.clone();
        let pid_copy = pid_file.clone();
        let watcher = tokio::spawn(async move {
            for _ in 0..200 {
                if pid_copy.exists() {
                    signal.cancel();
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("Test process did not start");
        });
        let result = tokio::time::timeout(
            Duration::from_secs(4),
            generate(
                config,
                MediaTools::new(ffmpeg.to_string_lossy(), ffprobe.to_string_lossy()),
                cancel,
                &|_| {},
            ),
        )
        .await
        .unwrap()
        .unwrap();
        watcher.await.unwrap();
        assert!(!result.success, "stage={stage}");
        assert!(result.apkg_path.is_none());
        let pid = std::fs::read_to_string(pid_file).unwrap();
        // Tokio reaps kill_on_drop children asynchronously; require termination
        // within a bounded wait, rather than leaving an orphaned sleep process.
        for _ in 0..100 {
            let alive = std::process::Command::new("kill")
                .args(["-0", &pid])
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap()
                .success();
            if !alive {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            !std::process::Command::new("kill")
                .args(["-0", &pid])
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap()
                .success(),
            "Child {pid} survived {stage} cancellation"
        );
    }
}

#[tokio::test]
async fn media_errors_preserve_existing_deck_instead_of_exporting_dangling_references() {
    let dir = tempfile::tempdir().unwrap();
    let subtitles = dir.path().join("source.srt");
    std::fs::write(&subtitles, "1\n00:00:00,000 --> 00:00:01,000\nFirst.\n").unwrap();
    let video = dir.path().join("empty.mp4");
    std::fs::write(&video, []).unwrap();
    let ffmpeg = dir.path().join("ffmpeg");
    script(
        &ffmpeg,
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ]; then exit 0; fi\necho 'deliberate extraction error' >&2\nexit 1\n",
    );
    let old_deck = dir.path().join("Existing.apkg");
    std::fs::write(&old_deck, b"existing deck").unwrap();
    let config = FlashcardConfig {
        target_subs_path: subtitles.to_string_lossy().into(),
        video_path: Some(video.to_string_lossy().into()),
        output_dir: dir.path().to_string_lossy().into(),
        deck_name: "Existing".into(),
        generate_snapshots: true,
        optimize_video: false,
        export_format: Some("apkg".into()),
        ..FlashcardConfig::default()
    };
    let error = generate(
        config,
        MediaTools::new(ffmpeg.to_string_lossy(), "ffprobe"),
        CancellationToken::new(),
        &|_| {},
    )
    .await
    .unwrap_err();
    assert!(error.contains("no deck was exported"));
    assert_eq!(std::fs::read(old_deck).unwrap(), b"existing deck");
}
