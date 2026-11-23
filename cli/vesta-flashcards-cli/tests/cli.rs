use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vesta-flashcards"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn each_leaf_help_explains_usage_and_preview_boundaries() {
    for (command, expected) in [
        (
            "generate",
            "A media source is needed only for enabled media outputs",
        ),
        ("info", "vesta-flashcards info subtitles.srt"),
        ("preview", "never creates the --output directory"),
    ] {
        let output = run(&[command, "--help"]);
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(expected),
            "{command} help missing {expected}"
        );
    }
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
    let result = Command::new(env!("CARGO_BIN_EXE_vesta-flashcards"))
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

    let child = Command::new(env!("CARGO_BIN_EXE_vesta-flashcards"))
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

#[test]
fn preview_works_with_or_without_output_and_accepts_new_flags() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    subtitles(&input);

    // Preview without --output
    let preview_no_output = run(&[
        "preview",
        "--target",
        input.to_str().unwrap(),
        "--pad-audio-start",
        "50",
        "--pad-audio-end",
        "50",
        "--audio-gain",
        "3",
        "--actor-filter",
        "Alice,Bob",
    ]);
    assert!(
        preview_no_output.status.success(),
        "{}",
        String::from_utf8_lossy(&preview_no_output.stderr)
    );
    assert!(preview_no_output.stderr.is_empty());
    assert!(String::from_utf8_lossy(&preview_no_output.stdout).contains("Total lines:"));

    // Preview with --output still works
    let out_path = dir.path().join("preview_out");
    let preview_with_output = run(&[
        "preview",
        "--target",
        input.to_str().unwrap(),
        "--output",
        out_path.to_str().unwrap(),
    ]);
    assert!(
        preview_with_output.status.success(),
        "{}",
        String::from_utf8_lossy(&preview_with_output.stderr)
    );
    assert!(
        !out_path.exists(),
        "preview must not create output directory"
    );
}

#[test]
fn generate_requires_output_argument() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    subtitles(&input);

    let result = run(&["generate", "--target", input.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains(
        "error: the following required arguments were not provided:\n  --output <OUTPUT>"
    ));
}

#[test]
fn dueling_exports_ass_and_srt_files() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.srt");
    let native = dir.path().join("native.srt");
    let ass_out = dir.path().join("bilingual.ass");
    let srt_out = dir.path().join("bilingual.srt");
    subtitles(&target);
    std::fs::write(&native, "1\n00:00:01,100 --> 00:00:01,900\nCiao!\n").unwrap();

    let ass_run = run(&[
        "dueling",
        "--target",
        target.to_str().unwrap(),
        "--native",
        native.to_str().unwrap(),
        "--dueling-format",
        "ass",
        "--output",
        ass_out.to_str().unwrap(),
    ]);
    assert!(
        ass_run.status.success(),
        "{}",
        String::from_utf8_lossy(&ass_run.stderr)
    );
    let ass_text = std::fs::read_to_string(&ass_out).unwrap();
    assert!(ass_text.contains("[V4+ Styles]"));
    assert!(ass_text.contains("Dialogue: 0,0:00:01.00,0:00:02.00,Subs1,,0,0,0,,Hello there"));
    assert!(ass_text.contains("Dialogue: 0,0:00:01.00,0:00:02.00,Subs2,,0,0,0,,Ciao!"));

    let srt_run = run(&[
        "dueling",
        "--target",
        target.to_str().unwrap(),
        "--native",
        native.to_str().unwrap(),
        "--dueling-format",
        "srt",
        "--native-first",
        "--output",
        srt_out.to_str().unwrap(),
    ]);
    assert!(
        srt_run.status.success(),
        "{}",
        String::from_utf8_lossy(&srt_run.stderr)
    );
    let srt_text = std::fs::read_to_string(&srt_out).unwrap();
    assert!(srt_text.contains("00:00:01,000 --> 00:00:02,000\nCiao!\nHello there"));
}

