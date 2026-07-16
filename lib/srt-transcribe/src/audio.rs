use anyhow::{anyhow, Context as _, Result};
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// Build an ffmpeg command with platform-appropriate flags: on Windows the
/// console window is suppressed and the priority lowered so long conversions
/// don't freeze the user's machine.
fn ffmpeg_command(ffmpeg_path: &str) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut command = tokio::process::Command::new(ffmpeg_path);
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW (0x0800_0000) | BELOW_NORMAL_PRIORITY_CLASS (0x0000_4000)
        command.creation_flags(0x0800_4000);
    }
    command
}

/// Convert input audio/video to 16kHz mono WAV using ffmpeg, with cancellation support
pub async fn convert_to_wav(
    ffmpeg_path: &str,
    input_path: &Path,
    output_path: &Path,
    cancel_token: Option<&CancellationToken>,
) -> Result<()> {
    let mut child = ffmpeg_command(ffmpeg_path)
        .arg("-y")
        .arg("-i")
        .arg(input_path)
        .args(["-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(output_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn FFmpeg process for audio conversion")?;

    let mut stderr_reader = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;
    
    // Spawn task to read stderr asynchronously to prevent deadlocks
    let stderr_handle = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        let _ = stderr_reader.read_to_end(&mut buf).await;
        buf
    });

    let res = if let Some(token) = cancel_token {
        tokio::select! {
            res = child.wait() => res,
            _ = token.cancelled() => {
                let _ = child.kill().await;
                anyhow::bail!("Audio conversion cancelled");
            }
        }
    } else {
        child.wait().await
    };

    let status = res.context("Failed to wait for FFmpeg process")?;
    let stderr_bytes = stderr_handle.await.unwrap_or_default();

    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
        anyhow::bail!("FFmpeg audio conversion failed: {}", stderr);
    }

    Ok(())
}

/// Split input audio/video into fixed-length 16kHz mono WAV chunks using ffmpeg's
/// segment muxer. Returns the chunk file paths in chronological order.
///
/// Used by the cloud transcription path: chunk N has timestamps relative to its
/// own start, so the caller offsets each chunk by `index * segment_seconds`.
pub async fn segment_to_wav_chunks(
    ffmpeg_path: &str,
    input_path: &Path,
    out_dir: &Path,
    segment_seconds: u32,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<std::path::PathBuf>> {
    let pattern = out_dir.join("chunk_%05d.wav");
    let mut child = ffmpeg_command(ffmpeg_path)
        .arg("-y")
        .arg("-i")
        .arg(input_path)
        .args(["-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", "-f", "segment", "-segment_time"])
        .arg(segment_seconds.max(1).to_string())
        .arg(&pattern)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn FFmpeg process for audio segmentation")?;

    let mut stderr_reader = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;
    let stderr_handle = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        let _ = stderr_reader.read_to_end(&mut buf).await;
        buf
    });

    let res = if let Some(token) = cancel_token {
        tokio::select! {
            res = child.wait() => res,
            _ = token.cancelled() => {
                let _ = child.kill().await;
                anyhow::bail!("Audio segmentation cancelled");
            }
        }
    } else {
        child.wait().await
    };

    let status = res.context("Failed to wait for FFmpeg process")?;
    let stderr_bytes = stderr_handle.await.unwrap_or_default();
    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
        anyhow::bail!("FFmpeg audio segmentation failed: {}", stderr);
    }

    // Collect and sort the produced chunks.
    let mut chunks: Vec<std::path::PathBuf> = std::fs::read_dir(out_dir)
        .context("Failed to read segment output dir")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "wav").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("chunk_"))
                    .unwrap_or(false)
        })
        .collect();
    chunks.sort();

    if chunks.is_empty() {
        anyhow::bail!("FFmpeg produced no audio chunks");
    }
    Ok(chunks)
}

/// Read WAV file into f32 samples for whisper-rs, averaging channels if stereo
pub fn read_wav_to_f32(wav_path: &Path) -> Result<Vec<f32>> {
    let reader = hound::WavReader::open(wav_path)
        .context("Failed to open WAV file")?;
    
    let spec = reader.spec();
    
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample.saturating_sub(1))) as f32;
            reader.into_samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => {
            reader.into_samples::<f32>()
                .filter_map(|s| s.ok())
                .collect()
        }
    };
    
    // If stereo, convert to mono by averaging channels
    if spec.channels == 2 {
        Ok(samples.chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    (chunk[0] + chunk[1]) / 2.0
                } else {
                    chunk[0]
                }
            })
            .collect())
    } else {
        Ok(samples)
    }
}
