use crate::plugins::manifest::PluginManifest;
use crate::plugins::url::ResolvedPluginSource;
use flate2::read::GzDecoder;
use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// Maximum allowed uncompressed archive size (100 MB).
const MAX_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;

/// Directories and files excluded when copying local plugin folders.
const EXCLUDED_NAMES: &[&str] = &[
    ".git",
    ".env",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    ".DS_Store",
    "target",
];

/// Downloads archive bytes via HTTP with a 30s timeout.
pub async fn download_archive(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(45))
        .user_agent("vesta-plugin-manager/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download archive from {url}: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status {} for {}",
            response.status(),
            url
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read archive body: {e}"))?;

    if bytes.is_empty() {
        return Err("Downloaded archive is empty".to_string());
    }

    Ok(bytes.to_vec())
}

/// Fallback: clones the git repository with `--depth 1` into the destination directory.
pub fn git_clone_fallback(git_url: &str, git_ref: &str, dest_dir: &Path) -> Result<(), String> {
    if dest_dir.exists() {
        let _ = fs::remove_dir_all(dest_dir);
    }
    fs::create_dir_all(dest_dir).map_err(|e| format!("Failed to create destination dir: {e}"))?;

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");

    if git_ref != "HEAD" {
        cmd.arg("--branch").arg(git_ref);
    }

    cmd.arg(git_url).arg(dest_dir);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute 'git clone': {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone failed: {stderr}"));
    }

    // Clean up .git directory to save disk space
    let git_dir = dest_dir.join(".git");
    if git_dir.exists() {
        let _ = fs::remove_dir_all(git_dir);
    }

    Ok(())
}

/// Safely checks that a relative path does not escape the destination root.
fn is_safe_path(path: &Path) -> bool {
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// Checks if bytes start with gzip magic header (0x1F, 0x8B).
fn is_gzip(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
}

/// Checks if bytes start with ZIP magic header ('P', 'K', 0x03, 0x04).
fn is_zip(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4b && bytes[2] == 0x03 && bytes[3] == 0x04
}

/// Safely extracts an archive (.tar.gz or .zip) into `dest_dir`, stripping
/// any common single root directory created by GitHub/GitLab tarballs.
pub fn extract_archive(bytes: &[u8], dest_dir: &Path) -> Result<(), String> {
    if dest_dir.exists() {
        let _ = fs::remove_dir_all(dest_dir);
    }
    fs::create_dir_all(dest_dir).map_err(|e| format!("Failed to create destination dir: {e}"))?;

    if is_gzip(bytes) {
        extract_tar_gz(bytes, dest_dir)
    } else if is_zip(bytes) {
        extract_zip(bytes, dest_dir)
    } else {
        extract_tar_gz(bytes, dest_dir).or_else(|_| extract_zip(bytes, dest_dir))
    }
}

fn extract_tar_gz(bytes: &[u8], dest_dir: &Path) -> Result<(), String> {
    let gz = GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);

    let mut paths: Vec<PathBuf> = Vec::new();
    let gz2 = GzDecoder::new(bytes);
    let mut archive2 = tar::Archive::new(gz2);

    for entry in archive2
        .entries()
        .map_err(|e| format!("Failed to read tar entries: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Corrupted tar entry: {e}"))?;
        let entry_type = entry.header().entry_type();

        // Reject symlinks and hardlinks for security
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err("Tar archive contains unsupported symlink or hardlink entry".to_string());
        }

        let path = entry
            .path()
            .map_err(|e| format!("Invalid tar path: {e}"))?
            .into_owned();
        if !is_safe_path(&path) {
            return Err(format!("Unsafe path in archive: {}", path.display()));
        }
        paths.push(path);
    }

    let prefix_to_strip = detect_common_prefix(&paths);
    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read tar archive entries: {e}"))?;

    let mut total_unpacked_bytes: u64 = 0;

    for entry in entries {
        let mut entry = entry.map_err(|e| format!("Corrupted tar entry: {e}"))?;
        let entry_type = entry.header().entry_type();

        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err("Tar archive contains unsupported symlink or hardlink entry".to_string());
        }

        let original_path = entry
            .path()
            .map_err(|e| format!("Invalid tar path: {e}"))?
            .into_owned();

        let stripped_path = if let Some(prefix) = &prefix_to_strip {
            match original_path.strip_prefix(prefix) {
                Ok(p) => p.to_path_buf(),
                Err(_) => original_path,
            }
        } else {
            original_path
        };

        if stripped_path.as_os_str().is_empty() {
            continue;
        }

        let target_path = dest_dir.join(&stripped_path);

        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&target_path).map_err(|e| {
                format!("Failed to create directory {}: {e}", target_path.display())
            })?;
        } else {
            let size = entry.header().size().unwrap_or(0);
            total_unpacked_bytes += size;
            if total_unpacked_bytes > MAX_ARCHIVE_BYTES {
                return Err(format!(
                    "Archive extracted size exceeds safety limit ({MAX_ARCHIVE_BYTES} bytes)"
                ));
            }

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create parent dir {}: {e}", parent.display())
                })?;
            }
            entry
                .unpack(&target_path)
                .map_err(|e| format!("Failed to extract file {}: {e}", target_path.display()))?;
        }
    }

    Ok(())
}