#[test]
fn generate_parity_flags_and_merge_subcommand_work_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let target1 = dir.path().join("ep1.srt");
    let target2 = dir.path().join("ep2.srt");
    std::fs::write(
        &target1,
        "1\n00:00:01,000 --> 00:00:02,000\nEpisode 1 line\n",
    )
    .unwrap();
    std::fs::write(
        &target2,
        "1\n00:00:01,000 --> 00:00:02,000\nEpisode 2 line\n",
    )
    .unwrap();

    let out1 = dir.path().join("out1");
    let out2 = dir.path().join("out2");

    let gen1 = run(&[
        "generate",
        "--target",
        target1.to_str().unwrap(),
        "--output",
        out1.to_str().unwrap(),
        "--format",
        "apkg",
        "--deck",
        "Ep1",
        "--note-type",
        "CustomVestaNote",
        "--target-language",
        "ja",
        "--no-dictionary-lookup",
        "--no-auto-card-font",
        "--no-embed-card-font",
        "--video-hw-accel",
        "off",
        "--pad-video-start=-100",
        "--pad-video-end",
        "250",
        "--no-audio",
        "--no-snapshots",
        "--no-video",
        "--quiet",
    ]);
    assert!(
        gen1.status.success(),
        "{}",
        String::from_utf8_lossy(&gen1.stderr)
    );
    let apkg1 = out1.join("Ep1.apkg");
    assert!(apkg1.exists());

    let gen2 = run(&[
        "generate",
        "--target",
        target2.to_str().unwrap(),
        "--output",
        out2.to_str().unwrap(),
        "--format",
        "apkg",
        "--deck",
        "Ep2",
        "--note-type",
        "CustomVestaNote",
        "--target-language",
        "ja",
        "--no-dictionary-lookup",
        "--no-auto-card-font",
        "--no-embed-card-font",
        "--no-audio",
        "--no-snapshots",
        "--no-video",
        "--quiet",
    ]);
    assert!(
        gen2.status.success(),
        "{}",
        String::from_utf8_lossy(&gen2.stderr)
    );
    let apkg2 = out2.join("Ep2.apkg");
    assert!(apkg2.exists());

    let merged = dir.path().join("merged.apkg");
    let merge_run = run(&[
        "merge",
        apkg1.to_str().unwrap(),
        apkg2.to_str().unwrap(),
        "--output",
        merged.to_str().unwrap(),
    ]);
    assert!(
        merge_run.status.success(),
        "{}",
        String::from_utf8_lossy(&merge_run.stderr)
    );
    assert!(std::fs::read(&merged).unwrap().starts_with(b"PK"));
    assert!(String::from_utf8_lossy(&merge_run.stdout).contains("Merged 2 APKG decks"));

    let alias_merged = dir.path().join("alias_merged.apkg");
    let alias_run = run(&[
        "merge-apkg",
        apkg1.to_str().unwrap(),
        apkg2.to_str().unwrap(),
        "--output",
        alias_merged.to_str().unwrap(),
    ]);
    assert!(alias_run.status.success());

    let bad_hw = run(&["generate", "--video-hw-accel", "bogus"]);
    assert_eq!(bad_hw.status.code(), Some(2));
    let bad_merge = run(&["merge", apkg1.to_str().unwrap(), "--output", "out.apkg"]);
    assert_eq!(bad_merge.status.code(), Some(2));
}

#[test]
fn preview_json_supports_filtering_and_offset_limit_pagination() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.srt");
    let native = dir.path().join("native.srt");
    let unused_output = dir.path().join("never-created");

    std::fs::write(
        &target,
        concat!(
            "1\n00:00:01,000 --> 00:00:02,000\nHi\n\n",
            "2\n00:00:03,000 --> 00:00:04,500\nSecond line\n\n",
            "3\n00:00:05,000 --> 00:00:06,200\nThird line here\n\n",
            "4\n00:00:07,000 --> 00:00:08,000\nFourth line\n",
        ),
    )
    .unwrap();
    std::fs::write(
        &native,
        concat!(
            "1\n00:00:03,100 --> 00:00:04,400\nSeconda riga\n\n",
            "2\n00:00:05,100 --> 00:00:06,100\nTerza riga\n",
        ),
    )
    .unwrap();

    let preview = run(&[
        "preview",
        "--target",
        target.to_str().unwrap(),
        "--native",
        native.to_str().unwrap(),
        "--output",
        unused_output.to_str().unwrap(),
        "--min-chars",
        "5",
        "--json",
        "--offset",
        "1",
        "--limit",
        "2",
    ]);
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert!(preview.stderr.is_empty());
    assert!(
        !unused_output.exists(),
        "preview --json must not create output dir"
    );

    let payload: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(payload["total"], 4);
    assert_eq!(payload["active"], 3);
    assert_eq!(payload["matched"], 2);
    assert_eq!(payload["offset"], 1);
    assert_eq!(payload["limit"], 2);

    let rows = payload["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["index"], 1);
    assert_eq!(rows[0]["start_ms"], 3000);
    assert_eq!(rows[0]["end_ms"], 4500);
    assert_eq!(rows[0]["duration_ms"], 1500);
    assert_eq!(rows[0]["subs1_text"], "Second line");
    assert_eq!(rows[0]["subs2_text"], "Seconda riga");
    assert_eq!(rows[0]["active"], true);
    assert!(rows[0]["actor"].is_null());
    assert!(rows[0]["leading_context"].is_array());
    assert!(rows[0]["trailing_context"].is_array());

    assert_eq!(rows[1]["index"], 2);
    assert_eq!(rows[1]["subs1_text"], "Third line here");
    assert_eq!(rows[1]["subs2_text"], "Terza riga");

    let empty_page = run(&[
        "preview",
        "--target",
        target.to_str().unwrap(),
        "--json",
        "--offset",
        "10",
        "--limit",
        "2",
    ]);
    assert!(empty_page.status.success());
    let empty_payload: serde_json::Value = serde_json::from_slice(&empty_page.stdout).unwrap();
    assert_eq!(empty_payload["total"], 4);
    assert_eq!(empty_payload["rows"].as_array().unwrap().len(), 0);
}

