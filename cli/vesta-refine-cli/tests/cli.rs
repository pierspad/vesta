use std::{
    io::{Read, Write},
    process::Command,
};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vesta-refine"))
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
    assert!(help.contains("vesta-refine inspect --input deck.apkg"));
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
    let mut child = Command::new(env!("CARGO_BIN_EXE_vesta-refine"))
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

#[test]
fn inspect_supports_case_insensitive_query_and_pagination() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("deck.tsv");
    std::fs::write(
        &input,
        concat!(
            "#columns:Expression\tMeaning\tNotes\n",
            "食べる\tto eat\tVerb ichidan\n",
            "飲む\tto drink\tVerb godan\n",
            "猫\tcat\tAnimal noun\n",
            "走る\tto run\tVERB movement\n",
        ),
    )
    .unwrap();

    // Unfiltered inspect preserves full JSON array output.
    let full = run(&["inspect", "--input", input.to_str().unwrap()]);
    assert!(full.status.success());
    let full_cards: Vec<serde_json::Value> = serde_json::from_slice(&full.stdout).unwrap();
    assert_eq!(full_cards.len(), 4);

    // Filter across notes case-insensitively and paginate.
    let filtered = run(&[
        "inspect",
        "--input",
        input.to_str().unwrap(),
        "--query",
        "verb",
        "--offset",
        "1",
        "--limit",
        "1",
    ]);
    assert!(
        filtered.status.success(),
        "{}",
        String::from_utf8_lossy(&filtered.stderr)
    );
    let page: Vec<serde_json::Value> = serde_json::from_slice(&filtered.stdout).unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0]["expression"], "飲む");

    // Match by meaning and by id.
    let by_meaning = run(&[
        "inspect",
        "--input",
        input.to_str().unwrap(),
        "--query",
        "ANIMAL",
    ]);
    let meaning_cards: Vec<serde_json::Value> = serde_json::from_slice(&by_meaning.stdout).unwrap();
    assert_eq!(meaning_cards.len(), 1);
    assert_eq!(meaning_cards[0]["expression"], "猫");

    let by_id = run(&[
        "inspect",
        "--input",
        input.to_str().unwrap(),
        "--query",
        "4",
    ]);
    let id_cards: Vec<serde_json::Value> = serde_json::from_slice(&by_id.stdout).unwrap();
    assert_eq!(id_cards.len(), 1);
    assert_eq!(id_cards[0]["expression"], "走る");

    let past_end = run(&[
        "inspect",
        "--input",
        input.to_str().unwrap(),
        "--offset",
        "99",
    ]);
    assert!(past_end.status.success());
    let empty_cards: Vec<serde_json::Value> = serde_json::from_slice(&past_end.stdout).unwrap();
    assert!(empty_cards.is_empty());
}

#[test]
fn edit_roundtrips_tsv_and_apkg_and_rejects_same_file_or_missing_id() {
    let dir = tempfile::tempdir().unwrap();
    let tsv_in = dir.path().join("cards.tsv");
    let tsv_out = dir.path().join("cards.edited.tsv");
    let original_tsv =
        "#columns:Expression\tMeaning\tNotes\n日本語\tJapanese\tOld note\n猫\tCat\tKeep me\n";
    std::fs::write(&tsv_in, original_tsv).unwrap();

    // Same file is rejected for `edit`.
    let same_file = run(&[
        "edit",
        "--input",
        tsv_in.to_str().unwrap(),
        "--output",
        tsv_in.to_str().unwrap(),
        "--id",
        "1",
        "--notes",
        "Updated",
    ]);
    assert_eq!(same_file.status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(&tsv_in).unwrap(), original_tsv);

    // Missing ID is rejected cleanly without creating output.
    let missing_id = run(&[
        "edit",
        "--input",
        tsv_in.to_str().unwrap(),
        "--output",
        tsv_out.to_str().unwrap(),
        "--id",
        "999",
        "--notes",
        "Updated",
    ]);
    assert_eq!(missing_id.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing_id.stderr).contains("999"));
    assert!(!tsv_out.exists());

    // Valid TSV edit updates target card and preserves source + sibling cards.
    let tsv_edit = run(&[
        "edit",
        "--input",
        tsv_in.to_str().unwrap(),
        "--output",
        tsv_out.to_str().unwrap(),
        "--id",
        "1",
        "--notes",
        "New TSV note",
    ]);
    assert!(
        tsv_edit.status.success(),
        "{}",
        String::from_utf8_lossy(&tsv_edit.stderr)
    );
    assert_eq!(std::fs::read_to_string(&tsv_in).unwrap(), original_tsv);
    let edited_tsv = std::fs::read_to_string(&tsv_out).unwrap();
    assert!(edited_tsv.contains("日本語\tJapanese\tNew TSV note"));
    assert!(edited_tsv.contains("猫\tCat\tKeep me"));

    // APKG -> APKG roundtrip using sample.apkg fixture.
    let apkg_in = dir.path().join("sample.apkg");
    let apkg_out = dir.path().join("edited.apkg");
    std::fs::write(&apkg_in, include_bytes!("fixtures/sample.apkg")).unwrap();

    let apkg_edit = run(&[
        "edit",
        "--input",
        apkg_in.to_str().unwrap(),
        "--output",
        apkg_out.to_str().unwrap(),
        "--id",
        "101",
        "--notes",
        "Updated APKG note",
    ]);
    assert!(
        apkg_edit.status.success(),
        "{}",
        String::from_utf8_lossy(&apkg_edit.stderr)
    );
    let edit_json: serde_json::Value = serde_json::from_slice(&apkg_edit.stdout).unwrap();
    assert_eq!(edit_json["ok"], true);
    assert_eq!(edit_json["id"], "101");

    let inspected = run(&["inspect", "--input", apkg_out.to_str().unwrap()]);
    assert!(inspected.status.success());
    let cards: Vec<serde_json::Value> = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0]["id"], "101");
    assert_eq!(cards[0]["notes"], "Updated APKG note");
    assert_eq!(cards[1]["id"], "102");
    assert_eq!(cards[1]["notes"], "Verb grammar");

    // Source APKG remains unchanged.
    let inspected_src = run(&["inspect", "--input", apkg_in.to_str().unwrap()]);
    let src_cards: Vec<serde_json::Value> = serde_json::from_slice(&inspected_src.stdout).unwrap();
    assert_eq!(src_cards[0]["notes"], "First note");
}

