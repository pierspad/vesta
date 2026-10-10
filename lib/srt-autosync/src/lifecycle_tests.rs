use super::*;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

fn executable(path: &std::path::Path, script: &str) {
    std::fs::write(path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[tokio::test]
async fn invalid_probe_durations_and_failed_segment_preparation_are_errors() {
    let directory = tempfile::tempdir().unwrap();
    let probe = directory.path().join("probe");
    for value in ["nan", "inf", "-1", "0", "not-a-number"] {
        executable(&probe, &format!("#!/bin/sh\nprintf '%s' '{value}'\n"));
        assert!(
            get_media_duration("unused", probe.to_str().unwrap())
                .await
                .is_err()
        );
    }
    let error = prepare_single_segment(
        0,
        0.0,
        20.0,
        100.0,
        true,
        "missing".into(),
        directory.path().to_path_buf(),
        directory
            .path()
            .join("missing-ffmpeg")
            .to_string_lossy()
            .into_owned(),
        CancellationToken::new(),
    )
    .await
    .err()
    .unwrap();
    assert!(error.contains("Failed to prepare segment"));
}

#[tokio::test]
async fn autosync_cancellation_stops_preparation_children_without_loading_whisper() {
    let directory = tempfile::tempdir().unwrap();
    let ffmpeg = directory.path().join("ffmpeg");
    let ffprobe = directory.path().join("ffprobe");
    let pid_path = directory.path().join("children.pid");
    let model = directory.path().join("dummy-model.bin");
    std::fs::write(&model, b"not loaded in this test").unwrap();
    executable(&ffprobe, "#!/bin/sh\nprintf '120'\n");
    let quoted_pid_path = pid_path.to_string_lossy().replace('\'', "'\\''");
    executable(
        &ffmpeg,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = -version ]; then exit 0; fi\necho $$ >> '{quoted_pid_path}'\nexec sleep 30\n"
        ),
    );
    let config = AutoSyncConfig {
        media_path: "unused".into(),
        model_path: model,
        language: None,
        quick: true,
        ffmpeg_cmd: ffmpeg.to_string_lossy().into_owned(),
        ffprobe_cmd: ffprobe.to_string_lossy().into_owned(),
    };
    let token = CancellationToken::new();
    let task_token = token.clone();
    let task =
        tokio::spawn(async move { run_auto_sync(&config, Vec::new(), None, &task_token).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if std::fs::read_to_string(&pid_path).is_ok_and(|value| !value.trim().is_empty()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    token.cancel();
    let outcome = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(outcome.cancelled);
    assert!(outcome.suggestions.is_empty());
    let pids = std::fs::read_to_string(&pid_path).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut alive = false;
            for pid in pids.lines() {
                alive |= tokio::process::Command::new("kill")
                    .args(["-0", pid])
                    .stderr(std::process::Stdio::null())
                    .status()
                    .await
                    .unwrap()
                    .success();
            }
            if !alive {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Autosync must not leave FFmpeg preparation processes running");
}
