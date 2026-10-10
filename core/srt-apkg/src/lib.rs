//! Minimal helpers for reading and writing Anki `.apkg` ZIP archives.
//!
//! `.apkg` files are plain ZIP archives. This crate provides two functions
//! that are shared by `srt-flashcards` and `srt-refine` to avoid duplication:
//!
//! - [`unzip_to`]: extract a ZIP archive into a directory.
//! - [`zip_from_dir`]: create a ZIP archive from the flat contents of a directory.

use std::fs;
use std::io::{self, BufReader, BufWriter, Seek, Write};
use std::path::Path;

fn is_media_extension(ext: &str) -> bool {
    matches!(
        ext,
        "mp3"
            | "m4a"
            | "wav"
            | "ogg"
            | "opus"
            | "flac"
            | "aac"
            | "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "gif"
            | "mp4"
            | "mkv"
            | "avi"
            | "mov"
            | "webm"
            | "ttf"
            | "otf"
            | "woff"
            | "woff2"
    )
}

/// Extract the ZIP archive at `zip_path` into `dest_dir`.
///
/// Existing files in `dest_dir` are overwritten. Subdirectories found inside
/// the archive are created as needed.
pub fn unzip_to(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open ZIP archive: {e}"))?;
    let reader = BufReader::with_capacity(128 * 1024, file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid ZIP archive: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("ZIP index error: {e}"))?;
        let outpath = match entry.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if entry.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent()
                && !parent.exists()
            {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Cannot create extracted file: {e}"))?;
            let mut writer = BufWriter::with_capacity(128 * 1024, outfile);
            io::copy(&mut entry, &mut writer)
                .map_err(|e| format!("Error writing extracted file: {e}"))?;
            writer
                .flush()
                .map_err(|e| format!("Error flushing extracted file: {e}"))?;
        }
    }
    Ok(())
}

/// Create a ZIP archive at `zip_path` containing every *file* (non-recursive)
/// directly inside `src_dir`.
///
/// Pre-compressed media files use Stored (0 compression) to save CPU/time,
/// while other files use Deflate.
pub fn zip_from_dir(src_dir: &Path, zip_path: &Path) -> Result<(), String> {
    let parent = zip_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Cannot create output ZIP: {e}"))?;
    let same_directory = fs::canonicalize(src_dir).map_err(|e| e.to_string())?
        == fs::canonicalize(parent).map_err(|e| e.to_string())?;
    let temporary_name = temporary.path().file_name().map(|name| name.to_owned());
    let writer = BufWriter::with_capacity(256 * 1024, temporary.as_file_mut());
    let mut zip = zip::ZipWriter::new(writer);
    let options_deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let options_stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let entries =
        fs::read_dir(src_dir).map_err(|e| format!("Cannot read source directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        // An archive in its source directory must not include itself, its
        // previous version, or the temporary archive being written.
        if same_directory
            && (path.file_name() == zip_path.file_name()
                || path.file_name() == temporary_name.as_deref())
        {
            continue;
        }
        if fs::metadata(&path)
            .map_err(|e| format!("Cannot inspect source file: {e}"))?
            .is_file()
        {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| "Invalid filename in source directory".to_string())?;
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let opt = if is_media_extension(&ext) {
                options_stored
            } else {
                options_deflated
            };
            zip.start_file(filename, opt)
                .map_err(|e| format!("ZIP start_file error: {e}"))?;
            let f = fs::File::open(&path).map_err(|e| format!("Cannot read source file: {e}"))?;
            let mut reader = BufReader::with_capacity(128 * 1024, f);
            io::copy(&mut reader, &mut zip).map_err(|e| format!("ZIP copy error: {e}"))?;
        }
    }

    finish_zip(zip)?;
    if let Ok(metadata) = fs::metadata(zip_path) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| format!("ZIP sync error: {e}"))?;
    temporary
        .persist(zip_path)
        .map_err(|e| format!("Cannot publish ZIP: {}", e.error))?;
    Ok(())
}

fn finish_zip<W: Write + Seek>(zip: zip::ZipWriter<W>) -> Result<(), String> {
    let mut writer = zip.finish().map_err(|e| format!("ZIP finish error: {e}"))?;
    // BufWriter's Drop ignores flush errors; success must include this flush.
    writer.flush().map_err(|e| format!("ZIP flush error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_is_media_extension() {
        assert!(is_media_extension("mp3"));
        assert!(is_media_extension("wav"));
        assert!(is_media_extension("webp"));
        assert!(is_media_extension("jpg"));
        assert!(is_media_extension("mp4"));
        assert!(is_media_extension("ttf"));

        assert!(!is_media_extension("txt"));
        assert!(!is_media_extension("json"));
        assert!(!is_media_extension("anki2"));
        assert!(!is_media_extension(""));
    }

    #[test]
    fn round_trip() {
        let src = tempfile::tempdir().unwrap();
        let mut f = fs::File::create(src.path().join("hello.txt")).unwrap();
        f.write_all(b"hello apkg").unwrap();

        let mut f_media = fs::File::create(src.path().join("audio.mp3")).unwrap();
        f_media.write_all(b"fake audio data").unwrap();

        let zip_path = src.path().join("out.zip");
        zip_from_dir(src.path(), &zip_path).unwrap();
        assert!(zip_path.exists());

        // Verify compression methods in the created zip archive
        let zip_file = fs::File::open(&zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(zip_file).unwrap();
        for i in 0..archive.len() {
            let entry = archive.by_index(i).unwrap();
            if entry.name() == "audio.mp3" {
                assert_eq!(entry.compression(), zip::CompressionMethod::Stored);
            } else if entry.name() == "hello.txt" {
                assert_eq!(entry.compression(), zip::CompressionMethod::Deflated);
            }
        }

        let dest = tempfile::tempdir().unwrap();
        unzip_to(&zip_path, dest.path()).unwrap();
        let content = fs::read_to_string(dest.path().join("hello.txt")).unwrap();
        assert_eq!(content, "hello apkg");
        let media_content = fs::read(dest.path().join("audio.mp3")).unwrap();
        assert_eq!(media_content, b"fake audio data");
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;

    #[test]
    fn archives_in_the_source_directory_do_not_include_themselves() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("export.apkg");
        fs::write(directory.path().join("hello.txt"), "hello").unwrap();
        for _ in 0..2 {
            zip_from_dir(directory.path(), &output).unwrap();
            let mut archive = zip::ZipArchive::new(fs::File::open(&output).unwrap()).unwrap();
            assert_eq!(archive.len(), 1);
            assert_eq!(archive.by_index(0).unwrap().name(), "hello.txt");
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn missing_source_preserves_an_existing_archive() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("export.apkg");
        fs::write(&output, "existing export").unwrap();
        assert!(zip_from_dir(&directory.path().join("missing"), &output).is_err());
        assert_eq!(fs::read_to_string(&output).unwrap(), "existing export");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn archive_completion_propagates_final_flush_errors() {
        struct FailingFlush(std::io::Cursor<Vec<u8>>);
        impl Write for FailingFlush {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.write(bytes)
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::other("disk flush failed"))
            }
        }
        impl Seek for FailingFlush {
            fn seek(&mut self, position: io::SeekFrom) -> io::Result<u64> {
                self.0.seek(position)
            }
        }
        let mut zip = zip::ZipWriter::new(FailingFlush(std::io::Cursor::new(Vec::new())));
        zip.start_file("hello.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"hello").unwrap();
        assert!(finish_zip(zip).unwrap_err().contains("disk flush failed"));
    }
}
