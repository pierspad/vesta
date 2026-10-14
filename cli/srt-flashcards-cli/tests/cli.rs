use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_srt-flashcards"))
        .args(args)
        .output()
        .unwrap()
}

fn subtitles(path: &std::path::Path) {
    std::fs::write(
        path,
        "1\n00:00:01,000 --> 00:00:02,000\nHello there\n\n2\n00:00:03,000 --> 00:00:04,500\nSecond line\n",
    )
    .unwrap();
}

#[test]
fn info_and_preview_print_summaries_without_stderr_noise() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    subtitles(&input);

    let info = run(&["info", input.to_str().unwrap()]);
    assert!(
        info.status.success(),
        "{}",
        String::from_utf8_lossy(&info.stderr)
    );
    let stdout = String::from_utf8_lossy(&info.stdout);
    assert!(stdout.contains("Format:   srt"));
    assert!(stdout.contains("Lines:    2"));
    assert!(stdout.contains("First:    Hello there"));
    assert!(stdout.contains("Last:     Second line"));
    assert!(info.stderr.is_empty());

    let output = dir.path().join("out");
    let preview = run(&[
        "preview",
        "--target",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--no-audio",
        "--no-snapshots",
        "--no-video",
    ]);
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&preview.stdout),
        "Total lines:   2\nActive lines:  2\nWith native:   0\n"
    );
    assert!(preview.stderr.is_empty());
    assert!(!output.exists(), "preview must not create output files");
}

#[test]
fn invalid_flags_and_missing_inputs_have_distinct_cli_exit_codes() {
    let invalid_flag = run(&["generate", "--format", "typo"]);
    assert_eq!(invalid_flag.status.code(), Some(2));
    assert!(invalid_flag.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid_flag.stderr).contains("typo"));

    let invalid_quality = run(&[
        "preview",
        "--target",
        "missing.srt",
        "--output",
        "unused",
        "--snapshot-quality",
        "101",
    ]);
    assert_eq!(invalid_quality.status.code(), Some(2));
    assert!(invalid_quality.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid_quality.stderr).contains("0 to 100"));

    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.srt");
    let info = run(&["info", missing.to_str().unwrap()]);
    assert_eq!(info.status.code(), Some(1));
    assert!(info.stdout.is_empty());
    assert!(String::from_utf8_lossy(&info.stderr).contains("missing.srt"));
}

#[test]
fn text_only_generation_reports_success_on_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let output_dir = dir.path().join("out");
    subtitles(&input);
    let result = run(&[
        "generate",
        "--target",
        input.to_str().unwrap(),
        "--output",
        output_dir.to_str().unwrap(),
        "--no-audio",
        "--no-snapshots",
        "--no-video",
        "--quiet",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("Generated 2 cards"));
    assert!(result.stderr.is_empty());
    assert!(output_dir.join("Vesta.tsv").exists());
}

#[cfg(unix)]
#[test]
fn failed_media_generation_preserves_existing_tsv_and_media() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let audio = dir.path().join("audio.wav");
    let output_dir = dir.path().join("out");
    let media_dir = output_dir.join("Vesta.media");
    let old_tsv = "existing deck\n";
    let old_media = b"existing media";
    subtitles(&input);
    std::fs::write(&audio, b"stub").unwrap();
    std::fs::create_dir_all(&media_dir).unwrap();
    std::fs::write(output_dir.join("Vesta.tsv"), old_tsv).unwrap();
    std::fs::write(media_dir.join("old.mp3"), old_media).unwrap();

    let ffmpeg = dir.path().join("ffmpeg-fail");
    std::fs::write(
        &ffmpeg,
        "#!/bin/sh\n[ \"$1\" = -version ] && exit 0\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&ffmpeg, std::fs::Permissions::from_mode(0o700)).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_srt-flashcards"))
        .args([
            "generate",
            "--target",
            input.to_str().unwrap(),
            "--audio",
            audio.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--ffmpeg",
            ffmpeg.to_str().unwrap(),
            "--no-snapshots",
            "--no-video",
            "--quiet",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("no deck was exported"));
    assert_eq!(
        std::fs::read_to_string(output_dir.join("Vesta.tsv")).unwrap(),
        old_tsv
    );
    assert_eq!(std::fs::read(media_dir.join("old.mp3")).unwrap(), old_media);
}

#[cfg(unix)]
#[test]
fn ctrl_c_returns_130_and_preserves_existing_tsv_and_media() {
    use std::{
        os::unix::fs::PermissionsExt,
        process::{Child, Stdio},
        thread,
        time::{Duration, Instant},
    };

    struct ChildGuard(Option<Child>);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let audio = dir.path().join("audio.wav");
    let output_dir = dir.path().join("out");
    let media_dir = output_dir.join("Vesta.media");
    let handshake = dir.path().join("ffmpeg.started");
    let ffmpeg = dir.path().join("ffmpeg-wait");
    let old_tsv = "existing deck\n";
    let old_media = b"existing media";
    subtitles(&input);
    std::fs::write(&audio, b"stub").unwrap();
    std::fs::create_dir_all(&media_dir).unwrap();
    std::fs::write(output_dir.join("Vesta.tsv"), old_tsv).unwrap();
    std::fs::write(media_dir.join("old.mp3"), old_media).unwrap();
    std::fs::write(
        &ffmpeg,
        format!(
            "#!/bin/sh\n[ \"$1\" = -version ] && exit 0\nprintf started > '{}'\nexec sleep 30\n",
            handshake.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&ffmpeg, std::fs::Permissions::from_mode(0o700)).unwrap();

    let child = Command::new(env!("CARGO_BIN_EXE_srt-flashcards"))
        .args([
            "generate",
            "--target",
            input.to_str().unwrap(),
            "--audio",
            audio.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--ffmpeg",
            ffmpeg.to_str().unwrap(),
            "--no-snapshots",
            "--no-video",
            "--quiet",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut guard = ChildGuard(Some(child));
    let child = guard.0.as_mut().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !handshake.exists() && Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("CLI exited before ffmpeg handshake: {status}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(handshake.exists(), "ffmpeg did not start in time");
    let signal = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(signal.success(), "failed to send SIGINT to CLI child");
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "CLI did not exit after SIGINT");
        thread::sleep(Duration::from_millis(10));
    };
    let output = guard.0.take().unwrap().wait_with_output().unwrap();
    assert_eq!(status.code(), Some(130));
    assert_eq!(output.status.code(), Some(130));
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Generated"));
    assert_eq!(
        std::fs::read_to_string(output_dir.join("Vesta.tsv")).unwrap(),
        old_tsv
    );
    assert_eq!(std::fs::read(media_dir.join("old.mp3")).unwrap(), old_media);
}
