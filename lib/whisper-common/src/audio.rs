use anyhow::{anyhow, Context as _, Result};
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// Convert input audio/video to 16kHz mono WAV using ffmpeg, with cancellation support
pub async fn convert_to_wav(
    ffmpeg_path: &str,
    input_path: &Path,
    output_path: &Path,
    cancel_token: Option<&CancellationToken>,
) -> Result<()> {
    let mut child = tokio::process::Command::new(ffmpeg_path)
        .args([
            "-y",
            "-i", input_path.to_str().ok_or_else(|| anyhow!("Invalid input path"))?,
            "-ar", "16000",
            "-ac", "1",
            "-c:a", "pcm_s16le",
            output_path.to_str().ok_or_else(|| anyhow!("Invalid output path"))?,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn FFmpeg process for audio conversion")?;

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
    if !status.success() {
        let output = child.wait_with_output().await.ok();
        let stderr = output.map(|o| String::from_utf8_lossy(&o.stderr).to_string()).unwrap_or_default();
        anyhow::bail!("FFmpeg audio conversion failed: {}", stderr);
    }

    Ok(())
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
