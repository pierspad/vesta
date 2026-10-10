use anyhow::{Context as _, Result};
use std::path::Path;
use tokio_util::sync::CancellationToken;

fn ffmpeg_command(ffmpeg_path: &str) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut command = tokio::process::Command::new(ffmpeg_path);
    #[cfg(windows)]
    {
        command.creation_flags(0x0800_4000);
    }
    command
}

pub async fn convert_to_wav(
    ffmpeg_path: &str,
    input_path: &Path,
    output_path: &Path,
    cancel_token: Option<&CancellationToken>,
) -> Result<()> {
    let mut command = ffmpeg_command(ffmpeg_path);
    command
        .args(["-nostdin", "-y", "-i"])
        .arg(input_path)
        .args(["-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(output_path);
    run_audio_command(&mut command, "conversion", cancel_token).await
}

/// Extract a bounded mono PCM segment using the same cancellable process runner.
pub async fn extract_wav_segment(
    ffmpeg_path: &str,
    input: &Path,
    output: &Path,
    start_seconds: f64,
    duration_seconds: f64,
    cancel: Option<&CancellationToken>,
) -> Result<()> {
    anyhow::ensure!(
        start_seconds.is_finite() && start_seconds >= 0.0,
        "Invalid audio segment start"
    );
    anyhow::ensure!(
        duration_seconds.is_finite() && duration_seconds > 0.0,
        "Invalid audio segment duration"
    );
    let mut command = ffmpeg_command(ffmpeg_path);
    command
        .args(["-nostdin", "-y", "-ss"])
        .arg(format!("{start_seconds:.2}"))
        .arg("-i")
        .arg(input)
        .arg("-t")
        .arg(format!("{duration_seconds:.2}"))
        .args(["-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(output);
    run_audio_command(&mut command, "segment extraction", cancel).await
}

pub async fn segment_to_wav_chunks(
    ffmpeg_path: &str,
    input_path: &Path,
    out_dir: &Path,
    segment_seconds: u32,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<std::path::PathBuf>> {
    let pattern = out_dir.join("chunk_%05d.wav");
    let mut command = ffmpeg_command(ffmpeg_path);
    command
        .args(["-nostdin", "-y", "-i"])
        .arg(input_path)
        .args([
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            "-f",
            "segment",
            "-segment_time",
        ])
        .arg(segment_seconds.max(1).to_string())
        .arg(&pattern);
    run_audio_command(&mut command, "segmentation", cancel_token).await?;

    let mut chunks = Vec::new();
    for entry in std::fs::read_dir(out_dir).context("Failed to read segment output dir")? {
        let entry = entry.context("Failed to read audio chunk directory entry")?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "wav")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("chunk_"))
            && entry
                .file_type()
                .context("Failed to inspect audio chunk")?
                .is_file()
        {
            chunks.push(path);
        }
    }
    chunks.sort();

    if chunks.is_empty() {
        anyhow::bail!("FFmpeg produced no audio chunks");
    }
    Ok(chunks)
}

// Drain stderr concurrently with waiting to avoid pipe deadlocks, but retain
// only its tail so long conversions cannot grow diagnostics without a bound.
const STDERR_LIMIT: usize = 64 * 1024;

async fn read_stderr(mut reader: impl tokio::io::AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut tail = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(tail);
        }
        let excess = (tail.len() + count).saturating_sub(STDERR_LIMIT);
        tail.drain(..excess);
        tail.extend_from_slice(&buffer[..count]);
    }
}

async fn run_audio_command(
    command: &mut tokio::process::Command,
    operation: &str,
    cancel_token: Option<&CancellationToken>,
) -> Result<()> {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        anyhow::bail!("Audio {operation} cancelled");
    }
    let mut child = command
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn FFmpeg process for audio {operation}"))?;
    let stderr = child
        .stderr
        .take()
        .context("Failed to capture FFmpeg stderr")?;
    // No detached reader task: cancellation drops both borrowed futures before
    // killing/reaping the child. Dropping this whole future also kills the child.
    let result = {
        let completion = async {
            tokio::try_join!(
                async {
                    child
                        .wait()
                        .await
                        .context("Failed to wait for FFmpeg process")
                },
                async {
                    read_stderr(stderr)
                        .await
                        .context("Failed to read FFmpeg stderr")
                },
            )
        };
        if let Some(token) = cancel_token {
            tokio::select! {
                biased;
                _ = token.cancelled() => None,
                result = completion => Some(result),
            }
        } else {
            Some(completion.await)
        }
    };
    let (status, stderr) = match result {
        None => {
            child
                .kill()
                .await
                .context("Failed to stop cancelled FFmpeg process")?;
            anyhow::bail!("Audio {operation} cancelled");
        }
        Some(Err(error)) => {
            if let Err(cleanup) = child.kill().await {
                return Err(error.context(format!("Also failed to stop FFmpeg: {cleanup}")));
            }
            return Err(error);
        }
        Some(Ok(output)) => output,
    };
    if !status.success() {
        anyhow::bail!(
            "FFmpeg audio {operation} failed ({status}): {}",
            String::from_utf8_lossy(&stderr).trim()
        );
    }
    Ok(())
}

