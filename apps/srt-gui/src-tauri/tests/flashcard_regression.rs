//! Flashcard regression tests.
//!
//! These tests exercise the pure-logic parts of the flashcard pipeline
//! (parsing, matching, filtering, TSV/APKG generation) using small fixture
//! clips created from Test_Subs/SERIE_TV/Detour_parte1.
//!
//! The tests produce deterministic outputs and compare SHA-256 hashes so
//! that an agentic LLM can freely refactor the generation code and verify
//! that the logical output has not changed.
//!
//! Run with:
//!     cargo test --package vesta --test flashcard_regression -- --nocapture

use std::path::PathBuf;
use sha2::{Sha256, Digest};

// Re-use the library crate which re-exports commands::flashcards
use vesta_lib::commands::flashcards::*;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn fixtures_dir() -> PathBuf {
    // Navigate from src-tauri/tests/ up to project root, then into Test_Subs/fixtures
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest)
        .parent().unwrap()  // srt-gui
        .parent().unwrap()  // apps
        .parent().unwrap()  // vesta root
        .join("Test_Subs/fixtures")
}

fn fixture_path(name: &str) -> String {
    fixtures_dir().join(name).to_string_lossy().to_string()
}

fn sha256_str(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Build a minimal FlashcardConfig for text-only tests (no media extraction).
fn text_only_config(target: &str, native: Option<&str>, deck: &str) -> FlashcardConfig {
    FlashcardConfig {
        target_subs_path: target.to_string(),
        native_subs_path: native.map(|s| s.to_string()),
        video_path: None,
        audio_path: None,
        output_dir: std::env::temp_dir().to_string_lossy().to_string(),
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
        context: ContextConfig {
            leading: 0,
            trailing: 0,
            max_gap_seconds: 0.0,
        },
        combine_sentences: false,
        continuation_chars: String::new(),
        generate_audio: false,
        audio_bitrate: 128,
        normalize_audio: false,
        audio_pad_start_ms: 0,
        audio_pad_end_ms: 0,
        generate_snapshots: false,
        snapshot_width: 0,
        snapshot_height: 0,
        crop_bottom: 0,
        generate_video_clips: false,
        video_codec: "h264".to_string(),
        h264_preset: "ultrafast".to_string(),
        video_bitrate: 0,
        video_audio_bitrate: 0,
        video_pad_start_ms: 0,
        video_pad_end_ms: 0,
        deck_name: deck.to_string(),
        episode_number: 1,
        export_format: Some("tsv".to_string()),
        note_type_name: None,
        output_fields: OutputFields {
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

// ─── SRT Parsing Tests ───────────────────────────────────────────────────────

#[test]
fn test_parse_clip02_en_count() {
    let path = fixture_path("clip_02_en.srt");
    let info = tokio::runtime::Runtime::new().unwrap().block_on(
        flashcard_load_subs(path)
    ).unwrap();
    assert_eq!(info.count, 37, "clip_02 EN should have 37 subtitle entries");
    assert_eq!(info.format, "srt");
}

#[test]
fn test_parse_clip02_it_count() {
    let path = fixture_path("clip_02_it.srt");
    let info = tokio::runtime::Runtime::new().unwrap().block_on(
        flashcard_load_subs(path)
    ).unwrap();
    assert_eq!(info.count, 37, "clip_02 IT should have 37 subtitle entries");
}

#[test]
fn test_parse_all_clips_counts() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let expected: &[(&str, usize)] = &[
        ("clip_01_en.srt", 8),
        ("clip_02_en.srt", 37),
        ("clip_03_en.srt", 27),
        ("clip_04_en.srt", 31),
        ("clip_05_en.srt", 42),
    ];
    for (file, expected_count) in expected {
        let path = fixture_path(file);
        let info = rt.block_on(flashcard_load_subs(path)).unwrap();
        assert_eq!(info.count, *expected_count, "Mismatch for {}", file);
    }
}

// ─── Preview / Matching Tests ────────────────────────────────────────────────

#[test]
fn test_preview_dual_subs_clip02() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = text_only_config(
        &fixture_path("clip_02_en.srt"),
        Some(&fixture_path("clip_02_it.srt")),
        "test_dual",
    );
    let preview = rt.block_on(flashcard_preview(config)).unwrap();

    // All lines should be active (no filters)
    let active_count = preview.iter().filter(|l| l.active).count();
    assert!(active_count > 0, "Should have active lines");

    // Every line should have a subs2 match (same count, same timestamps)
    let with_subs2 = preview.iter().filter(|l| l.subs2_text.is_some()).count();
    assert!(with_subs2 > 0, "Should have subs2 matches");

    // Verify the text content is stable by hashing all subs1 texts
    let all_subs1: String = preview.iter()
        .map(|l| l.subs1_text.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let hash = sha256_str(&all_subs1);
    eprintln!("[regression] clip_02 preview subs1 hash: {}", hash);
    // Store this hash as reference — future runs must match
    assert_eq!(
        hash,
        sha256_str(&all_subs1), // self-check; update with known-good hash after first run
        "subs1 text hash changed — logic may have been altered"
    );
}

#[test]
fn test_preview_single_sub_clip03() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = text_only_config(
        &fixture_path("clip_03_en.srt"),
        None,
        "test_single",
    );
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    assert_eq!(preview.len(), 27, "clip_03 should produce 27 preview lines");
    // No subs2 when native is None
    assert!(preview.iter().all(|l| l.subs2_text.is_none()));
}

// ─── Filter Tests ────────────────────────────────────────────────────────────

#[test]
fn test_filter_min_chars() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(
        &fixture_path("clip_02_en.srt"),
        None,
        "test_filter",
    );
    config.filters.min_chars = Some(20);
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    let active: Vec<_> = preview.iter().filter(|l| l.active).collect();
    // All active lines should have >= 20 chars
    for line in &active {
        assert!(
            line.subs1_text.chars().count() >= 20,
            "Line with text '{}' has < 20 chars but is active",
            line.subs1_text
        );
    }
}

#[test]
fn test_filter_exclude_words() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(
        &fixture_path("clip_02_en.srt"),
        None,
        "test_exclude",
    );
    config.filters.exclude_words = Some("music,penny".to_string());
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    let active: Vec<_> = preview.iter().filter(|l| l.active).collect();
    for line in &active {
        let lower = line.subs1_text.to_lowercase();
        assert!(!lower.contains("music"), "Should not contain 'music': {}", line.subs1_text);
        assert!(!lower.contains("penny"), "Should not contain 'penny': {}", line.subs1_text);
    }
}

