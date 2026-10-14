use std::process::Command;

#[test]
fn json_failure_is_one_machine_readable_line_and_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("missing.wav");
    let output = dir.path().join("transcript.srt");
    std::fs::write(&output, "existing output").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_srt-transcribe"))
        .current_dir(dir.path())
        .args([
            "run",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1));
    let stdout = String::from_utf8(result.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "unexpected JSON output: {stdout:?}");
    let value: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["cancelled"], false);
    assert!(value["error"].as_str().unwrap().contains("missing.wav"));
    assert!(result.stderr.is_empty());
    assert_eq!(std::fs::read_to_string(output).unwrap(), "existing output");
}

#[cfg(unix)]
#[test]
fn json_sigint_emits_cancelled_terminal_record_without_stderr_noise() {
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
    let input = dir.path().join("media.wav");
    let output = dir.path().join("transcript.srt");
    let ffmpeg = dir.path().join("ffmpeg");
    let handshake = dir.path().join("ffmpeg.started");
    std::fs::write(&input, b"media stub").unwrap();
    std::fs::write(&output, "existing transcript").unwrap();
    std::fs::write(
        &ffmpeg,
        format!(
            "#!/bin/sh\nprintf started > '{}'\nexec sleep 30\n",
            handshake.to_string_lossy().replace('\'', "'\\''")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&ffmpeg, std::fs::Permissions::from_mode(0o700)).unwrap();

    let child = Command::new(env!("CARGO_BIN_EXE_srt-transcribe"))
        .current_dir(dir.path())
        .args([
            "run",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--provider",
            "custom",
            "--api-key",
            "fake-offline-key",
            "--api-url",
            "http://127.0.0.1:1/v1",
            "--ffmpeg",
            ffmpeg.to_str().unwrap(),
            "--json",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(Some(child));

    let handshake_deadline = Instant::now() + Duration::from_secs(5);
    while !handshake.exists() {
        assert!(Instant::now() < handshake_deadline, "ffmpeg did not start");
        if let Some(status) = child.0.as_mut().unwrap().try_wait().unwrap() {
            panic!("CLI exited before ffmpeg handshake: {status}");
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
    assert!(result.stderr.is_empty());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let records: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        records.len() >= 2,
        "expected progress and terminal JSON: {stdout}"
    );
    let terminal = records.last().unwrap();
    assert_eq!(terminal["ok"], false);
    assert_eq!(terminal["cancelled"], true);
    assert!(terminal["error"].as_str().unwrap().contains("cancelled"));
    assert_eq!(
        std::fs::read_to_string(output).unwrap(),
        "existing transcript"
    );
}
