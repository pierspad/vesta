//! Own staging and publish complete exports using each format's rollback contract.
use super::*;

pub(super) struct PublishedExport {
    pub tsv_path: Option<String>,
    pub apkg_path: Option<String>,
    pub size_bytes: u64,
}

pub(super) fn publish_export(
    config: &FlashcardConfig,
    matched: &[MatchedLine],
    workspace: ExportWorkspace,
    cancel: &CancellationToken,
    progress: ProgressCallback<'_>,
) -> Result<Option<PublishedExport>, String> {
    let ExportWorkspace {
        directory: media_workspace,
        output: output_dir,
        media: media_dir,
    } = workspace;
    let export_format = config.export_format.as_deref().unwrap_or("tsv");
    let media_size_bytes = if export_format == "apkg" {
        0
    } else {
        dir_size_bytes(&media_dir)
    };
    let mut tsv_path_result: Option<String> = None;
    let mut apkg_path_result: Option<String> = None;

    if export_format == "apkg" {
        emit(
            progress,
            "tsv",
            "flashcards.progress.generatingApkg",
            90,
            100,
            90.0,
            HashMap::new(),
        );
        let apkg_path = output_dir.join(format!("{}.apkg", sanitize_filename(&config.deck_name)));
        if cancel.is_cancelled() {
            return Ok(None);
        }
        generate_apkg(matched, config, &media_dir, &apkg_path)?;
        apkg_path_result = Some(apkg_path.to_string_lossy().to_string());
    } else {
        emit(
            progress,
            "tsv",
            "flashcards.progress.generatingTsv",
            90,
            100,
            90.0,
            HashMap::new(),
        );
        let media_str = media_dir.to_str().unwrap_or("");
        let tsv_content = generate_tsv(matched, config, media_str, media_str, media_str);
        let tsv_path = output_dir.join(format!("{}.tsv", sanitize_filename(&config.deck_name)));
        if cancel.is_cancelled() {
            return Ok(None);
        }
        let final_media_dir =
            output_dir.join(format!("{}.media", sanitize_filename(&config.deck_name)));
        tsv_publication::publish(
            media_workspace,
            &final_media_dir,
            &tsv_path,
            tsv_content.as_bytes(),
        )?;
        tsv_path_result = Some(tsv_path.to_string_lossy().to_string());
    }

    // Exact, unlike a pre-generation estimate: the bytes are already on disk.
    // An apkg is self-contained; a TSV leaves its media loose in media_dir.
    let output_size_bytes = match &apkg_path_result {
        Some(path) => std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        None => media_size_bytes,
    };

    Ok(Some(PublishedExport {
        tsv_path: tsv_path_result,
        apkg_path: apkg_path_result,
        size_bytes: output_size_bytes,
    }))
}

pub(super) struct ExportWorkspace {
    directory: tempfile::TempDir,
    output: PathBuf,
    pub media: PathBuf,
}

impl ExportWorkspace {
    pub fn new(config: &FlashcardConfig) -> Result<Self, String> {
        let output_dir = PathBuf::from(&config.output_dir);

        std::fs::create_dir_all(&output_dir).map_err(|e| {
            format!(
                "Cannot create output directory '{}': {e}",
                output_dir.display()
            )
        })?;
        let export_format = config.export_format.as_deref().unwrap_or("tsv");

        let media_workspace = if export_format == "apkg" {
            tempfile::tempdir()
        } else {
            tempfile::Builder::new()
                .prefix(".vesta-export-")
                .tempdir_in(&output_dir)
        }
        .map_err(|error| format!("Cannot create media staging directory: {error}"))?;
        let media_dir = media_workspace.path().join("collection.media");
        std::fs::create_dir_all(&media_dir)
            .map_err(|e| format!("Cannot create output dir: {}", e))?;

        Ok(Self {
            directory: media_workspace,
            output: output_dir,
            media: media_dir,
        })
    }
}

/// Total size of the files directly inside `dir`, ignoring anything it cannot
/// stat. Media directories are flat, so this does not recurse.
fn dir_size_bytes(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}