fn extract_zip(bytes: &[u8], dest_dir: &Path) -> Result<(), String> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to read zip archive: {e}"))?;

    let mut paths: Vec<PathBuf> = Vec::new();
    let mut total_unpacked_bytes: u64 = 0;

    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("Corrupted zip entry at index {i}: {e}"))?;

        // Check for symlinks
        if let Some(mode) = file.unix_mode()
            && (mode & 0o170000) == 0o120000
        {
            return Err("Zip archive contains unsupported symlink entry".to_string());
        }

        total_unpacked_bytes += file.size();
        if total_unpacked_bytes > MAX_ARCHIVE_BYTES {
            return Err(format!(
                "Archive extracted size exceeds safety limit ({MAX_ARCHIVE_BYTES} bytes)"
            ));
        }

        let enclosed = file
            .enclosed_name()
            .ok_or_else(|| format!("Unsafe zip path in entry at index {i}"))?;
        paths.push(enclosed.to_path_buf());
    }

    let prefix_to_strip = detect_common_prefix(&paths);

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;

        let enclosed = file
            .enclosed_name()
            .ok_or_else(|| format!("Unsafe zip path in entry at index {i}"))?;

        let stripped_path = if let Some(prefix) = &prefix_to_strip {
            match enclosed.strip_prefix(prefix) {
                Ok(p) => p.to_path_buf(),
                Err(_) => enclosed.to_path_buf(),
            }
        } else {
            enclosed.to_path_buf()
        };

        if stripped_path.as_os_str().is_empty() {
            continue;
        }

        let target_path = dest_dir.join(&stripped_path);

        if file.is_dir() {
            fs::create_dir_all(&target_path).map_err(|e| {
                format!("Failed to create directory {}: {e}", target_path.display())
            })?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create parent dir {}: {e}", parent.display())
                })?;
            }
            let mut outfile = fs::File::create(&target_path).map_err(|e| {
                format!(
                    "Failed to create target file {}: {e}",
                    target_path.display()
                )
            })?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to write file {}: {e}", target_path.display()))?;
        }
    }

    Ok(())
}

/// Detects if all paths in the archive share a single root component (e.g. `repo-main/`).
fn detect_common_prefix(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }

    let mut first_components: Vec<Component> = Vec::new();
    for p in paths {
        if let Some(first) = p.components().next() {
            if first_components.is_empty() {
                first_components.push(first);
            } else if first_components[0] != first {
                return None;
            }
        }
    }

    if let Some(first) = first_components.first() {
        let has_nested = paths.iter().any(|p| p.components().count() > 1);
        if has_nested {
            return Some(PathBuf::from(first.as_os_str()));
        }
    }

    None
}

