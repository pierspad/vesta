//! Bundled sample media: a short clip + reference SRTs published as a
//! release asset, so a tester without a media file handy can still run a
//! meaningful benchmark in one click. Reuses the same 90s clip already
//! committed for the golden Detour test suite
//! (`Test_Subs/fixtures/detour/detour_clip_90s.mp4`) rather than shipping
//! new, unvetted media — see the `zip-sample` job in `whisper-bench.yml`.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

const SAMPLE_ZIP_URL: &str = "https://github.com/pierspad/vesta/releases/download/whisper-bench-latest/whisper-bench-sample.zip";

const SAMPLE_MEDIA_FILE: &str = "detour_clip_90s.mp4";

/// Downloads (once) and extracts the bundled sample zip, returning the path
/// to the sample media file ready to hand to `run_bench`.
pub async fn ensure_sample_media(app: &AppHandle, log: impl Fn(String)) -> Result<PathBuf, String> {
    let dest_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?
        .join("sample");
    let media_path = dest_dir.join(SAMPLE_MEDIA_FILE);
    if media_path.exists() {
        return Ok(media_path);
    }

    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    log("Downloading bundled sample episode...".to_string());
    let bytes = reqwest::get(SAMPLE_ZIP_URL)
        .await
        .map_err(|e| format!("Failed to reach {SAMPLE_ZIP_URL}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Sample download failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    log("Extracting sample...".to_string());
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("Bad zip: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let out_path = dest_dir.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out_file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
    }

    if !media_path.exists() {
        return Err(format!(
            "Sample archive didn't contain the expected {SAMPLE_MEDIA_FILE}"
        ));
    }
    log("Sample ready.".to_string());
    Ok(media_path)
}
