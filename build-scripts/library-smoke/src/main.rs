use std::collections::HashMap;

use srt_parser::{SrtParser, Subtitle, Timestamp};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let source =
        "1\n00:00:01,000 --> 00:00:02,000\nHello\n\n2\n00:00:03,000 --> 00:00:04,000\nWorld\n";
    let mut subtitles = SrtParser::parse_string(source)?;
    assert_eq!(subtitles.len(), 2);
    assert_eq!(srt_extract::calculate_stats(&subtitles).total_count, 2);
    let exported = srt_extract::extract_to_json(&subtitles)?;
    assert!(exported.contains("Hello"));

    let temp = tempfile::tempdir()?;
    let saved = temp.path().join("roundtrip.srt");
    SrtParser::save_file(&saved, &subtitles)?;
    subtitles = SrtParser::parse_file(&saved)?;
    assert_eq!(subtitles.len(), 2);

    let source_dir = temp.path().join("apkg-source");
    std::fs::create_dir(&source_dir)?;
    std::fs::write(source_dir.join("media"), "{}")?;
    let package = temp.path().join("roundtrip.apkg");
    srt_apkg::zip_from_dir(&source_dir, &package).map_err(anyhow::Error::msg)?;
    let unpacked = temp.path().join("apkg-unpacked");
    srt_apkg::unzip_to(&package, &unpacked).map_err(anyhow::Error::msg)?;
    assert_eq!(std::fs::read_to_string(unpacked.join("media"))?, "{}");

    let cached = temp.path().join("cached-resource");
    std::fs::write(&cached, "already present")?;
    let downloaded =
        srt_download::download_to("http://127.0.0.1:1/unused", &cached, |_| {}, None).await?;
    assert_eq!(downloaded, cached);

    let flashcard_srt = temp.path().join("flashcards.srt");
    std::fs::write(&flashcard_srt, source)?;
    let flashcard_config = srt_flashcards::FlashcardConfig {
        target_subs_path: flashcard_srt.to_string_lossy().into_owned(),
        ..Default::default()
    };
    assert_eq!(
        srt_flashcards::preview(&flashcard_config)
            .map_err(anyhow::Error::msg)?
            .len(),
        2
    );

    let mut sync = srt_sync::SyncEngine::new(&saved)?;
    sync.add_anchor(1, 1500, true)?;
    let synced = sync.get_synced_subtitle(1).expect("subtitle 1");
    assert_eq!(synced.start.milliseconds, 1500);

    let pool = srt_translate::build_pool(&[vec![srt_translate::TierEntry {
        provider: "local".into(),
        api_key: None,
        api_url: Some("http://127.0.0.1:11434/v1".into()),
        model: "test".into(),
        max_requests: None,
        rpm: None,
    }]])
    .map_err(anyhow::Error::msg)?;
    assert_eq!(pool.len(), 1);
    assert_eq!(
        srt_translate::provider_defaults("local").1,
        "http://localhost:11434/v1"
    );
    assert!(
        !srt_transcribe::gpu_supported(),
        "default smoke build should be CPU-only"
    );
    let cfg: srt_transcribe::pipeline::TranscriptionConfig =
        serde_json::from_value(serde_json::json!({
            "input_path":"input.wav", "output_path":"output.srt", "model":"base",
            "language":"auto", "translate_to_english":false, "word_timestamps":false,
            "max_segment_length":80
        }))?;
    assert!(srt_transcribe::pipeline::is_cloud_provider(cfg.provider.as_deref()) == false);

    let mapper = srt_sync::TimeMapper::new();
    let mut sampler = srt_sync::AdaptiveSampler::new(2, srt_sync::SamplerStrategy::Sequential);
    assert_eq!(sampler.suggest_next(&mapper), Some(1));
    sampler.mark_checked(1);
    assert_eq!(sampler.suggest_next(&mapper), Some(2));

    let card = srt_refine::RefineCard {
        id: "1".into(),
        expression: "<b>Hello</b>".into(),
        meaning: "Ciao".into(),
        notes: String::new(),
    };
    assert_eq!(
        srt_refine::interpolate_prompt("{{expression}}={{meaning}}", &card),
        "Hello=Ciao"
    );
    assert_eq!(srt_refine::strip_html("<i>x</i>"), "x");

    let _: HashMap<u32, Subtitle> = subtitles;
    let _: Timestamp = Timestamp { milliseconds: 0 };
    let _: srt_autosync::AutoSyncOutcome = srt_autosync::AutoSyncOutcome {
        suggestions: vec![],
        segments_analyzed: 0,
        cancelled: false,
    };
    let _: srt_ankiconnect::AnkiNote = srt_ankiconnect::AnkiNote {
        deck_name: "Smoke".into(),
        model_name: "Basic".into(),
        fields: HashMap::new(),
        tags: vec![],
    };
    Ok(())
}