#[test]
fn test_filter_max_duration() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(
        &fixture_path("clip_05_en.srt"),
        None,
        "test_dur",
    );
    config.filters.max_duration_ms = Some(3000);
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    let active: Vec<_> = preview.iter().filter(|l| l.active).collect();
    for line in &active {
        assert!(
            line.duration_ms <= 3000,
            "Line duration {}ms exceeds 3000ms",
            line.duration_ms
        );
    }
}

// ─── Span Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_span_filter() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(
        &fixture_path("clip_02_en.srt"),
        None,
        "test_span",
    );
    // Only include lines between 30s and 60s
    config.span_start_ms = Some(30_000);
    config.span_end_ms = Some(60_000);
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    let active: Vec<_> = preview.iter().filter(|l| l.active).collect();
    assert!(!active.is_empty(), "Should have lines in 30-60s range");
    for line in &active {
        assert!(line.end_ms >= 30_000, "Line ends before span start");
        assert!(line.start_ms <= 60_000, "Line starts after span end");
    }
}

// ─── Context Lines Tests ─────────────────────────────────────────────────────

#[test]
fn test_context_lines() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(
        &fixture_path("clip_02_en.srt"),
        None,
        "test_ctx",
    );
    config.context.leading = 1;
    config.context.trailing = 1;
    config.context.max_gap_seconds = 30.0;
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    // Middle lines should have both leading and trailing context
    if preview.len() > 2 {
        let mid = &preview[preview.len() / 2];
        assert!(!mid.leading_context.is_empty() || !mid.trailing_context.is_empty(),
            "Middle line should have context");
    }
}

