use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vesta-cli"))
        .args(args)
        .output()
        .expect("vesta-cli should start")
}

#[test]
fn top_level_help_lists_operations_and_no_arguments_show_help() {
    for output in [run(&["--help"]), run(&[])] {
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Vesta headless subtitle and flashcard workflows"));
        for operation in [
            "flashcards",
            "translate",
            "transcribe",
            "sync",
            "autosync",
            "refine",
            "extract",
        ] {
            assert!(
                stdout.contains(operation),
                "missing {operation} in {stdout}"
            );
        }
    }
}

#[test]
fn forwards_full_flashcard_help_and_runs_a_no_media_preview() {
    let help = run(&["flashcards", "--help"]);
    assert!(help.status.success());
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help_text.contains("Turn subtitles + media into Anki flashcards"));
    assert!(help_text.contains("preview"));
    assert!(help_text.contains("generate"));
    let generate_help = run(&["flashcards", "generate", "--help"]);
    assert!(generate_help.status.success());
    assert!(String::from_utf8_lossy(&generate_help.stdout).contains("--target"));

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("film.srt");
    let output = dir.path().join("unused-output");
    std::fs::write(
        &input,
        "1\n00:00:01,000 --> 00:00:02,000\nHello there\n\n2\n00:00:03,000 --> 00:00:04,500\nSecond line\n",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_vesta-cli"))
        .args([
            "flashcards",
            "preview",
            "--target",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--no-audio",
            "--no-snapshots",
            "--no-video",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "Total lines:   2\nActive lines:  2\nWith native:   0\n"
    );
    assert!(result.stderr.is_empty());
    assert!(!output.exists(), "preview must not create output files");
}

#[test]
fn unknown_operation_is_a_clap_usage_error() {
    let output = run(&["not-a-command"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized operation"));
}

#[test]
fn every_operation_exposes_help() {
    for operation in [
        "flashcards",
        "translate",
        "transcribe",
        "sync",
        "autosync",
        "refine",
        "extract",
    ] {
        let result = run(&[operation, "--help"]);
        assert!(
            result.status.success(),
            "{operation}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("Usage:"));
    }
}

#[test]
fn generates_a_deck_without_media_or_display() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let output = dir.path().join("deck");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello there\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_vesta-cli"))
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .args(["flashcards", "generate", "--target"])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .args([
            "--deck",
            "Test",
            "--no-audio",
            "--no-snapshots",
            "--no-video",
            "--quiet",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let tsv = std::fs::read_to_string(output.join("Test.tsv")).unwrap();
    assert!(tsv.contains("Hello there"));
}

#[test]
fn help_subcommand_forwards_nested_help() {
    for args in [&["help"][..], &["help", "flashcards", "generate"][..]] {
        let output = run(args);
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
}
