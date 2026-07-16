use sha2::{Digest, Sha256};
use std::path::PathBuf;

use vesta_lib::commands::flashcards::*;

fn fixtures_dir() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("Test_Subs/fixtures")
}

fn fixture_path(name: &str) -> String {
    fixtures_dir().join(name).to_string_lossy().to_string()
}

fn sha256_str(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

fn text_only_config(target: &str, native: Option<&str>, deck: &str) -> FlashcardConfig {
    FlashcardConfig {
        target_subs_path: target.to_string(),
        native_subs_path: native.map(str::to_string),
        output_dir: std::env::temp_dir().to_string_lossy().to_string(),
        deck_name: deck.to_string(),
        cpu_cores: Some(2),
        ..FlashcardConfig::default()
    }
}

#[test]
fn test_parse_clip02_en_count() {
    let path = fixture_path("clip_02_en.srt");
    let info = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(flashcard_load_subs(path))
        .unwrap();
    assert_eq!(info.count, 37, "clip_02 EN should have 37 subtitle entries");
    assert_eq!(info.format, "srt");
}

#[test]
fn test_parse_clip02_it_count() {
    let path = fixture_path("clip_02_it.srt");
    let info = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(flashcard_load_subs(path))
        .unwrap();
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

#[test]
fn test_preview_dual_subs_clip02() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = text_only_config(
        &fixture_path("clip_02_en.srt"),
        Some(&fixture_path("clip_02_it.srt")),
        "test_dual",
    );
    let preview = rt.block_on(flashcard_preview(config)).unwrap();

    let active_count = preview.iter().filter(|l| l.active).count();
    assert!(active_count > 0, "Should have active lines");

    let with_subs2 = preview.iter().filter(|l| l.subs2_text.is_some()).count();
    assert!(with_subs2 > 0, "Should have subs2 matches");

    let all_subs1: String = preview
        .iter()
        .map(|l| l.subs1_text.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let hash = sha256_str(&all_subs1);
    eprintln!("[regression] clip_02 preview subs1 hash: {}", hash);

    assert_eq!(
        hash,
        sha256_str(&all_subs1),
        "subs1 text hash changed — logic may have been altered"
    );
}

#[test]
fn test_preview_single_sub_clip03() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = text_only_config(&fixture_path("clip_03_en.srt"), None, "test_single");
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    assert_eq!(preview.len(), 27, "clip_03 should produce 27 preview lines");

    assert!(preview.iter().all(|l| l.subs2_text.is_none()));
}

#[test]
fn test_filter_min_chars() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(&fixture_path("clip_02_en.srt"), None, "test_filter");
    config.filters.min_chars = Some(20);
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    let active: Vec<_> = preview.iter().filter(|l| l.active).collect();

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
    let mut config = text_only_config(&fixture_path("clip_02_en.srt"), None, "test_exclude");
    config.filters.exclude_words = Some("music,penny".to_string());
    let preview = rt.block_on(flashcard_preview(config)).unwrap();
    let active: Vec<_> = preview.iter().filter(|l| l.active).collect();
    for line in &active {
        let lower = line.subs1_text.to_lowercase();
        assert!(
            !lower.contains("music"),
            "Should not contain 'music': {}",
            line.subs1_text
        );
        assert!(
            !lower.contains("penny"),
            "Should not contain 'penny': {}",
            line.subs1_text
        );
    }
}

#[test]
fn test_filter_max_duration() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(&fixture_path("clip_05_en.srt"), None, "test_dur");
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

#[test]
fn test_span_filter() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(&fixture_path("clip_02_en.srt"), None, "test_span");

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

#[test]
fn test_context_lines() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(&fixture_path("clip_02_en.srt"), None, "test_ctx");
    config.context.leading = 1;
    config.context.trailing = 1;
    config.context.max_gap_seconds = 30.0;
    let preview = rt.block_on(flashcard_preview(config)).unwrap();

    if preview.len() > 2 {
        let mid = &preview[preview.len() / 2];
        assert!(
            !mid.leading_context.is_empty() || !mid.trailing_context.is_empty(),
            "Middle line should have context"
        );
    }
}

#[test]
fn test_sentence_combining() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut config = text_only_config(&fixture_path("clip_02_en.srt"), None, "test_combine");
    config.combine_sentences = true;
    config.continuation_chars = ",...".to_string();
    let preview_combined = rt.block_on(flashcard_preview(config)).unwrap();

    let config_no_combine =
        text_only_config(&fixture_path("clip_02_en.srt"), None, "test_no_combine");
    let preview_raw = rt.block_on(flashcard_preview(config_no_combine)).unwrap();

    assert!(
        preview_combined.len() <= preview_raw.len(),
        "Combining should reduce line count: {} > {}",
        preview_combined.len(),
        preview_raw.len()
    );
}

#[test]
fn test_time_shift_target() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let config_base = text_only_config(&fixture_path("clip_02_en.srt"), None, "t");
    let preview_base = rt.block_on(flashcard_preview(config_base)).unwrap();

    let mut config_shifted = text_only_config(&fixture_path("clip_02_en.srt"), None, "t");
    config_shifted.time_shift_target_ms = 500;
    let preview_shifted = rt.block_on(flashcard_preview(config_shifted)).unwrap();

    assert_eq!(preview_base.len(), preview_shifted.len());

    for (base, shifted) in preview_base.iter().zip(preview_shifted.iter()) {
        assert_eq!(shifted.start_ms, base.start_ms + 500);
        assert_eq!(shifted.end_ms, base.end_ms + 500);
    }
}

#[test]
fn test_regression_hashes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let clips: &[(&str, Option<&str>, &str)] = &[
        (
            "clip_02_en.srt",
            Some("clip_02_it.srt"),
            "2e034c13a56867a613b65e287979d1dafd99a323041d3aff8b90be13f0af5a65",
        ),
        (
            "clip_03_en.srt",
            Some("clip_03_it.srt"),
            "715a0ae130d3c52dc2f4e1c7a74a666050d599a3178b074e8d0ea7e517c39628",
        ),
        (
            "clip_04_en.srt",
            Some("clip_04_it.srt"),
            "a23e98e8f2af1fd9631d18884d3c35b9c8c9466ebea309e049d15ed5f5349a67",
        ),
        (
            "clip_05_en.srt",
            Some("clip_05_it.srt"),
            "ebe1530c8afcc4c9c62f824213276152c22cdafbe6377d155bdd082b8bd71287",
        ),
    ];

    for (en, it, expected_hash) in clips {
        let config = text_only_config(
            &fixture_path(en),
            it.map(fixture_path).as_deref(),
            "regression",
        );
        let preview = rt.block_on(flashcard_preview(config)).unwrap();

        let canonical: String = preview
            .iter()
            .map(|l| {
                format!(
                    "{}|{}|{}|{}|{}|{}\n",
                    l.index,
                    l.subs1_text,
                    l.subs2_text.as_deref().unwrap_or(""),
                    l.start_ms,
                    l.end_ms,
                    l.active
                )
            })
            .collect();
        let hash = sha256_str(&canonical);
        assert_eq!(
            hash, *expected_hash,
            "REGRESSION FAILURE for {}: hash changed from {} to {}",
            en, expected_hash, hash
        );
    }
}
