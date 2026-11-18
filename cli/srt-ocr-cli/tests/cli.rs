use std::path::{Path, PathBuf};
use std::process::Command;

fn pgs_segment(pts_ms: u64, seg_type: u8, payload: &[u8]) -> Vec<u8> {
    let pts_90k = ((pts_ms * 90_000) / 1000) as u32;
    let mut out = Vec::with_capacity(13 + payload.len());
    out.extend_from_slice(b"PG");
    out.extend_from_slice(&pts_90k.to_be_bytes());
    out.extend_from_slice(&pts_90k.to_be_bytes());
    out.push(seg_type);
    out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn create_single_cue_pgs_mkv(dir: &Path) -> PathBuf {
    let mut sup = Vec::new();
    let mut pds = vec![0u8, 0u8];
    pds.extend_from_slice(&[0, 16, 128, 128, 0, 1, 235, 128, 128, 255]);
    let mut rle = Vec::new();
    for _ in 0..8 {
        rle.extend_from_slice(&[0x00, 0x80 | 20, 1, 0x00, 0x00]);
    }
    let obj_len = (4 + rle.len()) as u32;
    let mut ods = vec![
        0,
        0,
        0,
        0xC0,
        ((obj_len >> 16) & 0xFF) as u8,
        ((obj_len >> 8) & 0xFF) as u8,
        (obj_len & 0xFF) as u8,
    ];
    ods.extend_from_slice(&20u16.to_be_bytes());
    ods.extend_from_slice(&8u16.to_be_bytes());
    ods.extend_from_slice(&rle);

    let mut pcs = Vec::new();
    pcs.extend_from_slice(&1280u16.to_be_bytes());
    pcs.extend_from_slice(&720u16.to_be_bytes());
    pcs.push(0x10);
    pcs.extend_from_slice(&1u16.to_be_bytes());
    pcs.push(0x80);
    pcs.push(0x00);
    pcs.push(0x00);
    pcs.push(1);
    pcs.extend_from_slice(&0u16.to_be_bytes());
    pcs.push(0);
    pcs.push(0);
    pcs.extend_from_slice(&100u16.to_be_bytes());
    pcs.extend_from_slice(&600u16.to_be_bytes());

    sup.extend(pgs_segment(1000, 0x16, &pcs));
    sup.extend(pgs_segment(1000, 0x14, &pds));
    sup.extend(pgs_segment(1000, 0x15, &ods));
    sup.extend(pgs_segment(1000, 0x80, &[]));

    let mut clear_pcs = Vec::new();
    clear_pcs.extend_from_slice(&1280u16.to_be_bytes());
    clear_pcs.extend_from_slice(&720u16.to_be_bytes());
    clear_pcs.push(0x10);
    clear_pcs.extend_from_slice(&10u16.to_be_bytes());
    clear_pcs.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    sup.extend(pgs_segment(2500, 0x16, &clear_pcs));
    sup.extend(pgs_segment(2500, 0x80, &[]));

    let sup_path = dir.join("cue.sup");
    std::fs::write(&sup_path, sup).unwrap();
    let mkv_path = dir.join("cue.mkv");
    let status = Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-copyts",
            "-f",
            "lavfi",
            "-i",
            "color=size=1280x720:duration=4:rate=1",
            "-i",
        ])
        .arg(&sup_path)
        .args([
            "-map",
            "0:v",
            "-map",
            "1:s",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-c:s",
            "copy",
            "-y",
        ])
        .arg(&mkv_path)
        .status()
        .expect("ffmpeg is required for srt-ocr-cli tests");
    assert!(status.success());
    mkv_path
}

#[test]
fn top_level_and_run_help_expose_subcommands_and_flags() {
    let top = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(top.status.success());
    let top_text = String::from_utf8_lossy(&top.stdout);
    for sub in ["run", "models", "download", "remove"] {
        assert!(top_text.contains(sub), "missing {sub} in:\n{top_text}");
    }

    let run = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .args(["run", "--help"])
        .output()
        .unwrap();
    assert!(run.status.success());
    let run_text = String::from_utf8_lossy(&run.stdout);
    for flag in [
        "--input",
        "--output",
        "--track",
        "--provider",
        "--model",
        "--api-key",
        "--api-url",
        "--language",
        "--ffmpeg",
        "--ffprobe",
        "--quiet",
        "--json",
    ] {
        assert!(run_text.contains(flag), "missing {flag} in:\n{run_text}");
    }
}

