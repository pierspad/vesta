use std::io::Write;
use std::path::Path;

pub(crate) fn write_apkg_archive(
    output_path: &Path,
    database_path: &Path,
    media_manifest_path: &Path,
    media_files: &[(String, std::path::PathBuf)],
) -> Result<(), String> {
    srt_parser::persistence::atomic_write_with(output_path, |apkg_file| {
        let writer = std::io::BufWriter::with_capacity(256 * 1024, apkg_file);
        let mut zip = zip::ZipWriter::new(writer);
        let options_deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let options_stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("collection.anki2", options_deflated)
            .map_err(|error| anyhow::anyhow!("ZIP error: {error}"))?;
        let db_bytes = std::fs::read(database_path)
            .map_err(|error| anyhow::anyhow!("Cannot read DB: {error}"))?;
        zip.write_all(&db_bytes)
            .map_err(|error| anyhow::anyhow!("ZIP write error: {error}"))?;

        zip.start_file("media", options_deflated)
            .map_err(|error| anyhow::anyhow!("ZIP error: {error}"))?;
        let media_json = std::fs::read(media_manifest_path)
            .map_err(|error| anyhow::anyhow!("Cannot read media JSON: {error}"))?;
        zip.write_all(&media_json)
            .map_err(|error| anyhow::anyhow!("ZIP write error: {error}"))?;

        for (index, file_path) in media_files {
            zip.start_file(index, options_stored)
                .map_err(|error| anyhow::anyhow!("ZIP error adding media: {error}"))?;
            let file = std::fs::File::open(file_path)
                .map_err(|error| anyhow::anyhow!("Cannot read media file: {error}"))?;
            let mut reader = std::io::BufReader::with_capacity(128 * 1024, file);
            std::io::copy(&mut reader, &mut zip)
                .map_err(|error| anyhow::anyhow!("ZIP write error: {error}"))?;
        }

        let mut writer = zip
            .finish()
            .map_err(|error| anyhow::anyhow!("ZIP finish error: {error}"))?;
        writer.flush()?;
        Ok(())
    })
    .map_err(|error| format!("Cannot publish APKG: {error:#}"))
}
