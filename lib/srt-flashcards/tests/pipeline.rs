use srt_flashcards::{FlashcardConfig, build_matched_lines, load_sub_file_info, preview};

#[test]
fn overflowing_time_shifts_are_errors_in_both_tracks() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("input.srt");
    std::fs::write(&path, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    let path = path.to_string_lossy().into_owned();
    let mut settings = FlashcardConfig {
        target_subs_path: path.clone(),
        native_subs_path: Some(path),
        time_shift_target_ms: i64::MAX,
        ..Default::default()
    };
    assert!(
        build_matched_lines(&settings)
            .unwrap_err()
            .contains("overflows")
    );
    settings.time_shift_target_ms = 0;
    settings.time_shift_native_ms = i64::MAX;
    assert!(
        build_matched_lines(&settings)
            .unwrap_err()
            .contains("overflows")
    );
    settings.time_shift_native_ms = -500;
    assert_eq!(
        build_matched_lines(&settings).unwrap()[0]
            .subs2
            .as_ref()
            .unwrap()
            .start_ms,
        500
    );
}

fn test_subs_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("Test_Subs")
        .join("FILM")
}

fn en() -> String {
    test_subs_dir()
        .join("Detour-en.srt")
        .to_string_lossy()
        .into_owned()
}

fn it() -> String {
    test_subs_dir()
        .join("Detour-it.srt")
        .to_string_lossy()
        .into_owned()
}

fn media_present() -> bool {
    std::path::Path::new(&en()).exists() && std::path::Path::new(&it()).exists()
}

fn config(target: String, native: Option<String>) -> FlashcardConfig {
    FlashcardConfig {
        target_subs_path: target,
        native_subs_path: native,
        deck_name: "Test".to_string(),
        cpu_cores: Some(2),
        ..FlashcardConfig::default()
    }
}

#[test]
fn parses_detour_english() {
    if !media_present() {
        eprintln!("[skip] Test_Subs/FILM/Detour-*.srt not present");
        return;
    }
    let info = load_sub_file_info(&en()).expect("parse en");
    assert_eq!(info.format, "srt");
    assert_eq!(info.count, 1018, "Detour-en.srt should have 1018 entries");
}

#[test]
fn dual_subs_match_one_to_one() {
    if !media_present() {
        return;
    }
    let lines = preview(&config(en(), Some(it()))).expect("preview dual");
    assert_eq!(lines.len(), 1018);
    let with_native = lines.iter().filter(|l| l.subs2_text.is_some()).count();
    assert!(
        with_native > 900,
        "most lines should match a native sub, got {with_native}"
    );
}

#[test]
fn single_sub_has_no_native() {
    if !media_present() {
        return;
    }
    let lines = preview(&config(en(), None)).expect("preview single");
    assert_eq!(lines.len(), 1018);
    assert!(lines.iter().all(|l| l.subs2_text.is_none()));
}

#[test]
fn span_filter_narrows_active_window() {
    if !media_present() {
        return;
    }
    let mut cfg = config(en(), Some(it()));
    cfg.span_start_ms = Some(90_000);
    cfg.span_end_ms = Some(130_000);
    let matched = build_matched_lines(&cfg).expect("matched");
    let active = matched.iter().filter(|m| m.active).count();
    assert!(
        active > 0 && active < 50,
        "span should isolate a handful of lines, got {active}"
    );
    for m in matched.iter().filter(|m| m.active) {
        assert!(m.subs1.end_ms >= 90_000 && m.subs1.start_ms <= 130_000);
    }
}

#[test]
fn min_chars_filter_drops_short_lines() {
    if !media_present() {
        return;
    }
    let mut cfg = config(en(), None);
    cfg.filters.min_chars = Some(40);
    let lines = preview(&cfg).expect("preview");
    for line in lines.iter().filter(|l| l.active) {
        assert!(line.subs1_text.chars().count() >= 40);
    }
}