pub fn read_wav_to_f32(wav_path: &Path) -> Result<Vec<f32>> {
    let reader = hound::WavReader::open(wav_path)
        .with_context(|| format!("Failed to open WAV file {}", wav_path.display()))?;
    let spec = reader.spec();
    anyhow::ensure!(spec.channels > 0, "WAV has no audio channels");
    anyhow::ensure!(
        (1..=32).contains(&spec.bits_per_sample),
        "Unsupported WAV bit depth: {}",
        spec.bits_per_sample
    );
    let expected_frames = reader.len() as usize / usize::from(spec.channels);
    let result = match spec.sample_format {
        hound::SampleFormat::Int => {
            // Floating-point exponentiation avoids signed shifts overflowing or
            // turning the normalization factor negative for 32-bit PCM.
            let scale = 2.0_f32.powi(1 - i32::from(spec.bits_per_sample));
            downmix(
                reader
                    .into_samples::<i32>()
                    .map(|sample| sample.map(|s| s as f32)),
                spec.channels,
                expected_frames,
                scale,
            )
        }
        hound::SampleFormat::Float => downmix(
            reader.into_samples::<f32>(),
            spec.channels,
            expected_frames,
            1.0,
        ),
    };
    result.with_context(|| format!("Failed to decode WAV file {}", wav_path.display()))
}

fn downmix(
    samples: impl Iterator<Item = hound::Result<f32>>,
    channels: u16,
    expected_frames: usize,
    scale: f32,
) -> Result<Vec<f32>> {
    let mut samples = samples.enumerate();
    let mut mono = Vec::with_capacity(expected_frames);
    let inv_channels = 1.0 / f32::from(channels);
    while let Some((index, first)) = samples.next() {
        let mut sum = first.with_context(|| format!("Invalid WAV sample {index}"))?;
        anyhow::ensure!(sum.is_finite(), "Non-finite WAV sample {index}");
        for _ in 1..channels {
            let (index, sample) = samples.next().context("Incomplete WAV audio frame")?;
            let sample = sample.with_context(|| format!("Invalid WAV sample {index}"))?;
            anyhow::ensure!(sample.is_finite(), "Non-finite WAV sample {index}");
            sum += sample;
        }
        // Preserve the existing mono/stereo arithmetic ordering for PCM.
        let value = if channels <= 2 {
            sum * (scale * inv_channels)
        } else {
            (sum * inv_channels) * scale
        };
        anyhow::ensure!(value.is_finite(), "WAV downmix overflowed");
        mono.push(value);
    }
    Ok(mono)
}

#[cfg(test)]
mod tests;
