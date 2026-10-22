use std::process::Command;

#[test]
fn help_describes_both_file_modes_and_embedded_modes_reject_unused_format() {
    let help = Command::new(env!("CARGO_BIN_EXE_srt-extract"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help_text.contains("SRT file to read, or media container"));
    assert!(help_text.contains("--track requires --output"));
    assert!(help_text.contains("ffprobe"));
    assert!(help_text.contains("Output must be a different file"));

    let invalid = Command::new(env!("CARGO_BIN_EXE_srt-extract"))
        .args(["--input", "film.mkv", "--list-tracks", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("cannot be used with"));
}

#[test]
fn json_stdout_is_machine_readable() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\n日本語\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_srt-extract"))
        .args(["--input", input.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["1"]["text"], "日本語");
}

#[test]
fn invalid_format_is_rejected_before_opening_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_srt-extract"))
        .args(["--input", "missing.srt", "--format", "typo"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("typo"));
}

#[test]
fn missing_input_does_not_overwrite_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("missing.srt");
    let destination = dir.path().join("output.json");
    std::fs::write(&destination, "existing").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_srt-extract"))
        .args([
            "--input",
            input.to_str().unwrap(),
            "--output",
            destination.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing.srt"));
    assert_eq!(std::fs::read_to_string(destination).unwrap(), "existing");
}

#[test]
fn output_cannot_replace_the_input_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let original = "1\n00:00:01,000 --> 00:00:02,000\nHello\n";
    std::fs::write(&input, original).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_srt-extract"))
        .args([
            "--input",
            input.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            input.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("same file"));
    assert_eq!(std::fs::read_to_string(input).unwrap(), original);
}
