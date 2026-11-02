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
        use_gpu: None,
        max_offset_secs: None,
        existing_anchors: Vec::new(),
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

#[tokio::test]
async fn external_sync_needs_no_whisper_and_cancels_pending_http() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let directory = tempfile::tempdir().unwrap();
    let media = directory.path().join("speech.wav");
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=30",
            "-ar",
            "16000",
            "-ac",
            "1",
        ])
        .arg(&media)
        .status()
        .await
        .unwrap();
    assert!(status.success());

    for cancel_pending in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            assert!(String::from_utf8_lossy(&request).contains("timestamp-model"));
            let _ = started_tx.send(());
            if cancel_pending {
                std::future::pending::<()>().await;
            }
            let body = r#"{"segments":[{"start":1.0,"end":2.0,"text":"Hello there friend"}]}"#;
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let mut config = AutoSyncConfig::new(
            media.to_string_lossy().into_owned(),
            directory.path().join("no-model.bin"),
        );
        config.quick = true;
        let cloud = srt_transcribe::cloud::CloudConfig {
            provider: "custom".into(),
            api_key: String::new(),
            api_url: Some(url),
            model: "timestamp-model".into(),
            language: None,
            translate_to_english: false,
        };
        let token = CancellationToken::new();
        let task_token = token.clone();
        let task = tokio::spawn(async move {
            run_auto_sync_with_cloud(
                &config,
                vec![SubtitleLine {
                    id: 1,
                    start_ms: 6000,
                    text: "Hello there friend".into(),
                }],
                None,
                &task_token,
                Some(cloud),
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(10), started_rx)
            .await
            .unwrap()
            .unwrap();
        if cancel_pending {
            token.cancel();
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(outcome.cancelled, cancel_pending);
        if cancel_pending {
            assert!(outcome.suggestions.is_empty());
            server.abort();
            let _ = server.await;
        } else {
            assert_eq!(outcome.segments_analyzed, 1);
            server.await.unwrap();
        }
    }
}
