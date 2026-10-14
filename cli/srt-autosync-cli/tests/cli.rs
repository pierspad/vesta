use std::process::Command;

#[test]
fn missing_subtitle_fails_without_touching_output_or_downloading_a_model() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("missing.srt");
    let media = dir.path().join("missing.mp4");
    let output = dir.path().join("synced.srt");
    std::fs::write(&output, "keep this output").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_srt-autosync"))
        .args([
            input.to_str().unwrap(),
            media.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("missing.srt"));
    assert_eq!(std::fs::read_to_string(output).unwrap(), "keep this output");
}

#[cfg(unix)]
#[test]
fn ctrl_c_cancels_analysis_with_130_and_preserves_output() {
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

    fn executable(path: &std::path::Path, script: &str) {
        std::fs::write(path, script).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    fn shell_quote(value: &std::path::Path) -> String {
        format!("'{}'", value.to_string_lossy().replace('\'', "'\\''"))
    }

    let dir = tempfile::tempdir().unwrap();
    let isolated_home = dir.path().join("home");
    let cache = dir.path().join("isolated-cache");
    let model = cache.join("whisper/ggml-base.bin");
    std::fs::create_dir_all(model.parent().unwrap()).unwrap();
    std::fs::write(&model, b"offline model stub").unwrap();
    // macOS resolves the cache under ~/Library/Caches instead of honoring
    // XDG_CACHE_HOME; seed both isolated locations so this test never needs
    // a real model cache or network access on any Unix host.
    let macos_model = isolated_home.join("Library/Caches/whisper/ggml-base.bin");
    std::fs::create_dir_all(macos_model.parent().unwrap()).unwrap();
    std::fs::write(&macos_model, b"offline model stub").unwrap();

    let input = dir.path().join("input.srt");
    let media = dir.path().join("media.mp4");
    let output = dir.path().join("output.srt");
    let handshake = dir.path().join("probe.started");
    let ffmpeg = dir.path().join("ffmpeg");
    let ffprobe = dir.path().join("ffprobe");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    std::fs::write(&output, "existing output").unwrap();
    executable(
        &ffmpeg,
        "#!/bin/sh\n[ \"$1\" = -version ] && exit 0\nexit 0\n",
    );
    executable(
        &ffprobe,
        &format!(
            "#!/bin/sh\nprintf started > {}\nexec sleep 30\n",
            shell_quote(&handshake)
        ),
    );

    let child = Command::new(env!("CARGO_BIN_EXE_srt-autosync"))
        .args([
            input.to_str().unwrap(),
            media.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--ffmpeg",
            ffmpeg.to_str().unwrap(),
            "--ffprobe",
            ffprobe.to_str().unwrap(),
            "--quiet",
        ])
        .env("HOME", &isolated_home)
        .env("XDG_CACHE_HOME", &cache)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(Some(child));

    let handshake_deadline = Instant::now() + Duration::from_secs(5);
    while !handshake.exists() {
        assert!(Instant::now() < handshake_deadline, "ffprobe did not start");
        if let Some(status) = child.0.as_mut().unwrap().try_wait().unwrap() {
            panic!("CLI exited before ffprobe handshake: {status}");
        }
        thread::sleep(Duration::from_millis(10));
    }

    let signal = Command::new("kill")
        .args(["-INT", &child.0.as_ref().unwrap().id().to_string()])
        .status()
        .unwrap();
    assert!(signal.success(), "failed to send SIGINT to CLI child");

    let exit_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.0.as_mut().unwrap().try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < exit_deadline,
            "CLI did not exit after SIGINT"
        );
        thread::sleep(Duration::from_millis(10));
    }

    let result = child.0.take().unwrap().wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(130));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cancelled"));
    assert_eq!(std::fs::read_to_string(output).unwrap(), "existing output");
}
