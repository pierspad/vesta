use srt_flashcards::FlashcardConfig;
use vesta_mcp::{Jobs, Paths, Status};

#[tokio::test]
async fn cancellation_drains_work_and_shutdown_prevents_more_jobs() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let source = input.path().join("film.srt");
    std::fs::write(&source, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
    let paths = Paths::new(vec![input.path().to_owned()], output.path().to_owned()).unwrap();
    let config = FlashcardConfig {
        target_subs_path: source.to_str().unwrap().into(),
        deck_name: "Cancel".into(),
        auto_card_font: false,
        embed_card_font: false,
        ..FlashcardConfig::default()
    };
    let jobs = Jobs::default();
    let job = jobs.start(config.clone(), &paths).unwrap();
    assert!(jobs.start(config.clone(), &paths).is_err());
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 1);
    assert!(matches!(
        jobs.cancel(&job.job_id).unwrap().status,
        Status::Cancelling
    ));
    jobs.shutdown().await;
    assert!(matches!(
        jobs.get(&job.job_id).unwrap().status,
        Status::Cancelled
    ));
    assert!(jobs.start(config, &paths).is_err());
    assert!(
        !std::path::Path::new(&job.output_dir)
            .join("Cancel.tsv")
            .exists()
    );
}

#[test]
fn input_roots_reject_traversal_and_non_subtitle_files() {
    let input = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let paths = Paths::new(vec![input.path().to_owned()], input.path().to_owned()).unwrap();
    let secret = outside.path().join("secret.srt");
    std::fs::write(&secret, "private").unwrap();
    assert!(paths.input(&secret, true).is_err());
    let config = input.path().join("keys.json");
    std::fs::write(&config, "private").unwrap();
    assert!(paths.input(&config, true).is_err());
    assert!(Paths::new(vec![], input.path().to_owned()).is_err());
}

#[cfg(unix)]
#[test]
fn canonical_paths_reject_symlinks_that_escape_read_roots() {
    let input = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("secret.srt");
    std::fs::write(&secret, "private").unwrap();
    let link = input.path().join("film.srt");
    std::os::unix::fs::symlink(secret, &link).unwrap();
    let paths = Paths::new(vec![input.path().to_owned()], input.path().to_owned()).unwrap();
    assert!(paths.input(&link, true).is_err());
}
