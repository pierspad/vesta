use anyhow::{anyhow, Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use futures::StreamExt as _;

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

/// Get the directory where Whisper models are stored
pub fn get_models_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".cache")
    });
    Ok(cache_dir.join("whisper"))
}

/// Check if a whisper model file exists locally and return its path
pub fn model_file_path(model_id: &str) -> Result<PathBuf> {
    let models_dir = get_models_dir()?;
    let filename = if model_id == "large" {
        "ggml-large-v3.bin".to_string()
    } else {
        format!("ggml-{model_id}.bin")
    };
    Ok(models_dir.join(filename))
}

/// Validate if the model_id is a known Whisper model
pub fn validate_model_id(model_id: &str) -> Result<()> {
    if WHISPER_MODELS.iter().any(|(id, _, _, _)| *id == model_id) {
        Ok(())
    } else {
        Err(anyhow!("Unknown Whisper model: {model_id}"))
    }
}

/// List all Whisper models with download status
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

/// Uninstall (delete) a specific Whisper model
pub fn uninstall_model(model_id: &str) -> Result<bool> {
    validate_model_id(model_id)?;
    let path = model_file_path(model_id)?;
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove model file: {}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Download a Whisper model from Hugging Face with progress reporting and cancellation support
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
    
    // If already exists, return immediately
    if path.exists() {
        progress_callback(100);
        return Ok(path);
    }
    
    // Create models directory if needed
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    
    let partial = path.with_extension("bin.partial");
    let url_id = if model_id == "large" {
        "large-v3"
    } else {
        model_id
    };
    
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{url_id}.bin"
    );
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request for Whisper model download")?;
        
    if !response.status().is_success() {
        anyhow::bail!(
            "Whisper model download failed with status: {}",
            response.status()
        );
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
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                let _ = tokio::fs::remove_file(&partial).await;
                anyhow::bail!("Download cancelled");
            }
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
    Ok(path)
}