/// Reads and validates `vesta-plugin.json` from the root of an extracted directory.
pub fn load_manifest_from_dir(dir: &Path) -> Result<PluginManifest, String> {
    let manifest_path = dir.join("vesta-plugin.json");
    if !manifest_path.exists() {
        return Err(format!(
            "Missing 'vesta-plugin.json' manifest in plugin root ({})",
            dir.display()
        ));
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read 'vesta-plugin.json': {e}"))?;

    let manifest: PluginManifest = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse 'vesta-plugin.json': {e}"))?;

    manifest.validate()?;
    Ok(manifest)
}

/// Recursively copies files and subdirectories from src to dst,
/// skipping build/cache directories and symlinks.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| format!("Failed to create {}: {e}", dst.display()))?;
    }

    for entry in
        fs::read_dir(src).map_err(|e| format!("Failed to read dir {}: {e}", src.display()))?
    {
        let entry = entry.map_err(|e| format!("Corrupted dir entry: {e}"))?;
        let name = entry.file_name().to_string_lossy().to_string();

        if EXCLUDED_NAMES.contains(&name.as_str()) {
            continue;
        }

        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to get file type: {e}"))?;

        // Skip symlinks
        if file_type.is_symlink() {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "Failed to copy {} to {}: {e}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }

    Ok(())
}

/// High-level function: downloads, clones, or copies from `source` into `dest_dir`,
/// and verifies the manifest.
pub async fn fetch_and_extract_plugin(
    source: &ResolvedPluginSource,
    dest_dir: &Path,
) -> Result<PluginManifest, String> {
    // Check if source is a local directory or local archive
    let local_path = Path::new(&source.git_clone_url);
    if local_path.is_dir() {
        if dest_dir.exists() {
            let _ = fs::remove_dir_all(dest_dir);
        }
        copy_dir_recursive(local_path, dest_dir)?;
        return load_manifest_from_dir(dest_dir);
    } else if local_path.is_file() {
        let bytes = fs::read(local_path).map_err(|e| {
            format!(
                "Failed to read local archive file {}: {e}",
                local_path.display()
            )
        })?;
        extract_archive(&bytes, dest_dir)?;
        return load_manifest_from_dir(dest_dir);
    }

    let mut downloaded = false;

    if let Some(archive_url) = &source.archive_url {
        match download_archive(archive_url).await {
            Ok(bytes) => match extract_archive(&bytes, dest_dir) {
                Ok(()) => {
                    downloaded = true;
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Archive extraction failed ({e}), falling back to git clone."
                    );
                }
            },
            Err(e) => {
                eprintln!("Warning: Archive download failed ({e}), falling back to git clone.");
            }
        }
    }

    if !downloaded {
        git_clone_fallback(&source.git_clone_url, &source.git_ref, dest_dir)?;
    }

    load_manifest_from_dir(dest_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;

    #[test]
    fn test_is_safe_path() {
        assert!(is_safe_path(Path::new("foo/bar/baz.txt")));
        assert!(is_safe_path(Path::new("vesta-plugin.json")));
        assert!(!is_safe_path(Path::new("../escape.txt")));
        assert!(!is_safe_path(Path::new("foo/../../escape.txt")));
        assert!(!is_safe_path(Path::new("/root/escape.txt")));
    }

    #[test]
    fn test_tar_gz_extraction_with_prefix_strip() {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("extracted");

        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut tar = tar::Builder::new(&mut enc);

            let manifest_content = r#"{
                "id": "test-plugin",
                "name": "Test Plugin",
                "version": "1.0.0",
                "apiVersion": 1,
                "entry": { "command": ["python3", "main.py"] }
            }"#;

            let mut header = tar::Header::new_gnu();
            header.set_size(manifest_content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();

            tar.append_data(
                &mut header,
                "repo-main/vesta-plugin.json",
                manifest_content.as_bytes(),
            )
            .unwrap();
            tar.finish().unwrap();
        }
        let gz_bytes = enc.finish().unwrap();

        extract_archive(&gz_bytes, &dest).expect("extraction failed");

        let manifest = load_manifest_from_dir(&dest).expect("manifest load failed");
        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.name, "Test Plugin");
    }

    #[test]
    fn test_copy_dir_recursive_excludes() {
        let temp_src = tempfile::tempdir().unwrap();
        let temp_dst = tempfile::tempdir().unwrap();

        fs::write(temp_src.path().join("vesta-plugin.json"), "{}").unwrap();
        fs::write(temp_src.path().join(".env"), "SECRET=123").unwrap();
        fs::create_dir(temp_src.path().join(".git")).unwrap();
        fs::write(temp_src.path().join(".git").join("config"), "git config").unwrap();
        fs::create_dir(temp_src.path().join("node_modules")).unwrap();
        fs::write(
            temp_src.path().join("node_modules").join("pkg.js"),
            "module",
        )
        .unwrap();

        copy_dir_recursive(temp_src.path(), temp_dst.path()).unwrap();

        assert!(temp_dst.path().join("vesta-plugin.json").exists());
        assert!(!temp_dst.path().join(".env").exists());
        assert!(!temp_dst.path().join(".git").exists());
        assert!(!temp_dst.path().join("node_modules").exists());
    }
}
