//! Golden-file integration tests based on the committed Detour (1945) fixtures.
//!
//! Fixture layout (all committed, small):
//!   Test_Subs/FILM/Detour-en.srt / Detour-it.srt          full-film subtitles
//!   Test_Subs/fixtures/detour/detour_clip_90s.mp4         90s clip, 320x240 (~1.3MB)
//!   Test_Subs/fixtures/detour/detour_clip_90s_{en,it}.srt subtitles for the clip window
//!   Test_Subs/fixtures/detour/golden_detour_film.tsv      reference TSV (generated via
//!                                                         `srt-flashcards generate -f tsv`)
//!
//! The golden TSV was produced by the CLI itself and the pipeline is fully
//! deterministic for SRT-only runs, so any diff means the parse/match/filter/
//! export behaviour changed — the assertion messages point at the exact line.

use srt_flashcards::{
    ContextConfig, FieldNamesConfig, FlashcardConfig, MediaTools, OutputFields, SubtitleFilters,
    generate, preview,
};
use tokio_util::sync::CancellationToken;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn film(name: &str) -> String {
    repo_root()
        .join("Test_Subs")
        .join("FILM")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn fixture(name: &str) -> String {
    repo_root()
        .join("Test_Subs")
        .join("fixtures")
        .join("detour")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Mirror of `srt-flashcards-cli generate` defaults (see GenerateArgs::to_config):
/// the golden TSV was generated with the CLI, so tests must build the same config.
fn cli_like_config(
    target: String,
    native: Option<String>,
    video: Option<String>,
    output_dir: String,
    deck: &str,
    format: &str,
    media: bool,
) -> FlashcardConfig {
    FlashcardConfig {
        target_subs_path: target,
        native_subs_path: native.clone(),
        video_path: video,
        audio_path: None,
        output_dir,
        filters: SubtitleFilters::default(),
        context: ContextConfig::default(),
        generate_audio: media,
        generate_snapshots: media,
        generate_video_clips: false,
        deck_name: deck.to_string(),
        episode_number: 1,
        export_format: Some(format.to_string()),
        field_names: Some(FieldNamesConfig::default()),
        output_fields: OutputFields {
            include_audio: media,
            include_snapshot: media,
            include_video: false,
            include_subs2: native.is_some(),
            ..OutputFields::default()
        },
        cpu_cores: Some(2),
        ..FlashcardConfig::default()
    }
}

async fn run_generate(config: FlashcardConfig) -> srt_flashcards::FlashcardResult {
    let tools = MediaTools::new("ffmpeg", "ffprobe");
    let progress = |_ev| {};
    generate(config, tools, CancellationToken::new(), &progress)
        .await
        .expect("generate() returned Err — the flashcard pipeline itself failed")
}

// ─── 1. Full-film golden TSV ────────────────────────────────────────────────

#[tokio::test]
async fn film_tsv_matches_golden() {
    let out = tempfile::tempdir().expect("create tempdir");
    let config = cli_like_config(
        film("Detour-en.srt"),
        Some(film("Detour-it.srt")),
        None,
        out.path().to_string_lossy().into_owned(),
        "DetourGolden",
        "tsv",
        false,
    );

    let result = run_generate(config).await;
    assert!(
        result.success,
        "SRT-only TSV generation on Detour film failed: {:?}",
        result.message
    );
    assert_eq!(
        result.cards_generated, 1018,
        "Detour film should produce 1018 cards (one per subtitle line): the \
         parse or match stage changed behaviour"
    );

    let tsv_path = out.path().join("DetourGolden.tsv");
    let generated = std::fs::read_to_string(&tsv_path)
        .unwrap_or_else(|e| panic!("generated TSV missing at {}: {e}", tsv_path.display()));
    let golden = std::fs::read_to_string(fixture("golden_detour_film.tsv"))
        .expect("golden_detour_film.tsv fixture missing — regenerate it with the CLI");

    let gen_lines: Vec<&str> = generated.lines().collect();
    let gold_lines: Vec<&str> = golden.lines().collect();
    assert_eq!(
        gen_lines.len(),
        gold_lines.len(),
        "TSV row count changed: golden has {} rows, this build produced {}",
        gold_lines.len(),
        gen_lines.len()
    );
    for (i, (got, want)) in gen_lines.iter().zip(gold_lines.iter()).enumerate() {
        assert_eq!(
            got,
            want,
            "TSV row {} differs from golden.\n  expected: {}\n  got:      {}\n\
             If the change is intentional, regenerate the golden with:\n  \
             srt-flashcards generate -t Test_Subs/FILM/Detour-en.srt \
             -n Test_Subs/FILM/Detour-it.srt -o /tmp/g -f tsv -d DetourGolden \
             --no-audio --no-snapshots --no-video && \
             cp /tmp/g/DetourGolden.tsv Test_Subs/fixtures/detour/golden_detour_film.tsv",
            i + 1,
            want,
            got
        );
    }
}

// ─── 2. Clip pipeline: line counts stay in lockstep ─────────────────────────

#[test]
fn clip_preview_counts() {
    let config = cli_like_config(
        fixture("detour_clip_90s_en.srt"),
        Some(fixture("detour_clip_90s_it.srt")),
        None,
        String::new(),
        "DetourClip",
        "tsv",
        false,
    );
    let lines = preview(&config).expect("preview on the 90s clip fixtures failed");
    assert_eq!(
        lines.len(),
        26,
        "the 90s Detour clip window contains 26 subtitle lines; parser or span \
         handling changed (got {})",
        lines.len()
    );
    let matched = lines.iter().filter(|l| l.subs2_text.is_some()).count();
    assert!(
        matched >= 20,
        "at least 20 of 26 clip lines should match an Italian native line by \
         time-overlap, got {matched}: the subtitle matcher regressed"
    );
}

// ─── 3. Full media pipeline + APKG structure on the committed clip ──────────

#[tokio::test]
async fn clip_apkg_full_media_pipeline() {
    if !ffmpeg_available() {
        eprintln!("[skip] clip_apkg_full_media_pipeline: ffmpeg not found in PATH");
        return;
    }

    let out = tempfile::tempdir().expect("create tempdir");
    let config = cli_like_config(
        fixture("detour_clip_90s_en.srt"),
        Some(fixture("detour_clip_90s_it.srt")),
        Some(fixture("detour_clip_90s.mp4")),
        out.path().to_string_lossy().into_owned(),
        "DetourClip",
        "apkg",
        true,
    );

    let result = run_generate(config).await;
    assert!(
        result.success,
        "APKG generation with media on the 90s clip failed: {:?}",
        result.message
    );
    assert_eq!(
        result.cards_generated, 26,
        "expected 26 cards from the clip"
    );
    assert_eq!(
        result.audio_clips, 26,
        "every card should get an audio clip (ffmpeg extraction failed for some lines)"
    );
    assert_eq!(
        result.snapshots, 26,
        "every card should get a snapshot (ffmpeg frame extraction failed for some lines)"
    );

    // ── Inspect the produced .apkg ──
    let apkg_path = out.path().join("DetourClip.apkg");
    let file = std::fs::File::open(&apkg_path)
        .unwrap_or_else(|e| panic!("missing .apkg at {}: {e}", apkg_path.display()));
    let mut zip = zip::ZipArchive::new(file).expect("apkg is not a valid zip archive");

    // Media manifest: JSON map { "0": "filename", ... } — one entry per clip+snapshot.
    let media_manifest: serde_json::Value = {
        let entry = zip
            .by_name("media")
            .expect("apkg has no `media` manifest entry — Anki cannot import it");
        serde_json::from_reader(entry).expect("`media` manifest is not valid JSON")
    };
    let media_count = media_manifest
        .as_object()
        .expect("`media` manifest must be a JSON object")
        .len();
    assert_eq!(
        media_count, 52,
        "expected 52 media files in the apkg (26 audio + 26 snapshots), found {media_count}"
    );

    // Database: notes/cards/deck must be consistent.
    let tmp = tempfile::tempdir().expect("create tempdir for db");
    let db_path = tmp.path().join("collection.anki2");
    {
        let mut entry = zip
            .by_name("collection.anki2")
            .expect("apkg has no collection.anki2 database");
        let mut outf = std::fs::File::create(&db_path).expect("create temp db file");
        std::io::copy(&mut entry, &mut outf).expect("extract collection.anki2");
    }
    let conn = rusqlite::Connection::open(&db_path).expect("open collection.anki2");

    let notes: i64 = conn
        .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
        .expect("query notes count");
    assert_eq!(
        notes, 26,
        "collection.anki2 should contain 26 notes, found {notes}"
    );

    let cards: i64 = conn
        .query_row("SELECT COUNT(*) FROM cards", [], |r| r.get(0))
        .expect("query cards count");
    assert_eq!(
        cards, 26,
        "collection.anki2 should contain 26 cards, found {cards}"
    );

    let decks_json: String = conn
        .query_row("SELECT decks FROM col", [], |r| r.get(0))
        .expect("read decks JSON from col table");
    assert!(
        decks_json.contains("DetourClip"),
        "deck name 'DetourClip' not found in the collection's deck registry"
    );

    // Every note must reference its media: [sound:...] on audio, <img on snapshot.
    let mut with_sound = 0;
    let mut with_img = 0;
    let mut stmt = conn
        .prepare("SELECT flds FROM notes")
        .expect("prepare notes query");
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .expect("iterate notes");
    for flds in rows {
        let flds = flds.expect("read note fields");
        if flds.contains("[sound:") {
            with_sound += 1;
        }
        if flds.contains("<img") {
            with_img += 1;
        }
    }
    assert_eq!(
        with_sound, 26,
        "every note should embed a [sound:...] tag, only {with_sound}/26 do"
    );
    assert_eq!(
        with_img, 26,
        "every note should embed an <img> snapshot, only {with_img}/26 do"
    );
}
