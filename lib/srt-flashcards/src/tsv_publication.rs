//! Stage all media before touching an existing export. Roll back the media swap
//! if TSV publication fails. This is not an atomic multi-file crash transaction.
use std::{
    fs,
    path::{Path, PathBuf},
};

fn lock_path(tsv: &Path) -> PathBuf {
    tsv.with_file_name(format!(
        ".{}.export.lock",
        tsv.file_name().unwrap_or_default().to_string_lossy()
    ))
}

pub(crate) fn publish(
    workspace: tempfile::TempDir,
    media_destination: &Path,
    tsv_destination: &Path,
    content: &[u8],
) -> Result<(), String> {
    // Keep the lock inode: unlinking it could allow two publishers to lock
    // different files under the same name. OS locks release on process exit.
    let lock_path = lock_path(tsv_destination);
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| format!("Cannot open export lock: {error}"))?;
    lock.try_lock().map_err(|error| {
        format!("Cannot publish this deck while another export is publishing: {error}")
    })?;

    let had_previous = match fs::symlink_metadata(media_destination) {
        Ok(metadata) => {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err("Media destination must be a directory, not a file or symlink".into());
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(format!("Cannot inspect media destination: {error}")),
    };
    let backup = workspace.path().join("previous.media");
    // A TSV symlink into the media tree would change meaning during the swap
    // and could overwrite one of the newly generated media files.
    if had_previous
        && fs::symlink_metadata(tsv_destination)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        let media_root = fs::canonicalize(media_destination)
            .map_err(|error| format!("Cannot resolve media destination: {error}"))?;
        let tsv_target = fs::canonicalize(tsv_destination)
            .map_err(|error| format!("Cannot resolve TSV destination: {error}"))?;
        if tsv_target.starts_with(media_root) {
            return Err("TSV destination must not point inside its media directory".into());
        }
    }
    if had_previous {
        fs::rename(media_destination, &backup)
            .map_err(|error| format!("Cannot preserve previous media: {error}"))?;
    }
    let staged_media = workspace.path().join("collection.media");
    if let Err(error) = fs::rename(&staged_media, media_destination) {
        if had_previous && let Err(restore) = fs::rename(&backup, media_destination) {
            let recovery = workspace.keep();
            return Err(format!(
                "Cannot publish media: {error}; cannot restore old media: {restore}. Previous media retained in {}",
                recovery.display()
            ));
        }
        return Err(format!("Cannot publish media: {error}"));
    }
    if let Err(error) = srt_parser::persistence::atomic_write(tsv_destination, content) {
        let restored = fs::remove_dir_all(media_destination).and_then(|()| {
            if had_previous {
                fs::rename(&backup, media_destination)
            } else {
                Ok(())
            }
        });
        if let Err(restore) = restored {
            let recovery = workspace.keep();
            return Err(format!(
                "Cannot write TSV: {error:#}; media rollback failed: {restore}. Recovery files retained in {}",
                recovery.display()
            ));
        }
        return Err(format!(
            "Cannot write TSV: {error:#}; previous export preserved"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staged(root: &Path) -> tempfile::TempDir {
        let workspace = tempfile::Builder::new()
            .prefix(".vesta-export-")
            .tempdir_in(root)
            .unwrap();
        fs::create_dir(workspace.path().join("collection.media")).unwrap();
        fs::write(
            workspace.path().join("collection.media/new.mp3"),
            b"new audio",
        )
        .unwrap();
        workspace
    }

    #[test]
    fn successful_publication_replaces_both_media_and_tsv() {
        let dir = tempfile::tempdir().unwrap();
        let media = dir.path().join("Deck.media");
        let tsv = dir.path().join("Deck.tsv");
        fs::create_dir(&media).unwrap();
        fs::write(media.join("old.mp3"), b"old audio").unwrap();
        fs::write(&tsv, b"old tsv").unwrap();
        publish(staged(dir.path()), &media, &tsv, b"new tsv").unwrap();
        assert_eq!(fs::read(&tsv).unwrap(), b"new tsv");
        assert_eq!(fs::read(media.join("new.mp3")).unwrap(), b"new audio");
        assert!(!media.join("old.mp3").exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3); // TSV, media, stable lock
    }

    #[test]
    fn failed_tsv_write_restores_previous_media_and_cleans_staging() {
        let dir = tempfile::tempdir().unwrap();
        let media = dir.path().join("Deck.media");
        let tsv = dir.path().join("Deck.tsv");
        fs::create_dir(&media).unwrap();
        fs::write(media.join("old.mp3"), b"old audio").unwrap();
        fs::write(&tsv, b"old tsv").unwrap();
        let permissions = fs::metadata(&tsv).unwrap().permissions();
        let mut readonly = permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&tsv, readonly).unwrap();
        let result = publish(staged(dir.path()), &media, &tsv, b"new tsv");
        fs::set_permissions(&tsv, permissions).unwrap();
        assert!(result.is_err());
        assert_eq!(fs::read(&tsv).unwrap(), b"old tsv");
        assert_eq!(fs::read(media.join("old.mp3")).unwrap(), b"old audio");
        assert!(!media.join("new.mp3").exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
    }

    #[test]
    fn overlapping_publication_is_rejected_without_touching_existing_media() {
        let dir = tempfile::tempdir().unwrap();
        let media = dir.path().join("Deck.media");
        let tsv = dir.path().join("Deck.tsv");
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path(&tsv))
            .unwrap();
        lock.try_lock().unwrap();
        fs::create_dir(&media).unwrap();
        fs::write(media.join("old.mp3"), b"old audio").unwrap();
        assert!(publish(staged(dir.path()), &media, &tsv, b"new tsv").is_err());
        assert_eq!(fs::read(media.join("old.mp3")).unwrap(), b"old audio");
        assert!(!tsv.exists());
    }

    #[test]
    fn failed_media_swap_restores_previous_directory() {
        let dir = tempfile::tempdir().unwrap();
        let media = dir.path().join("Deck.media");
        let tsv = dir.path().join("Deck.tsv");
        fs::create_dir(&media).unwrap();
        fs::write(media.join("old.mp3"), b"old audio").unwrap();
        fs::write(&tsv, b"old tsv").unwrap();
        let missing_stage = tempfile::Builder::new()
            .prefix(".vesta-export-")
            .tempdir_in(dir.path())
            .unwrap();
        assert!(publish(missing_stage, &media, &tsv, b"new tsv").is_err());
        assert_eq!(fs::read(media.join("old.mp3")).unwrap(), b"old audio");
        assert_eq!(fs::read(&tsv).unwrap(), b"old tsv");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
    }

    #[test]
    fn failed_new_export_does_not_leave_published_media() {
        let dir = tempfile::tempdir().unwrap();
        let media = dir.path().join("Deck.media");
        let tsv = dir.path().join("Deck.tsv");
        fs::create_dir(&tsv).unwrap(); // Invalid destination forces TSV failure after media swap.
        assert!(publish(staged(dir.path()), &media, &tsv, b"new tsv").is_err());
        assert!(!media.exists());
        assert!(tsv.is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn tsv_symlink_into_media_is_rejected_before_swap() {
        let dir = tempfile::tempdir().unwrap();
        let media = dir.path().join("Deck.media");
        let tsv = dir.path().join("Deck.tsv");
        fs::create_dir(&media).unwrap();
        fs::write(media.join("new.mp3"), b"old audio").unwrap();
        std::os::unix::fs::symlink(media.join("new.mp3"), &tsv).unwrap();
        assert!(publish(staged(dir.path()), &media, &tsv, b"new tsv").is_err());
        assert_eq!(fs::read(media.join("new.mp3")).unwrap(), b"old audio");
        assert!(fs::symlink_metadata(tsv).unwrap().file_type().is_symlink());
    }
}
