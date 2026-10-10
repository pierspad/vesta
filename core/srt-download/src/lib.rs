//! Download mechanics, independent from font/model catalogs and desktop events.
use anyhow::{Context, Result};
use futures::StreamExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

/// Reuse a nonempty cached file or stream into a unique temporary sibling.
/// Publish only after HTTP, writes and flush succeed. Cancellation includes
/// waiting for headers/body, and dropping the future removes partial files.
/// Concurrent callers never share a temporary file or overwrite a completed one.
pub async fn download_to(
    url: &str,
    path: &Path,
    progress: impl Fn(u32),
    cancel: Option<&CancellationToken>,
) -> Result<PathBuf> {
    let download = async {
        match tokio::fs::metadata(path).await {
            Ok(metadata) => {
                anyhow::ensure!(
                    metadata.is_file() && metadata.len() > 0,
                    "Cached download is not a nonempty file: {}",
                    path.display()
                );
                progress(100);
                return Ok(path.to_path_buf());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Failed to inspect cached download"),
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create download directory")?;
        let temporary = tempfile::Builder::new()
            .prefix(".vesta-download-")
            .tempfile_in(parent)
            .context("Failed to create download temporary file")?
            .into_temp_path();
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create download client")?;
        let response = client
            .get(url)
            .send()
            .await
            .context("Failed to send download request")?
            .error_for_status()
            .context("Download HTTP error")?;
        let total = response.content_length();
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(&temporary)
            .await
            .context("Failed to open partial download")?;
        let mut downloaded = 0u64;
        let mut last_emit = Instant::now();
        progress(0);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read download chunk")?;
            file.write_all(&chunk)
                .await
                .context("Failed to write download chunk")?;
            downloaded += chunk.len() as u64;
            if let Some(total) = total.filter(|total| *total > 0)
                && last_emit.elapsed() >= Duration::from_millis(150)
            {
                // Completion is emitted only after successful publication.
                progress(((u128::from(downloaded) * 100 / u128::from(total)).min(99)) as u32);
                last_emit = Instant::now();
            }
        }
        anyhow::ensure!(downloaded > 0, "Download was empty");
        file.flush().await.context("Failed to flush download")?;
        file.sync_all()
            .await
            .context("Failed to synchronize download")?;
        drop(file);
        match temporary.persist_noclobber(path) {
            Ok(()) => {}
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = tokio::fs::metadata(path).await?;
                anyhow::ensure!(
                    metadata.is_file() && metadata.len() > 0,
                    "Concurrent download destination is not a nonempty file"
                );
            }
            Err(error) => return Err(error.error).context("Failed to publish download"),
        }
        progress(100);
        Ok(path.to_path_buf())
    };
    if let Some(cancel) = cancel {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => anyhow::bail!("Download cancelled"),
            result = download => result,
        }
    } else {
        download.await
    }
}

#[cfg(test)]
mod tests;