#[test]
fn models_and_remove_work_with_isolated_cache_dir() {
    let cache = tempfile::tempdir().unwrap();
    let list_out = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .env("VESTA_OCR_CACHE_DIR", cache.path())
        .arg("models")
        .output()
        .unwrap();
    assert!(
        list_out.status.success(),
        "{}",
        String::from_utf8_lossy(&list_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    for id in [
        "paddleocr-v4-mobile-multi",
        "paddleocr-v4-mobile-cjk",
        "paddleocr-v4-mobile-latin",
    ] {
        assert!(stdout.contains(id), "missing model id {id} in:\n{stdout}");
    }
    assert!(!stdout.contains("[downloaded]"));

    let model_file = cache.path().join("paddleocr-v4-mobile-latin.onnx");
    std::fs::write(&model_file, b"stub").unwrap();
    let list_installed = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .env("VESTA_OCR_CACHE_DIR", cache.path())
        .arg("models")
        .output()
        .unwrap();
    assert!(list_installed.status.success());
    assert!(String::from_utf8_lossy(&list_installed.stdout).contains("[downloaded]"));

    let remove_ok = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .env("VESTA_OCR_CACHE_DIR", cache.path())
        .args(["remove", "paddleocr-v4-mobile-latin"])
        .output()
        .unwrap();
    assert!(remove_ok.status.success());
    assert!(String::from_utf8_lossy(&remove_ok.stdout).contains("Model removed."));
    assert!(!model_file.exists());

    let remove_absent = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .env("VESTA_OCR_CACHE_DIR", cache.path())
        .args(["remove", "paddleocr-v4-mobile-latin"])
        .output()
        .unwrap();
    assert!(remove_absent.status.success());
    assert!(String::from_utf8_lossy(&remove_absent.stdout).contains("Model was not installed."));
}

#[test]
fn rejects_cloud_provider_without_api_key_in_text_and_json_modes() {
    for provider in ["openai", "google", "anthropic"] {
        let text_res = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
            .args([
                "run",
                "--input",
                "missing.mkv",
                "--output",
                "out.srt",
                "--track",
                "1",
                "--provider",
                provider,
            ])
            .output()
            .unwrap();
        assert_eq!(text_res.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&text_res.stderr).contains("--api-key is required"),
            "unexpected stderr for {provider}: {}",
            String::from_utf8_lossy(&text_res.stderr)
        );

        let json_res = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
            .args([
                "run",
                "--input",
                "missing.mkv",
                "--output",
                "out.srt",
                "--track",
                "1",
                "--provider",
                provider,
                "--json",
            ])
            .output()
            .unwrap();
        assert_eq!(json_res.status.code(), Some(1));
        assert!(json_res.stderr.is_empty());
        let stdout = String::from_utf8(json_res.stdout).unwrap();
        let lines: Vec<_> = stdout.lines().collect();
        assert_eq!(lines.len(), 1, "unexpected JSON output: {stdout:?}");
        let terminal: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(terminal["ok"], false);
        assert_eq!(terminal["cancelled"], false);
        assert!(
            terminal["error"]
                .as_str()
                .unwrap()
                .contains("--api-key is required")
        );
    }
}

#[test]
fn rejects_custom_provider_without_api_url_in_text_and_json_modes() {
    let text_res = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .args([
            "run",
            "--input",
            "missing.mkv",
            "--output",
            "out.srt",
            "--track",
            "1",
            "--provider",
            "custom",
        ])
        .output()
        .unwrap();
    assert_eq!(text_res.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&text_res.stderr).contains("--api-url is required"));

    let json_res = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .args([
            "run",
            "--input",
            "missing.mkv",
            "--output",
            "out.srt",
            "--track",
            "1",
            "--provider",
            "custom",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(json_res.status.code(), Some(1));
    assert!(json_res.stderr.is_empty());
    let stdout = String::from_utf8(json_res.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 1);
    let terminal: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(terminal["ok"], false);
    assert_eq!(terminal["cancelled"], false);
    assert!(
        terminal["error"]
            .as_str()
            .unwrap()
            .contains("--api-url is required")
    );
}

#[test]
fn rejects_identical_input_and_output_files() {
    let dir = tempfile::tempdir().unwrap();
    let same = dir.path().join("media.mkv");
    std::fs::write(&same, b"stub media").unwrap();

    let text_res = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .args([
            "run",
            "--input",
            same.to_str().unwrap(),
            "--output",
            same.to_str().unwrap(),
            "--track",
            "1",
        ])
        .output()
        .unwrap();
    assert_eq!(text_res.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&text_res.stderr).contains("same file"));
    assert_eq!(std::fs::read(&same).unwrap(), b"stub media");

    let json_res = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .args([
            "run",
            "--input",
            same.to_str().unwrap(),
            "--output",
            same.to_str().unwrap(),
            "--track",
            "1",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(json_res.status.code(), Some(1));
    assert!(json_res.stderr.is_empty());
    let stdout = String::from_utf8(json_res.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 1);
    let terminal: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(terminal["ok"], false);
    assert_eq!(terminal["cancelled"], false);
    assert!(terminal["error"].as_str().unwrap().contains("same file"));
    assert_eq!(std::fs::read(&same).unwrap(), b"stub media");
}

#[test]
fn end_to_end_json_ocr_against_loopback_vision_endpoint() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    let dir = tempfile::tempdir().unwrap();
    let mkv_path = create_single_cue_pgs_mkv(dir.path());
    let out_srt = dir.path().join("out.srt");

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let header_end = loop {
            let n = stream.read(&mut chunk).unwrap();
            assert!(n > 0);
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos + 4;
            }
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (k, v) = line.split_once(':')?;
                k.trim()
                    .eq_ignore_ascii_case("content-length")
                    .then(|| v.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        while buf.len() < header_end + content_length {
            let n = stream.read(&mut chunk).unwrap();
            assert!(n > 0);
            buf.extend_from_slice(&chunk[..n]);
        }
        let body =
            r#"{"choices":[{"message":{"role":"assistant","content":"Hello from OCR CLI"}}]}"#;
        let http = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(http.as_bytes()).unwrap();
    });

    let result = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .args([
            "run",
            "--input",
            mkv_path.to_str().unwrap(),
            "--output",
            out_srt.to_str().unwrap(),
            "--track",
            "1",
            "--provider",
            "custom",
            "--model",
            "vision-test",
            "--api-url",
            &format!("http://{addr}/v1"),
            "--language",
            "en",
            "--json",
        ])
        .output()
        .unwrap();

    server.join().unwrap();
    assert!(
        result.status.success(),
        "stderr={}\nstdout={}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    assert!(result.stderr.is_empty());

    let stdout = String::from_utf8(result.stdout).unwrap();
    let records: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(records.len() >= 2, "expected progress + terminal: {stdout}");
    let terminal = records.last().unwrap();
    assert_eq!(terminal["ok"], true);
    assert_eq!(terminal["outcome"]["subtitle_count"], 1);

    let srt_content = std::fs::read_to_string(&out_srt).unwrap();
    assert!(srt_content.contains("Hello from OCR CLI"), "{srt_content}");
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
    let input = dir.path().join("media.mkv");
    let output = dir.path().join("out.srt");
    let ffprobe = dir.path().join("ffprobe");
    let handshake = dir.path().join("ffprobe.started");
    std::fs::write(&input, b"media stub").unwrap();
    std::fs::write(&output, "existing output").unwrap();
    std::fs::write(
        &ffprobe,
        format!(
            "#!/bin/sh\nprintf started > '{}'\nexec sleep 30\n",
            handshake.to_string_lossy().replace('\'', "'\\''")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&ffprobe, std::fs::Permissions::from_mode(0o700)).unwrap();

    let child = Command::new(env!("CARGO_BIN_EXE_srt-ocr"))
        .current_dir(dir.path())
        .args([
            "run",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--track",
            "1",
            "--provider",
            "custom",
            "--api-url",
            "http://127.0.0.1:1/v1",
            "--ffprobe",
            ffprobe.to_str().unwrap(),
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
    assert_eq!(std::fs::read_to_string(output).unwrap(), "existing output");
}
