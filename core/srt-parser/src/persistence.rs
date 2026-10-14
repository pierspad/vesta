//! Atomic publication for subtitle files and their companion exports/sessions.
use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

/// Reject an existing output alias of the input before conversion or resume.
/// File identity detects symlinks and hard links on supported platforms.
/// This is a preflight check, not protection against external path replacement.
pub fn ensure_distinct_files(input: &Path, output: &Path) -> Result<()> {
    fs::metadata(input)
        .with_context(|| format!("Cannot inspect input file: {}", input.display()))?;
    match fs::metadata(output) {
        Ok(_) => anyhow::ensure!(
            !same_file::is_same_file(input, output)
                .context("Cannot compare input and output files")?,
            "Input and output refer to the same file; choose a separate output path"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("Cannot inspect output file"),
    }
    Ok(())
}

/// Write a temporary sibling, flush and synchronize it, then replace the target.
/// Existing `fs::Permissions` are retained (not ownership or extended ACLs).
/// New files use the temporary file's restrictive permissions.
/// Existing symlinks are followed, as with
/// `fs::write`; dangling symlinks are rejected instead of silently replaced.
/// Readers see either the old or complete new file. Hard links to the old inode
/// retain the old contents. This does not guarantee directory durability after
/// power loss, nor serialize concurrent writers (last successful writer wins).
pub fn atomic_write(path: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
    atomic_write_with(path, |file| {
        file.write_all(bytes)?;
        Ok(())
    })
}

/// The callback must finish/flush any buffered wrappers before returning.
/// Callback, flush and publication failures leave the previous target intact.
pub fn atomic_write_with<T>(
    path: impl AsRef<Path>,
    write: impl FnOnce(&mut File) -> Result<T>,
) -> Result<T> {
    let path = path.as_ref();
    let resolved;
    let target = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            resolved = fs::canonicalize(path).context("Cannot resolve output symlink")?;
            resolved.as_path()
        }
        Ok(_) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => path,
        Err(error) => return Err(error).context("Cannot inspect output path"),
    };
    let permissions = match fs::metadata(target) {
        Ok(metadata) => {
            anyhow::ensure!(
                metadata.is_file(),
                "Output is not a regular file: {}",
                path.display()
            );
            anyhow::ensure!(
                !metadata.permissions().readonly(),
                "Output is read-only: {}",
                path.display()
            );
            Some(metadata.permissions())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).context("Cannot inspect output file"),
    };
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".vesta-save-")
        .tempfile_in(parent)
        .context("Cannot create output temporary file")?;
    let result = write(temporary.as_file_mut()).context("Cannot write output temporary file")?;
    temporary
        .flush()
        .context("Cannot flush output temporary file")?;
    if let Some(permissions) = permissions {
        temporary
            .as_file()
            .set_permissions(permissions)
            .context("Cannot preserve output permissions")?;
    }
    temporary
        .as_file()
        .sync_all()
        .context("Cannot synchronize output temporary file")?;
    temporary
        .persist(target)
        .map_err(|error| error.error)
        .with_context(|| format!("Cannot replace output file: {}", path.display()))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_identical_and_hard_linked_inputs_but_accepts_separate_output() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.srt");
        let alias = dir.path().join("alias.srt");
        fs::write(&input, "original").unwrap();
        fs::hard_link(&input, &alias).unwrap();
        assert!(ensure_distinct_files(&input, &input).is_err());
        assert!(ensure_distinct_files(&input, &alias).is_err());
        assert!(ensure_distinct_files(&input, &dir.path().join("new.srt")).is_ok());
        assert_eq!(fs::read(&input).unwrap(), b"original");
    }

    #[test]
    fn failed_writer_keeps_old_file_and_removes_temporary() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output.srt");
        fs::write(&output, b"original").unwrap();
        let result: Result<()> = atomic_write_with(&output, |file| {
            file.write_all(b"incomplete")?;
            anyhow::bail!("simulated write failure");
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&output).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn replaces_complete_file_and_rejects_directory_destination() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("日本語.srt");
        atomic_write(&output, b"first").unwrap();
        atomic_write(&output, "日本語".as_bytes()).unwrap();
        assert_eq!(fs::read_to_string(&output).unwrap(), "日本語");
        assert!(atomic_write(dir.path(), b"bad").is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failure_does_not_publish_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("new.srt");
        let result: Result<()> = atomic_write_with(&output, |file| {
            file.write_all(b"partial")?;
            anyhow::bail!("failed before publication");
        });
        assert!(result.is_err());
        assert!(!output.exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn read_only_target_is_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("read-only.srt");
        fs::write(&output, "original").unwrap();
        let original_permissions = fs::metadata(&output).unwrap().permissions();
        let mut readonly = original_permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&output, readonly).unwrap();
        let result = atomic_write(&output, b"new");
        fs::set_permissions(&output, original_permissions).unwrap();
        assert!(result.is_err());
        assert_eq!(fs::read(&output).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn preserves_permissions_and_symlink_and_rejects_dangling_link() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.srt");
        let link = dir.path().join("link.srt");
        fs::write(&target, "old").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
        symlink("target.srt", &link).unwrap();
        atomic_write(&link, b"new").unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert_eq!(
            fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o640
        );
        fs::remove_file(&target).unwrap();
        assert!(atomic_write(&link, b"bad").is_err());
        assert!(!target.exists());
    }
}
