//! Local OCR model catalog and atomic download management.

use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};
use srt_download::download_to;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// Metadata describing an OCR model in the catalog and its local availability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcrModelInfo {
    /// Unique catalog identifier for this model.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Approximate download size string.
    pub size: String,
    /// Whether the model file is currently cached on disk.
    pub downloaded: bool,
}

/// Catalog of supported local OCR models: `(id, name, filename, url, size)`.
pub const OCR_MODELS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "paddleocr-v4-mobile-multi",
        "PaddleOCR v4 Mobile (Multilingual)",
        "paddleocr-v4-mobile-multi.onnx",
        "https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/ch_PP-OCRv4_rec_infer.onnx",
        "~11 MB",
    ),
    (
        "paddleocr-v4-mobile-cjk",
        "PaddleOCR v4 Mobile (CJK)",
        "paddleocr-v4-mobile-cjk.onnx",
        "https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/ch_PP-OCRv4_rec_mobile_infer.onnx",
        "~11 MB",
    ),
    (
        "paddleocr-v4-mobile-latin",
        "PaddleOCR v4 Mobile (Latin)",
        "paddleocr-v4-mobile-latin.onnx",
        "https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/en_PP-OCRv4_rec_infer.onnx",
        "~10 MB",
    ),
];

fn resolve_models_dir(override_dir: Option<&OsStr>) -> PathBuf {
    if let Some(dir) = override_dir
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vesta")
        .join("ocr")
}

/// Returns the local directory where OCR models are cached.
///
/// Honors `VESTA_OCR_CACHE_DIR` when set and non-empty.
pub fn get_models_dir() -> Result<PathBuf> {
    Ok(resolve_models_dir(
        std::env::var_os("VESTA_OCR_CACHE_DIR").as_deref(),
    ))
}

fn find_model_entry(
    model_id: &str,
) -> Result<&'static (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
)> {
    if model_id.is_empty() || model_id.contains(['/', '\\']) || model_id.contains("..") {
        return Err(anyhow!("Invalid OCR model ID: {model_id}"));
    }
    OCR_MODELS
        .iter()
        .find(|(id, _, _, _, _)| *id == model_id)
        .ok_or_else(|| anyhow!("Unknown OCR model: {model_id}"))
}

/// Validates that `model_id` is present in [`OCR_MODELS`] and contains no path traversal.
pub fn validate_model_id(model_id: &str) -> Result<()> {
    find_model_entry(model_id).map(|_| ())
}

fn model_file_path_in(models_dir: &Path, model_id: &str) -> Result<PathBuf> {
    let (_, _, filename, _, _) = find_model_entry(model_id)?;
    Ok(models_dir.join(filename))
}

/// Returns the expected on-disk path for a cataloged OCR model.
pub fn model_file_path(model_id: &str) -> Result<PathBuf> {
    let models_dir = get_models_dir()?;
    model_file_path_in(&models_dir, model_id)
}

fn list_models_in(models_dir: &Path) -> Vec<OcrModelInfo> {
    OCR_MODELS
        .iter()
        .map(|&(id, name, filename, _, size)| OcrModelInfo {
            id: id.to_string(),
            name: name.to_string(),
            size: size.to_string(),
            downloaded: models_dir.join(filename).is_file(),
        })
        .collect()
}

/// Lists all cataloged OCR models along with their local cache status.
pub fn list_models() -> Result<Vec<OcrModelInfo>> {
    let models_dir = get_models_dir()?;
    Ok(list_models_in(&models_dir))
}

fn remove_if_present(path: &Path) -> Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("Failed to remove model file: {}", path.display()))
        }
    }
}

fn uninstall_model_in(models_dir: &Path, model_id: &str) -> Result<bool> {
    remove_if_present(&model_file_path_in(models_dir, model_id)?)
}

/// Deletes a cached OCR model if present. Returns `true` if a file was removed.
pub fn uninstall_model(model_id: &str) -> Result<bool> {
    let models_dir = get_models_dir()?;
    uninstall_model_in(&models_dir, model_id)
}

