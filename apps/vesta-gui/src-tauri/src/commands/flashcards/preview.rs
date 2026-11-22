//! Preview files live for the application session, not for a single command.
//! Each request owns a distinct temporary path, including overlapping requests.
use std::sync::Mutex;

use crate::media_stream::MediaAllowlist;

#[derive(Default)]
pub struct PreviewWorkspace {
    directory: Mutex<Option<tempfile::TempDir>>,
    allowlist: MediaAllowlist,
}

impl PreviewWorkspace {
    pub fn with_allowlist(allowlist: MediaAllowlist) -> Self {
        Self {
            directory: Mutex::new(None),
            allowlist,
        }
    }

    pub fn allocate(&self, extension: &str) -> Result<tempfile::TempPath, String> {
        let mut directory = self.directory.lock().map_err(|error| error.to_string())?;
        if directory.is_none() {
            *directory = Some(
                tempfile::Builder::new()
                    .prefix("vesta-previews-")
                    .tempdir()
                    .map_err(|error| error.to_string())?,
            );
        }
        let directory = directory.as_ref().ok_or("Missing preview directory")?;
        let temp_path = tempfile::Builder::new()
            .prefix("preview-")
            .suffix(&format!(".{extension}"))
            .tempfile_in(directory.path())
            .map(tempfile::NamedTempFile::into_temp_path)
            .map_err(|error| error.to_string())?;
        if let Err(error) = self.allowlist.allow_file(&temp_path) {
            return Err(format!("Failed to allowlist preview media: {error}"));
        }
        Ok(temp_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_previews_remain_distinct_until_session_ends() {
        let workspace = PreviewWorkspace::default();
        let older = workspace.allocate("jpg").unwrap();
        let newer = workspace.allocate("jpg").unwrap();
        assert_ne!(&*older, &*newer);
        // The new request finishes first. The old one cannot overwrite it.
        std::fs::write(&newer, b"new frame").unwrap();
        let published = newer.keep().unwrap();
        std::fs::write(&older, b"old frame").unwrap();
        let old_path = older.to_path_buf();
        drop(older); // failed/cancelled commands clean up their own file
        assert!(!old_path.exists());
        assert_eq!(std::fs::read(&published).unwrap(), b"new frame");
        drop(workspace);
        assert!(!published.exists());
    }

    #[test]
    fn allocated_preview_files_are_registered_in_shared_media_allowlist() {
        let allowlist = MediaAllowlist::default();
        let workspace = PreviewWorkspace::with_allowlist(allowlist.clone());
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"outside").unwrap();

        let allocated = workspace.allocate("mp3").unwrap();
        std::fs::write(&allocated, b"clip").unwrap();
        let kept = allocated.keep().unwrap();
        assert!(allowlist.is_allowed(&kept));
        assert!(!allowlist.is_allowed(outside.path()));

        drop(workspace);
        assert!(!allowlist.is_allowed(&kept));
    }
}
