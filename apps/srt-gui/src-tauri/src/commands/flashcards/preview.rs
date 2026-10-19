//! Preview files live for the application session, not for a single command.
//! Each request owns a distinct temporary path, including overlapping requests.
use std::sync::Mutex;

#[derive(Default)]
pub struct PreviewWorkspace {
    directory: Mutex<Option<tempfile::TempDir>>,
}

impl PreviewWorkspace {
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
        tempfile::Builder::new()
            .prefix("preview-")
            .suffix(&format!(".{extension}"))
            .tempfile_in(directory.path())
            .map(tempfile::NamedTempFile::into_temp_path)
            .map_err(|error| error.to_string())
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
}
