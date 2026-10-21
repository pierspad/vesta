use std::{
    io::{Read, Write},
    process::Command,
};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_srt-refine"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn enrich_help_documents_the_json_contract_and_output_conversions() {
    let output = run(&["enrich", "--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for detail in [
        "non-empty `prompt`",
        "batch_size` defaults to 8",
        "YOUR_API_KEY",
        "APKG to APKG",
    ] {
        assert!(help.contains(detail), "missing {detail} in help:\n{help}");
    }
}

#[test]
fn inspect_help_shows_a_no_provider_example() {
    let output = run(&["inspect", "--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("srt-refine inspect --input deck.apkg"));
    assert!(help.contains("without contacting any provider"));
}

fn deck(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("deck.tsv");
    std::fs::write(
        &path,
        "#columns:Expression\tMeaning\tNotes\n日本語\tJapanese\tOriginal\n",
    )
    .unwrap();
    path
}

fn config(dir: &std::path::Path, url: &str) -> std::path::PathBuf {
    let path = dir.join("config.json");
    std::fs::write(&path, serde_json::json!({"tiers":[[{"provider":"local","model":"test","api_url":url}]],"prompt":"Explain {{expression}}"}).to_string()).unwrap();
    path
}

fn server(content: &str) -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let body = serde_json::json!({"choices":[{"message":{"role":"assistant","content":content}}]})
        .to_string();
    let task = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let start = std::time::Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        && start.elapsed().as_secs() < 5 =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(10))
                }
                Err(e) => panic!("Expected local provider request: {e}"),
            }
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = [0; 8192];
        assert!(stream.read(&mut request).unwrap() > 0);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    (url, task)
}

#[test]
fn inspect_emits_only_json_and_preserves_unicode() {
    let dir = tempfile::tempdir().unwrap();
    let input = deck(dir.path());
    let result = run(&["inspect", "--input", input.to_str().unwrap()]);
    assert!(result.status.success());
    let cards: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(cards[0]["expression"], "日本語");
    assert_eq!(cards[0]["notes"], "Original");
}

#[test]
fn enrichment_uses_the_shared_engine_and_keeps_source_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let input = deck(dir.path());
    let original = std::fs::read(&input).unwrap();
    let output = dir.path().join("annotated.tsv");
    let (url, task) = server("Explanation");
    let configuration = config(dir.path(), &url);
    let result = run(&[
        "enrich",
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--config",
        configuration.to_str().unwrap(),
    ]);
    task.join().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(input).unwrap(), original);
    assert!(
        std::fs::read_to_string(output)
            .unwrap()
            .contains("Explanation")
    );
    let summary: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(summary["summary"]["done"], 1);
}

#[test]
fn incomplete_run_does_not_publish_output_without_explicit_partial_policy() {
    for partial in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let input = deck(dir.path());
        let output = dir.path().join("annotated.tsv");
        std::fs::write(&output, "existing").unwrap();
        let (url, task) = server("");
        let configuration = config(dir.path(), &url);
        let mut args = vec![
            "enrich",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--config",
            configuration.to_str().unwrap(),
        ];
        if partial {
            args.push("--allow-partial");
        }
        let result = run(&args);
        task.join().unwrap();
        assert!(!result.status.success());
        assert_eq!(std::fs::read_to_string(output).unwrap(), "existing");
        let summary: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(summary["summary"]["failed"], 1);
        assert!(summary["output"].is_null());
    }
}