#[test]
fn batch_manifest_generates_episodes_and_merges_apkgs() {
    let dir = tempfile::tempdir().unwrap();
    let ep1 = dir.path().join("ep1.srt");
    let ep2 = dir.path().join("ep2.srt");
    std::fs::write(&ep1, "1\n00:00:01,000 --> 00:00:02,000\nEpisode one\n").unwrap();
    std::fs::write(&ep2, "1\n00:00:01,000 --> 00:00:02,000\nEpisode two\n").unwrap();

    let out_dir = dir.path().join("batch_out");
    let merged_path = dir.path().join("merged").join("series.apkg");
    let manifest_path = dir.path().join("manifest.json");
    let manifest = serde_json::json!({
        "output_dir": out_dir.to_str().unwrap(),
        "merge_output": merged_path.to_str().unwrap(),
        "defaults": {
            "format": "apkg",
            "deck": "Series",
            "no_audio": true,
            "no_snapshots": true,
            "no_video": true,
            "no_optimize": true,
            "jobs": 2
        },
        "episodes": [
            { "target": ep1.to_str().unwrap(), "deck": "Ep1", "episode": 1 },
            { "target": ep2.to_str().unwrap(), "deck": "Ep2", "episode": 2 }
        ]
    });
    std::fs::write(&manifest_path, manifest.to_string()).unwrap();

    let result = run(&[
        "batch",
        "--manifest",
        manifest_path.to_str().unwrap(),
        "--quiet",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
    assert!(out_dir.join("Ep1.apkg").is_file());
    assert!(out_dir.join("Ep2.apkg").is_file());
    assert!(merged_path.is_file());
    assert!(std::fs::read(&merged_path).unwrap().starts_with(b"PK"));
    assert!(String::from_utf8_lossy(&result.stdout).contains("Merged 2 APKG decks"));

    // Shared default deck name without per-episode deck override must isolate files and still merge.
    let shared_out = dir.path().join("shared_out");
    let shared_merged = shared_out.join("SharedSeries.apkg");
    let shared_manifest = dir.path().join("shared.json");
    std::fs::write(
        &shared_manifest,
        serde_json::json!({
            "output_dir": shared_out.to_str().unwrap(),
            "merge_output": shared_merged.to_str().unwrap(),
            "defaults": {
                "format": "apkg",
                "deck": "SharedSeries",
                "no_audio": true,
                "no_snapshots": true,
                "no_video": true
            },
            "episodes": [
                { "target": ep1.to_str().unwrap() },
                { "target": ep2.to_str().unwrap() }
            ]
        })
        .to_string(),
    )
    .unwrap();
    let shared_run = run(&[
        "batch",
        "--manifest",
        shared_manifest.to_str().unwrap(),
        "--quiet",
    ]);
    assert!(
        shared_run.status.success(),
        "{}",
        String::from_utf8_lossy(&shared_run.stderr)
    );
    assert!(shared_merged.is_file());
    assert!(std::fs::read(&shared_merged).unwrap().starts_with(b"PK"));

    // Single episode with merge_output copies cleanly.
    let single_out = dir.path().join("single_out");
    let single_merged = dir.path().join("single_merged.apkg");
    let single_manifest = dir.path().join("single.json");
    std::fs::write(
        &single_manifest,
        serde_json::json!({
            "output_dir": single_out.to_str().unwrap(),
            "merge_output": single_merged.to_str().unwrap(),
            "defaults": {
                "format": "apkg",
                "deck": "Solo",
                "no_audio": true,
                "no_snapshots": true,
                "no_video": true
            },
            "episodes": [
                { "target": ep1.to_str().unwrap() }
            ]
        })
        .to_string(),
    )
    .unwrap();
    let single_run = run(&[
        "batch",
        "--manifest",
        single_manifest.to_str().unwrap(),
        "--quiet",
    ]);
    assert!(single_run.status.success());
    assert!(single_merged.is_file());
    assert!(std::fs::read(&single_merged).unwrap().starts_with(b"PK"));
}

#[test]
fn batch_rejects_invalid_manifests() {
    let dir = tempfile::tempdir().unwrap();
    let ep1 = dir.path().join("ep1.srt");
    std::fs::write(&ep1, "1\n00:00:01,000 --> 00:00:02,000\nLine\n").unwrap();

    for (name, body, expected) in [
        (
            "empty_episodes.json",
            serde_json::json!({ "output_dir": "out", "episodes": [] }),
            "at least one episode",
        ),
        (
            "empty_output.json",
            serde_json::json!({ "output_dir": "  ", "episodes": [{ "target": ep1.to_str().unwrap() }] }),
            "output_dir must not be empty",
        ),
        (
            "merge_with_tsv.json",
            serde_json::json!({
                "output_dir": "out",
                "merge_output": "merged.apkg",
                "defaults": { "format": "tsv" },
                "episodes": [{ "target": ep1.to_str().unwrap() }]
            }),
            "requires defaults.format to be 'apkg'",
        ),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, body.to_string()).unwrap();
        let result = run(&["batch", "--manifest", path.to_str().unwrap()]);
        assert_eq!(result.status.code(), Some(1), "case {name}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains(expected), "case {name}: {stderr}");
    }
}
