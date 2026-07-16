use anyhow::{Context as _, Result, anyhow};
use futures::StreamExt as _;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperModelInfo {
    pub id: String,
    pub name: String,
    pub size: String,
    pub speed: String,
    pub downloaded: bool,
}

pub const WHISPER_MODELS: &[(&str, &str, &str, &str)] = &[
    ("tiny", "Tiny", "~75 MB", "~32x"),
    ("base", "Base", "~150 MB", "~16x"),
    ("small", "Small", "~500 MB", "~6x"),
    ("medium", "Medium", "~1.5 GB", "~2x"),
    ("large", "Large", "~3 GB", "~1x"),
];

pub fn get_models_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".cache")
    });
    Ok(cache_dir.join("whisper"))
}

pub fn model_file_path(model_id: &str) -> Result<PathBuf> {
    let models_dir = get_models_dir()?;
    let filename = if model_id == "large" {
        "ggml-large-v3.bin".to_string()
    } else {
        format!("ggml-{model_id}.bin")
    };
    Ok(models_dir.join(filename))
}

pub fn validate_model_id(model_id: &str) -> Result<()> {
    if WHISPER_MODELS.iter().any(|(id, _, _, _)| *id == model_id) {
        Ok(())
    } else {
        Err(anyhow!("Unknown Whisper model: {model_id}"))
    }
}

pub fn list_models() -> Result<Vec<WhisperModelInfo>> {
    let mut result = Vec::new();
    for &(id, name, size, speed) in WHISPER_MODELS {
        let downloaded = model_file_path(id)
            .map(|path| path.exists())
            .unwrap_or(false);

        result.push(WhisperModelInfo {
            id: id.to_string(),
            name: name.to_string(),
            size: size.to_string(),
            speed: speed.to_string(),
            downloaded,
        });
    }
    Ok(result)
}

pub fn uninstall_model(model_id: &str) -> Result<bool> {
    validate_model_id(model_id)?;
    remove_if_present(&model_file_path(model_id)?)
}

pub const VAD_MODELS: &[(&str, &str, &str, &str)] = &[
    (
        "v5.1.2",
        "ggml-silero-v5.1.2.bin",
        "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v5.1.2.bin",
        "~0.9 MB",
    ),
    (
        "v6.2.0",
        "ggml-silero-v6.2.0.bin",
        "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin",
        "~0.9 MB",
    ),
];

pub const DEFAULT_VAD_MODEL_ID: &str = "v5.1.2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadModelInfo {
    pub id: String,
    pub size: String,
    pub downloaded: bool,
}

fn vad_model_entry(
    id: &str,
) -> Result<&'static (&'static str, &'static str, &'static str, &'static str)> {
    VAD_MODELS
        .iter()
        .find(|(entry_id, _, _, _)| *entry_id == id)
        .ok_or_else(|| anyhow!("Unknown VAD model: {id}"))
}

/// Path where a given Silero VAD variant is (or would be) stored.
pub fn vad_model_path(id: &str) -> Result<PathBuf> {
    let (_, filename, _, _) = vad_model_entry(id)?;
    Ok(get_models_dir()?.join(filename))
}

/// Whether a given Silero VAD variant is present on disk.
pub fn vad_model_installed(id: &str) -> bool {
    vad_model_path(id).map(|p| p.exists()).unwrap_or(false)
}

/// List every known VAD variant with its download status.
pub fn list_vad_models() -> Vec<VadModelInfo> {
    VAD_MODELS
        .iter()
        .map(|&(id, _, _, size)| VadModelInfo {
            id: id.to_string(),
            size: size.to_string(),
            downloaded: vad_model_installed(id),
        })
        .collect()
}

/// Delete a given Silero VAD variant. Returns `true` if a file was removed.
pub fn uninstall_vad_model(id: &str) -> Result<bool> {
    remove_if_present(&vad_model_path(id)?)
}

/// Download a given Silero VAD variant with progress reporting and cancellation.
pub async fn download_vad_model<F>(
    id: &str,
    progress_callback: F,
    cancel_token: Option<&CancellationToken>,
) -> Result<PathBuf>
where
    F: Fn(u32) + Send + 'static,
{
    let (_, _, url, _) = vad_model_entry(id)?;
    let path = vad_model_path(id)?;
    download_to(url, &path, progress_callback, cancel_token).await
}

fn remove_if_present(path: &std::path::Path) -> Result<bool> {
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to remove model file: {}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn download_model<F>(
    model_id: &str,
    progress_callback: F,
    cancel_token: Option<&CancellationToken>,
) -> Result<PathBuf>
where
    F: Fn(u32) + Send + 'static,
{
    validate_model_id(model_id)?;
    let path = model_file_path(model_id)?;

    let url_id = if model_id == "large" {
        "large-v3"
    } else {
        model_id
    };
    let url =
        format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{url_id}.bin");

    download_to(&url, &path, progress_callback, cancel_token).await
}

/// Stream `url` into `path` (via a `.partial` sibling renamed on completion),
/// reporting whole-percent progress and honouring cancellation. Returns
/// immediately when `path` already exists.
async fn download_to<F>(
    url: &str,
    path: &std::path::Path,
    progress_callback: F,
    cancel_token: Option<&CancellationToken>,
) -> Result<PathBuf>
where
    F: Fn(u32) + Send + 'static,
{
    if path.exists() {
        progress_callback(100);
        return Ok(path.to_path_buf());
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let partial = path.with_extension("bin.partial");

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to send request for model download")?;

    if !response.status().is_success() {
        anyhow::bail!("Model download failed with status: {}", response.status());
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&partial)
        .await
        .context("Failed to create partial download file")?;

    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    let mut last_emit = std::time::Instant::now();

    progress_callback(0);

    while let Some(chunk_result) = stream.next().await {
        if let Some(token) = cancel_token
            && token.is_cancelled()
        {
            let _ = tokio::fs::remove_file(&partial).await;
            anyhow::bail!("Download cancelled");
        }

        let chunk = chunk_result.context("Error reading response chunk")?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .context("Failed to write chunk to file")?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let percentage = (downloaded as f64 / total_size as f64 * 100.0) as u32;
            if last_emit.elapsed() >= std::time::Duration::from_millis(150) || percentage == 100 {
                progress_callback(percentage);
                last_emit = std::time::Instant::now();
            }
        }
    }

    tokio::io::AsyncWriteExt::flush(&mut file)
        .await
        .context("Failed to flush download file")?;

    tokio::fs::rename(&partial, &path)
        .await
        .context("Failed to rename partial file to destination")?;

    progress_callback(100);
    Ok(path.to_path_buf())
}