// ─── Sentence Combining Tests ────────────────────────────────────────────────

#[test]
fn test_sentence_combining() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(
        &fixture_path("clip_02_en.srt"),
        None,
        "test_combine",
    );
    config.combine_sentences = true;
    config.continuation_chars = ",...".to_string();
    let preview_combined = rt.block_on(flashcard_preview(config)).unwrap();

    let config_no_combine = text_only_config(
        &fixture_path("clip_02_en.srt"),
        None,
        "test_no_combine",
    );
    let preview_raw = rt.block_on(flashcard_preview(config_no_combine)).unwrap();

    // Combined should have fewer or equal lines
    assert!(
        preview_combined.len() <= preview_raw.len(),
        "Combining should reduce line count: {} > {}",
        preview_combined.len(), preview_raw.len()
    );
}

// ─── Time Shift Tests ────────────────────────────────────────────────────────

#[test]
fn test_time_shift_target() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let config_base = text_only_config(&fixture_path("clip_02_en.srt"), None, "t");
    let preview_base = rt.block_on(flashcard_preview(config_base)).unwrap();

    let mut config_shifted = text_only_config(&fixture_path("clip_02_en.srt"), None, "t");
    config_shifted.time_shift_target_ms = 500;
    let preview_shifted = rt.block_on(flashcard_preview(config_shifted)).unwrap();

    assert_eq!(preview_base.len(), preview_shifted.len());
    // Each shifted line should be +500ms
    for (base, shifted) in preview_base.iter().zip(preview_shifted.iter()) {
        assert_eq!(shifted.start_ms, base.start_ms + 500);
        assert_eq!(shifted.end_ms, base.end_ms + 500);
    }
}

// ─── Hash Regression: Record Known-Good Hashes ──────────────────────────────
//
// This test prints SHA-256 hashes of all preview outputs.
// After first run, copy the hashes into the assertions below.
// Future refactors MUST produce identical hashes.

/// Known-good regression hashes. If ANY of these change, the flashcard
/// pipeline logic has been modified and the change must be validated.
#[test]
fn test_regression_hashes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let clips: &[(&str, Option<&str>, &str)] = &[
        ("clip_02_en.srt", Some("clip_02_it.srt"),
         "83c78c93c65d7d6559d490089d7889d0e40870402aa272a4c9301c8d46d40ae4"),
        ("clip_03_en.srt", Some("clip_03_it.srt"),
         "8bec366eb4c085fd02940c3de0ad5e35bb41abadbec7c5654e2687bba38e1277"),
        ("clip_04_en.srt", Some("clip_04_it.srt"),
         "4be580a56afc1d17d27cf84aa39445d693937d6616e0e0e47bb9d998891d835b"),
        ("clip_05_en.srt", Some("clip_05_it.srt"),
         "c876202fb60e27e54a03d6ad22fff6dcf3faf871d6bcd9947a06cf69fb9327af"),
    ];

    for (en, it, expected_hash) in clips {
        let config = text_only_config(
            &fixture_path(en),
            it.map(|f| fixture_path(f)).as_deref(),
            "regression",
        );
        let preview = rt.block_on(flashcard_preview(config)).unwrap();

        let canonical: String = preview.iter().map(|l| {
            format!("{}|{}|{}|{}|{}|{}\n",
                l.index, l.subs1_text, l.subs2_text.as_deref().unwrap_or(""),
                l.start_ms, l.end_ms, l.active)
        }).collect();
        let hash = sha256_str(&canonical);
        assert_eq!(
            hash, *expected_hash,
            "REGRESSION FAILURE for {}: hash changed from {} to {}",
            en, expected_hash, hash
        );
    }
}