#[test]
fn malformed_configuration_fails_without_leaking_its_contents() {
    let dir = tempfile::tempdir().unwrap();
    let input = deck(dir.path());
    let configuration = dir.path().join("config.json");
    std::fs::write(&configuration, "PRIVATE-KEY invalid json").unwrap();
    let output = dir.path().join("annotated.tsv");
    let result = run(&[
        "enrich",
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--config",
        configuration.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(!output.exists());
    assert!(!String::from_utf8_lossy(&result.stderr).contains("PRIVATE-KEY"));
}

#[test]
fn partial_publication_requires_explicit_opt_in_and_still_reports_failure() {
    for partial in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let input = deck(dir.path());
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&input)
            .unwrap();
        writeln!(file, "Second\tMeaning\tKeep me").unwrap();
        let output = dir.path().join("annotated.tsv");
        std::fs::write(&output, "existing").unwrap();
        let (url, task) = server("Explanation");
        let configuration = config(dir.path(), &url);
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&configuration).unwrap()).unwrap();
        value["tiers"][0][0]["max_requests"] = 1.into();
        std::fs::write(&configuration, value.to_string()).unwrap();
        let mut args = vec![
            "enrich",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--config",
            configuration.to_str().unwrap(),
        ];
        if partial {
            args.push("--allow-partial");
        }
        let result = run(&args);
        task.join().unwrap();
        assert_eq!(result.status.code(), Some(1));
        let summary: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(summary["summary"]["remaining"], 1);
        if partial {
            let saved = std::fs::read_to_string(&output).unwrap();
            assert!(saved.contains("Explanation"));
            assert!(saved.contains("Keep me"));
            assert!(summary["output"].is_string());
        } else {
            assert_eq!(std::fs::read_to_string(&output).unwrap(), "existing");
            assert!(summary["output"].is_null());
        }
    }
}

#[test]
fn invalid_output_conversion_is_rejected_before_any_provider_request() {
    let dir = tempfile::tempdir().unwrap();
    let input = deck(dir.path());
    let output = dir.path().join("invalid.apkg");
    let configuration = config(dir.path(), "http://127.0.0.1:1");
    let result = run(&[
        "enrich",
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--config",
        configuration.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Supported conversions"));
    assert!(!output.exists());
}

#[test]
fn batch_configuration_updates_multiple_cards_from_one_provider_response() {
    let dir = tempfile::tempdir().unwrap();
    let input = deck(dir.path());
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&input)
        .unwrap();
    writeln!(file, "Second\tMeaning\tOriginal second").unwrap();
    let output = dir.path().join("annotated.tsv");
    let response = serde_json::json!({"results":[{"id":"1","notes":"First annotation"},{"id":"2","notes":"Second annotation"}]}).to_string();
    let (url, task) = server(&response);
    let configuration = config(dir.path(), &url);
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&configuration).unwrap()).unwrap();
    value["batch_mode"] = true.into();
    value["batch_size"] = 2.into();
    std::fs::write(&configuration, value.to_string()).unwrap();
    let result = run(&[
        "enrich",
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--config",
        configuration.to_str().unwrap(),
    ]);
    task.join().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let saved = std::fs::read_to_string(output).unwrap();
    assert!(saved.contains("日本語\tJapanese\tFirst annotation"));
    assert!(saved.contains("Second\tMeaning\tSecond annotation"));
    let summary: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(summary["summary"]["done"], 2);
}

#[cfg(unix)]
#[test]
fn ctrl_c_cancels_inflight_work_without_publishing() {
    use std::process::Stdio;
    let dir = tempfile::tempdir().unwrap();
    let input = deck(dir.path());
    let output = dir.path().join("annotated.tsv");
    std::fs::write(&output, "existing").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let configuration = config(
        dir.path(),
        &format!("http://{}/v1", listener.local_addr().unwrap()),
    );
    let (started, ready) = std::sync::mpsc::channel();
    let task = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut stream = loop {
            if let Ok((stream, _)) = listener.accept() {
                break stream;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "CLI did not contact local server"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut buffer = [0; 8192];
        assert!(stream.read(&mut buffer).unwrap() > 0);
        started.send(()).unwrap();
        while let Ok(count) = stream.read(&mut buffer) {
            if count == 0 {
                break;
            }
        }
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_srt-refine"))
        .args([
            "enrich",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--config",
            configuration.to_str().unwrap(),
            "--allow-partial",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if ready
        .recv_timeout(std::time::Duration::from_secs(5))
        .is_err()
    {
        let _ = child.kill();
        let _ = child.wait();
        panic!("CLI did not begin refinement");
    }
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("CLI ignored cancellation");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let result = child.wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(130));
    assert_eq!(std::fs::read_to_string(output).unwrap(), "existing");
    let summary: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(summary["summary"]["cancelled"], true);
    task.join().unwrap();
}
