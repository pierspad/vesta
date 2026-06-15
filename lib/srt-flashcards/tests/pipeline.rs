//! Fast, GUI-free regression guard for the subtitle pipeline.
//!
//! Runs against the real public-domain *Detour (1945)* subtitles shipped in
//! `Test_Subs/`. Pure CPU work (parse → match → filter → context), so it needs
//! neither ffmpeg nor Tauri and finishes in milliseconds. If the test media is
//! absent (e.g. a slim checkout) the tests skip instead of failing.

use srt_flashcards::{
    build_matched_lines, load_sub_file_info, preview, ContextConfig, FlashcardConfig,
    SubtitleFilters,
};

fn test_subs_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("Test_Subs")
}

fn en() -> String {
    test_subs_dir().join("Detour-en.srt").to_string_lossy().into_owned()
}

fn it() -> String {
    test_subs_dir().join("Detour-it.srt").to_string_lossy().into_owned()
}

fn media_present() -> bool {
    std::path::Path::new(&en()).exists() && std::path::Path::new(&it()).exists()
}

/// Minimal text-only config (no media extraction) for pipeline tests.
fn config(target: String, native: Option<String>) -> FlashcardConfig {
    FlashcardConfig {
        target_subs_path: target,
        native_subs_path: native,
        video_path: None,
        audio_path: None,
        output_dir: String::new(),
        use_timings_from: "target".to_string(),
        span_start_ms: None,
        span_end_ms: None,
        time_shift_target_ms: 0,
        time_shift_native_ms: 0,
        filters: SubtitleFilters {
            include_words: None,
            exclude_words: None,
            exclude_duplicates_subs1: false,
            exclude_duplicates_subs2: false,
            min_chars: None,
            max_chars: None,
            min_duration_ms: None,
            max_duration_ms: None,
            exclude_styled: false,
            actor_filter: None,
            only_cjk: false,
            remove_no_match: false,
        },
        context: ContextConfig { leading: 0, trailing: 0, max_gap_seconds: 0.0 },
        combine_sentences: false,
        continuation_chars: String::new(),
        generate_audio: false,
        audio_bitrate: 128,
        audio_track_index: None,
        normalize_audio: false,
        audio_pad_start_ms: 0,
        audio_pad_end_ms: 0,
        generate_snapshots: false,
        snapshot_width: 240,
        snapshot_height: 160,
        crop_bottom: 0,
        generate_video_clips: false,
        video_codec: "h264".to_string(),
        h264_preset: "ultrafast".to_string(),
        video_bitrate: 1000,
        video_audio_bitrate: 128,
        video_pad_start_ms: 0,
        video_pad_end_ms: 0,
        deck_name: "Test".to_string(),
        episode_number: 1,
        export_format: Some("tsv".to_string()),
        note_type_name: None,
        field_names: None,
        output_fields: srt_flashcards::OutputFields {
            include_tag: true,
            include_sequence: true,
            include_audio: false,
            include_snapshot: false,
            include_video: false,
            include_subs1: true,
            include_subs2: true,
        },
        cpu_cores: Some(2),
        card_front_html: None,
        card_back_html: None,
        card_css: None,
    }
}

#[test]
fn parses_detour_english() {
    if !media_present() {
        eprintln!("[skip] Test_Subs not present");
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
    assert!(with_native > 900, "most lines should match a native sub, got {with_native}");
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
    assert!(active > 0 && active < 50, "span should isolate a handful of lines, got {active}");
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
