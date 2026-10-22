use std::process::Command;

fn subtitle(text: &str) -> String {
    format!("1\n00:00:01,000 --> 00:00:02,000\n{text}\n")
}

fn config(path: &std::path::Path, url: &str) {
    std::fs::write(
        path,
        format!(
            "[translation]\nbatch_size = 1\n\n[[tiers]]\n[[tiers.entries]]\nprovider = \"local\"\nmodel = \"offline-test\"\napi_url = \"{url}\"\nrpm = 0\n"
        ),
    )
    .unwrap();
}

#[test]
fn language_list_is_a_standalone_mode_and_rejects_ignored_options() {
    let help = Command::new(env!("CARGO_BIN_EXE_srt-translate"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("--language-list is standalone"));

    let dir = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_srt-translate"))
        .current_dir(dir.path())
        .arg("--language-list")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("English"));
    assert!(!dir.path().join("config.toml").exists());

    let invalid = Command::new(env!("CARGO_BIN_EXE_srt-translate"))
        .args(["--language-list", "--check-missing", "translated.srt"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("cannot be used with"));
}

#[test]
fn incomplete_check_returns_failure_for_scripts() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("original.srt");
    let translated = dir.path().join("translated.srt");
    std::fs::write(
        &input,
        "1\n00:00:01,000 --> 00:00:02,000\nOne\n\n2\n00:00:03,000 --> 00:00:04,000\nTwo\n",
    )
    .unwrap();
    std::fs::write(&translated, subtitle("Uno")).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_srt-translate"))
        .current_dir(dir.path())
        .args([
            "--input",
            input.to_str().unwrap(),
            "--check-missing",
            translated.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stdout).contains("Found 1 missing subtitles"));
    assert!(result.stderr.is_empty());
}

#[test]
#[cfg(unix)]
fn output_symlink_to_input_is_rejected_before_translation() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("movie.srt");
    let output_alias = dir.path().join("movie-alias.srt");
    let configuration = dir.path().join("config.toml");
    std::fs::write(&input, subtitle("Hello")).unwrap();
    symlink(&input, &output_alias).unwrap();
    config(&configuration, "http://127.0.0.1:1/v1");
    let before = std::fs::read(&input).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_srt-translate"))
        .current_dir(dir.path())
        .args([
            "--input",
            input.to_str().unwrap(),
            "--language",
            "it",
            "--config",
            configuration.to_str().unwrap(),
            "--output",
            output_alias.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("same file"));
    assert_eq!(std::fs::read(input).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn output_hardlink_to_input_is_rejected_before_translation() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("movie.srt");
    let output_alias = dir.path().join("movie-hardlink.srt");
    let configuration = dir.path().join("config.toml");
    std::fs::write(&input, subtitle("Hello")).unwrap();
    std::fs::hard_link(&input, &output_alias).unwrap();
    config(&configuration, "http://127.0.0.1:1/v1");
    let before = std::fs::read(&input).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_srt-translate"))
        .current_dir(dir.path())
        .args([
            "--input",
            input.to_str().unwrap(),
            "--language",
            "it",
            "--config",
            configuration.to_str().unwrap(),
            "--output",
            output_alias.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("same file"));
    assert_eq!(std::fs::read(input).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn ctrl_c_cancels_translation_with_130_and_keeps_last_checkpoint() {
    use std::{
        io::Read,
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
    let input = dir.path().join("movie.srt");
    let output = dir.path().join("movie.it.srt");
    let configuration = dir.path().join("config.toml");
    let handshake = dir.path().join("request.started");
    let before = subtitle("Old translation");
    std::fs::write(&input, subtitle("Hello")).unwrap();
    std::fs::write(&output, &before).unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    config(&configuration, &url);
    let server_handshake = handshake.clone();
    let server = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("Expected local translation request: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut buffer = [0; 4096];
        assert!(stream.read(&mut buffer).unwrap() > 0);
        std::fs::write(server_handshake, b"request received").unwrap();
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });

    let child = Command::new(env!("CARGO_BIN_EXE_srt-translate"))
        .current_dir(dir.path())
        .args([
            "--input",
            input.to_str().unwrap(),
            "--language",
            "it",
            "--config",
            configuration.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(Some(child));

    let handshake_deadline = Instant::now() + Duration::from_secs(5);
    while !handshake.exists() {
        assert!(
            Instant::now() < handshake_deadline,
            "provider request did not start"
        );
        if let Some(status) = child.0.as_mut().unwrap().try_wait().unwrap() {
            panic!("CLI exited before provider handshake: {status}");
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
    server.join().unwrap();
    assert_eq!(result.status.code(), Some(130));
    assert!(String::from_utf8_lossy(&result.stderr).contains("cancelled"));
    assert_eq!(std::fs::read_to_string(output).unwrap(), before);
}
