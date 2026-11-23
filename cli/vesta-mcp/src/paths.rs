use std::path::{Path, PathBuf};

/// Restrict inputs to configured directories and create outputs in fresh subdirectories.
/// This is an application policy, not an OS sandbox against concurrent filesystem edits.
#[derive(Clone)]
pub struct Paths {
    reads: Vec<PathBuf>,
    output: PathBuf,
}

impl Paths {
    pub fn new(reads: Vec<PathBuf>, output: PathBuf) -> anyhow::Result<Self> {
        anyhow::ensure!(!reads.is_empty(), "At least one read root is required");
        let roots = reads
            .into_iter()
            .map(std::fs::canonicalize)
            .collect::<std::io::Result<Vec<_>>>()?;
        anyhow::ensure!(
            roots.iter().all(|root| root.is_dir()),
            "Read roots must be existing directories"
        );
        let output = output.canonicalize()?;
        anyhow::ensure!(output.is_dir(), "Output root must be an existing directory");
        anyhow::ensure!(
            output.to_str().is_some() && roots.iter().all(|root| root.to_str().is_some()),
            "Configured roots must be valid UTF-8"
        );
        Ok(Self {
            reads: roots,
            output,
        })
    }

    pub fn read_roots(&self) -> &[PathBuf] {
        &self.reads
    }
    pub fn output_root(&self) -> &Path {
        &self.output
    }

    pub fn input(&self, path: &Path, subtitle: bool) -> Result<String, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("Cannot resolve input: {e}"))?;
        if !self.reads.iter().any(|root| path.starts_with(root)) {
            return Err("Input is outside the configured read roots".into());
        }
        let metadata = path.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("Input must be a regular file".into());
        }
        if subtitle {
            let ext = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(
                ext.as_str(),
                "srt" | "ass" | "ssa" | "vtt" | "webvtt" | "lrc"
            ) {
                return Err("Expected an SRT, ASS, SSA, VTT or LRC subtitle file".into());
            }
            if metadata.len() > 16 * 1024 * 1024 {
                return Err("Subtitle input exceeds 16 MiB".into());
            }
        }
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| "Input path must be valid UTF-8".into())
    }

    /// Resolve a deck input (`.apkg` or `.tsv`) inside any read root OR the output root
    /// so generated outputs can be chained into `refine_flashcards` and `merge_apkg`.
    pub fn input_deck(&self, path: &Path, allowed_exts: &[&str]) -> Result<String, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("Cannot resolve input: {e}"))?;
        if !self.reads.iter().any(|root| path.starts_with(root)) && !path.starts_with(&self.output)
        {
            return Err("Input is outside the configured read and output roots".into());
        }
        let metadata = path.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("Input must be a regular file".into());
        }
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !allowed_exts
            .iter()
            .any(|expected| ext.eq_ignore_ascii_case(expected))
        {
            return Err(format!(
                "Expected a {} file",
                allowed_exts
                    .iter()
                    .map(|e| e.to_ascii_uppercase())
                    .collect::<Vec<_>>()
                    .join(" or ")
            ));
        }
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| "Input path must be valid UTF-8".into())
    }

    pub fn new_output(&self) -> Result<PathBuf, String> {
        // Each job owns a new directory, so existing user exports are never overwritten.
        tempfile::Builder::new()
            .prefix("vesta-")
            .tempdir_in(&self.output)
            .map(|dir| dir.keep())
            .map_err(|e| e.to_string())
    }
}