async fn download_model_with_url<F>(
    models_dir: &Path,
    model_id: &str,
    url: &str,
    progress_callback: F,
    cancel_token: Option<&CancellationToken>,
) -> Result<PathBuf>
where
    F: Fn(u32) + Send + 'static,
{
    let path = model_file_path_in(models_dir, model_id)?;
    download_to(url, &path, progress_callback, cancel_token).await
}

/// Downloads a cataloged OCR model into the local cache directory with progress and cancellation.
pub async fn download_model<F>(
    model_id: &str,
    progress_callback: F,
    cancel_token: Option<&CancellationToken>,
) -> Result<PathBuf>
where
    F: Fn(u32) + Send + 'static,
{
    let (_, _, _, url, _) = find_model_entry(model_id)?;
    let models_dir = get_models_dir()?;
    download_model_with_url(&models_dir, model_id, url, progress_callback, cancel_token).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn resolve_models_dir_respects_non_empty_override() {
        let custom = OsString::from("/tmp/vesta-ocr-override");
        assert_eq!(
            resolve_models_dir(Some(custom.as_os_str())),
            PathBuf::from("/tmp/vesta-ocr-override")
        );

        let empty = OsString::from("");
        let fallback = resolve_models_dir(Some(empty.as_os_str()));
        assert!(fallback.ends_with(Path::new("vesta").join("ocr")));
        assert_eq!(resolve_models_dir(None), fallback);
    }

    #[test]
    fn validates_catalog_ids_and_rejects_traversal() {
        for bad in [
            "",
            "unknown",
            "../paddleocr-v4-mobile-multi",
            "paddleocr-v4-mobile-multi/../../etc/passwd",
            "..\\evil",
        ] {
            assert!(validate_model_id(bad).is_err(), "{bad}");
            assert!(model_file_path(bad).is_err(), "{bad}");
        }

        for &(id, _, filename, _, _) in OCR_MODELS {
            assert!(validate_model_id(id).is_ok());
            assert_eq!(model_file_path(id).unwrap().file_name().unwrap(), filename);
        }
    }

    #[tokio::test]
    async fn list_download_and_uninstall_lifecycle_with_mock_server() {
        let dir = tempfile::tempdir().unwrap();
        let initial = list_models_in(dir.path());
        assert_eq!(initial.len(), OCR_MODELS.len());
        assert!(initial.iter().all(|m| !m.downloaded));

        let payload = b"ONNX-MOCK-MODEL-BYTES";
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await.unwrap();
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.write_all(payload).await.unwrap();
        });

        let reported = Arc::new(Mutex::new(Vec::new()));
        let reported_cb = reported.clone();
        let url = format!("http://{addr}/model.onnx");
        let saved_path = download_model_with_url(
            dir.path(),
            "paddleocr-v4-mobile-multi",
            &url,
            move |pct| reported_cb.lock().unwrap().push(pct),
            None,
        )
        .await
        .unwrap();
        server.await.unwrap();

        assert_eq!(std::fs::read(&saved_path).unwrap(), payload);
        assert_eq!(reported.lock().unwrap().last().copied(), Some(100));

        let after_dl = list_models_in(dir.path());
        let downloaded_entry = after_dl
            .iter()
            .find(|m| m.id == "paddleocr-v4-mobile-multi")
            .unwrap();
        assert!(downloaded_entry.downloaded);

        assert!(uninstall_model_in(dir.path(), "paddleocr-v4-mobile-multi").unwrap());
        assert!(!uninstall_model_in(dir.path(), "paddleocr-v4-mobile-multi").unwrap());
        assert!(!saved_path.exists());
    }

    #[tokio::test]
    async fn cancelled_download_fails_without_publishing() {
        let dir = tempfile::tempdir().unwrap();
        let token = CancellationToken::new();
        token.cancel();
        let err = download_model_with_url(
            dir.path(),
            "paddleocr-v4-mobile-latin",
            "http://127.0.0.1:1/unused.onnx",
            |_| {},
            Some(&token),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("cancel"));
        assert!(
            !model_file_path_in(dir.path(), "paddleocr-v4-mobile-latin")
                .unwrap()
                .exists()
        );
    }
}