fn mock_ankiconnect_server(
    expected_actions: usize,
) -> (
    String,
    std::sync::mpsc::Receiver<serde_json::Value>,
    std::thread::JoinHandle<()>,
) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        for _ in 0..expected_actions {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut raw = Vec::new();
            let mut buf = [0u8; 2048];
            let header_end = loop {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0, "unexpected EOF before HTTP headers");
                raw.extend_from_slice(&buf[..n]);
                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
            };
            let headers = String::from_utf8_lossy(&raw[..header_end]);
            let content_length: usize = headers
                .lines()
                .find_map(|line| {
                    let (k, v) = line.split_once(':')?;
                    k.eq_ignore_ascii_case("content-length")
                        .then(|| v.trim().parse().ok())
                        .flatten()
                })
                .unwrap_or(0);
            while raw.len() - header_end < content_length {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0, "unexpected EOF reading HTTP body");
                raw.extend_from_slice(&buf[..n]);
            }
            let payload: serde_json::Value =
                serde_json::from_slice(&raw[header_end..header_end + content_length]).unwrap();
            tx.send(payload.clone()).unwrap();

            let resp_body = match payload["action"].as_str().unwrap_or("") {
                "version" => serde_json::json!({"result": 6, "error": null}).to_string(),
                "deckNames" => {
                    serde_json::json!({"result": ["Default", "Vesta::Japanese"], "error": null})
                        .to_string()
                }
                "importPackage" => serde_json::json!({"result": true, "error": null}).to_string(),
                other => panic!("unexpected AnkiConnect action: {other}"),
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
                resp_body.len()
            )
            .unwrap();
        }
    });
    (url, rx, handle)
}

#[test]
fn ankiconnect_ping_decks_and_import_delegate_via_http() {
    let dir = tempfile::tempdir().unwrap();
    let apkg_path = dir.path().join("sample.apkg");
    std::fs::write(&apkg_path, include_bytes!("fixtures/sample.apkg")).unwrap();

    // Missing file fails before contacting server.
    let missing = dir.path().join("missing.apkg");
    let missing_run = run(&[
        "ankiconnect",
        "--url",
        "http://127.0.0.1:1",
        "import",
        "--apkg",
        missing.to_str().unwrap(),
    ]);
    assert_eq!(missing_run.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing_run.stderr).contains("APKG file not found"));

    let (url, rx, handle) = mock_ankiconnect_server(3);

    let ping = run(&["ankiconnect", "--url", &url, "ping"]);
    assert!(
        ping.status.success(),
        "{}",
        String::from_utf8_lossy(&ping.stderr)
    );
    let ping_json: serde_json::Value = serde_json::from_slice(&ping.stdout).unwrap();
    assert_eq!(ping_json["ok"], true);
    assert_eq!(ping_json["version"], 6);

    let decks = run(&["ankiconnect", "--url", &url, "decks"]);
    assert!(
        decks.status.success(),
        "{}",
        String::from_utf8_lossy(&decks.stderr)
    );
    let decks_json: Vec<String> = serde_json::from_slice(&decks.stdout).unwrap();
    assert_eq!(decks_json, vec!["Default", "Vesta::Japanese"]);

    // Pass a non-canonical relative path (via current_dir) to verify canonicalization.
    let import = Command::new(env!("CARGO_BIN_EXE_vesta-refine"))
        .current_dir(dir.path())
        .args([
            "ankiconnect",
            "--url",
            &url,
            "import",
            "--apkg",
            "./sample.apkg",
        ])
        .output()
        .unwrap();
    assert!(
        import.status.success(),
        "{}",
        String::from_utf8_lossy(&import.stderr)
    );
    let canonical = std::fs::canonicalize(&apkg_path).unwrap();
    let import_json: serde_json::Value = serde_json::from_slice(&import.stdout).unwrap();
    assert_eq!(import_json["ok"], true);
    assert_eq!(import_json["apkg"], canonical.to_string_lossy().as_ref());

    handle.join().unwrap();
    let reqs: Vec<serde_json::Value> = rx.try_iter().collect();
    assert_eq!(reqs.len(), 3);
    assert_eq!(reqs[0]["action"], "version");
    assert_eq!(reqs[0]["version"], 6);
    assert_eq!(reqs[1]["action"], "deckNames");
    assert_eq!(reqs[2]["action"], "importPackage");
    assert_eq!(
        reqs[2]["params"]["path"],
        canonical.to_string_lossy().as_ref()
    );
}
