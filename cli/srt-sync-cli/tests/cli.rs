use srt_parser::SrtParser;
use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_srt-sync"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_explains_repeated_anchor_syntax_and_ordering() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--anchor SUBTITLE_ID:CORRECTED_MS"));
    assert!(help.contains("must increase with source time"));
}

#[test]
fn retimes_with_the_shared_engine_and_keeps_stdout_clean() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let output = dir.path().join("output.srt");
    std::fs::write(
        &input,
        "1\n00:00:01,000 --> 00:00:02,000\n日本語\n\n2\n00:00:03,000 --> 00:00:04,000\nNext\n",
    )
    .unwrap();
    let result = run(&[
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--anchor",
        "1:1500",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    let subs = SrtParser::parse_file(output).unwrap();
    assert_eq!(subs[&1].start.total_milliseconds(), 1500);
    assert_eq!(subs[&2].end.total_milliseconds(), 4500);
    assert_eq!(subs[&1].text, "日本語");
}

#[test]
fn invalid_and_missing_anchors_fail_before_writing() {
    for anchor in ["bad", "0:1000", "1:-5", "1:9223372036854775808"] {
        let output = run(&[
            "--input",
            "missing.srt",
            "--output",
            "unused.srt",
            "--anchor",
            anchor,
        ]);
        assert_eq!(output.status.code(), Some(2), "anchor {anchor}");
        assert!(output.stdout.is_empty());
    }
    assert_eq!(
        run(&["--input", "missing.srt", "--output", "unused.srt"])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn invalid_subtitle_and_duplicate_anchors_preserve_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let output = dir.path().join("output.srt");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    std::fs::write(&output, "existing").unwrap();
    for anchors in [
        vec!["--anchor", "99:1000"],
        vec!["--anchor", "1:1000", "--anchor", "1:2000"],
    ] {
        let mut args = vec![
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ];
        args.extend(anchors);
        let result = run(&args);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "existing");
    }
}

#[test]
fn two_anchors_correct_drift_and_reject_reversed_times() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let output = dir.path().join("output.srt");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nFirst\n\n2\n00:00:03,000 --> 00:00:04,000\nSecond\n\n3\n00:00:05,000 --> 00:00:06,000\nThird\n").unwrap();
    let result = run(&[
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--anchor",
        "1:2000",
        "--anchor",
        "3:10000",
    ]);
    assert!(result.status.success());
    let synced = SrtParser::parse_file(&output).unwrap();
    assert_eq!(synced[&2].start.milliseconds, 6000);
    let original_output = std::fs::read(&output).unwrap();
    let result = run(&[
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--anchor",
        "1:10000",
        "--anchor",
        "3:2000",
    ]);
    assert!(!result.status.success());
    assert_eq!(std::fs::read(output).unwrap(), original_output);
}

#[test]
fn unrepresentable_result_preserves_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let output = dir.path().join("output.srt");
    std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    std::fs::write(&output, "existing").unwrap();
    let result = run(&[
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--anchor",
        "1:9223372036854775807",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("SRT"));
    assert_eq!(std::fs::read_to_string(output).unwrap(), "existing");
}
