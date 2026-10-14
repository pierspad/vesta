//! Full-pipeline checks: generation never touches the previous TSV media early.
#![cfg(unix)]
use srt_flashcards::{FlashcardConfig, MediaTools, OutputFields, generate};
use std::os::unix::fs::PermissionsExt;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn tsv_regeneration_preserves_previous_export_on_failure_or_cancellation() {
    for mode in ["success", "failure", "cancel"] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.srt");
        let audio = dir.path().join("input.wav");
        let output = dir.path().join("output");
        let media = output.join("Deck.media");
        let tsv = output.join("Deck.tsv");
        std::fs::create_dir_all(&media).unwrap();
        std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
        std::fs::write(&audio, "source audio").unwrap();
        std::fs::write(&tsv, "previous tsv").unwrap();
        std::fs::write(media.join("old.mp3"), "previous audio").unwrap();
        let ffmpeg = dir.path().join("ffmpeg");
        let script = if mode == "failure" {
            "#!/bin/sh\n[ \"$1\" = -version ] && exit 0\nexit 1\n"
        } else {
            "#!/bin/sh\n[ \"$1\" = -version ] && exit 0\nfor arg do out=\"$arg\"; done\nprintf 'new audio' > \"$out\"\n"
        };
        std::fs::write(&ffmpeg, script).unwrap();
        std::fs::set_permissions(&ffmpeg, std::fs::Permissions::from_mode(0o700)).unwrap();
        let cancel = CancellationToken::new();
        let signal = cancel.clone();
        let config = FlashcardConfig {
            target_subs_path: input.to_string_lossy().into(),
            audio_path: Some(audio.to_string_lossy().into()),
            output_dir: output.to_string_lossy().into(),
            deck_name: "Deck".into(),
            export_format: Some("tsv".into()),
            generate_audio: true,
            generate_snapshots: false,
            generate_video_clips: false,
            output_fields: OutputFields {
                include_audio: true,
                ..OutputFields::default()
            },
            ..FlashcardConfig::default()
        };
        let result = generate(
            config,
            MediaTools::new(ffmpeg.to_string_lossy(), "unused-ffprobe"),
            cancel,
            &|event| {
                if mode == "cancel" && event.message == "flashcards.progress.generatingTsv" {
                    signal.cancel();
                }
            },
        )
        .await;
        if mode == "success" {
            let result = result.unwrap();
            assert!(result.success);
            assert_eq!(result.output_size_bytes, 9);
            assert!(!media.join("old.mp3").exists());
            let generated: Vec<_> = std::fs::read_dir(&media).unwrap().collect();
            assert_eq!(generated.len(), 1);
            let generated = generated[0].as_ref().unwrap();
            assert_eq!(std::fs::read(generated.path()).unwrap(), b"new audio");
            let text = std::fs::read_to_string(&tsv).unwrap();
            assert!(text.contains(&format!(
                "[sound:{}]",
                generated.file_name().to_string_lossy()
            )));
        } else {
            if mode == "failure" {
                assert!(result.is_err());
            } else {
                assert!(!result.unwrap().success);
            }
            assert_eq!(std::fs::read(&tsv).unwrap(), b"previous tsv");
            assert_eq!(
                std::fs::read(media.join("old.mp3")).unwrap(),
                b"previous audio"
            );
            assert_eq!(std::fs::read_dir(&media).unwrap().count(), 1);
        }
        for entry in std::fs::read_dir(&output).unwrap() {
            assert!(
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".vesta-export-")
            );
        }
    }
}
