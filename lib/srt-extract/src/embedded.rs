//! Text subtitle tracks inside media containers (bitmap tracks require OCR).
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Stdio, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedSubtitleTrack {
    pub index: u32,
    pub codec: String,
    pub language: String,
    pub title: String,
    pub text_based: bool,
}

fn text_codec(codec: &str) -> bool {
    matches!(
        codec,
        "subrip"
            | "srt"
            | "ass"
            | "ssa"
            | "webvtt"
            | "mov_text"
            | "text"
            | "microdvd"
            | "sami"
            | "subviewer"
            | "subviewer1"
            | "realtext"
            | "jacosub"
    )
}

pub async fn list_embedded_subtitles(
    ffprobe: &str,
    input: &Path,
) -> Result<Vec<EmbeddedSubtitleTrack>> {
    let mut command = tokio::process::Command::new(ffprobe);
    command.kill_on_drop(true);
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        command
            .args([
                "-v",
                "error",
                "-select_streams",
                "s",
                "-show_entries",
                "stream=index,codec_name:stream_tags=language,title",
                "-of",
                "json",
            ])
            .arg(input)
            .output(),
    )
    .await
    .context("Subtitle probe timed out")??;
    if !output.status.success() {
        bail!(
            "Unable to inspect subtitles: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    parse_subtitle_tracks(&output.stdout)
}

// Keep FFprobe's data contract independent from process execution.
fn parse_subtitle_tracks(bytes: &[u8]) -> Result<Vec<EmbeddedSubtitleTrack>> {
    let data: serde_json::Value = serde_json::from_slice(bytes)?;
    Ok(data["streams"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|stream| {
            let codec = stream["codec_name"].as_str()?.to_string();
            Some(EmbeddedSubtitleTrack {
                index: u32::try_from(stream["index"].as_u64()?).ok()?,
                text_based: text_codec(&codec),
                codec,
                language: stream["tags"]["language"].as_str().unwrap_or("und").into(),
                title: stream["tags"]["title"].as_str().unwrap_or("").into(),
            })
        })
        .collect())
}

pub async fn extract_embedded_subtitle(
    ffmpeg: &str,
    ffprobe: &str,
    input: &Path,
    index: u32,
    output: &Path,
) -> Result<()> {
    extract_track(ffmpeg, ffprobe, input, index, output, None).await
}

async fn extract_track(
    ffmpeg: &str,
    ffprobe: &str,
    input: &Path,
    index: u32,
    output: &Path,
    preview_limit: Option<u32>,
) -> Result<()> {
    let tracks = list_embedded_subtitles(ffprobe, input).await?;
    let track = tracks
        .iter()
        .find(|track| track.index == index)
        .context("Subtitle track not found")?;
    if !track.text_based {
        bail!(
            "Bitmap subtitles ({}) need OCR before conversion to SRT",
            track.codec
        );
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    let mut command = tokio::process::Command::new(ffmpeg);
    command.kill_on_drop(true).stdin(Stdio::null());
    command
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(input)
        .args([
            "-map",
            &format!("0:{index}"),
            "-c:s",
            "srt",
            "-f",
            "srt",
            "-y",
        ]);
    if let Some(limit) = preview_limit {
        command.args(["-frames:s", &limit.to_string()]);
    }
    let result = tokio::time::timeout(
        Duration::from_secs(120),
        command.arg(temporary.path()).output(),
    )
    .await
    .context("Subtitle extraction timed out")??;
    if !result.status.success() {
        bail!(
            "Subtitle extraction failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    if temporary.as_file().metadata()?.len() == 0 {
        bail!("Subtitle track is empty");
    }
    temporary.persist(output).map_err(|error| error.error)?;
    Ok(())
}

/// A bounded, plain-text preview, with automatic cleanup of extracted files.
pub async fn preview_embedded_subtitle(
    ffmpeg: &str,
    ffprobe: &str,
    input: &Path,
    index: u32,
) -> Result<String> {
    use std::io::Read;
    let directory = tempfile::tempdir()?;
    let output = directory.path().join("preview.srt");
    extract_track(ffmpeg, ffprobe, input, index, &output, Some(100)).await?;
    let mut bytes = Vec::new();
    std::fs::File::open(output)?
        .take(64 * 1024)
        .read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy_owned(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probe_metadata_defaults_and_invalid_indices() {
        let tracks = parse_subtitle_tracks(
            r#"{"streams":[
            {"index":2,"codec_name":"ass","tags":{"language":"ita","title":"日本語"}},
            {"index":3,"codec_name":"hdmv_pgs_subtitle"},
            {"index":4294967296,"codec_name":"subrip"},
            {"index":-1,"codec_name":"subrip"},
            {"index":4},
            {"codec_name":"subrip"}
        ]}"#
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].index, 2);
        assert_eq!(tracks[0].language, "ita");
        assert_eq!(tracks[0].title, "日本語");
        assert!(tracks[0].text_based);
        assert_eq!(tracks[1].language, "und");
        assert_eq!(tracks[1].title, "");
        assert!(!tracks[1].text_based);
    }

    #[test]
    fn probe_empty_response_and_malformed_json() {
        assert!(parse_subtitle_tracks(b"{}").unwrap().is_empty());
        assert!(
            parse_subtitle_tracks(br#"{"streams":[]}"#)
                .unwrap()
                .is_empty()
        );
        assert!(parse_subtitle_tracks(b"not json").is_err());
    }

    #[tokio::test]
    async fn embedded_srt_round_trip_and_invalid_track_preserve_existing_output() {
        let directory = tempfile::tempdir().unwrap();
        let srt = directory.path().join("track.srt");
        std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:01,000\nCiao mondo!\n\n").unwrap();
        let mkv = directory.path().join("film.mkv");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=size=32x32:duration=1",
            ])
            .arg("-i")
            .arg(&srt)
            .args([
                "-map",
                "0:v",
                "-map",
                "1:s",
                "-c:v",
                "libx264",
                "-c:s",
                "srt",
                "-metadata:s:s:0",
                "language=ita",
                "-y",
            ])
            .arg(&mkv)
            .status()
            .expect("FFmpeg is required for the embedded subtitle integration test");
        assert!(status.success());
        let tracks = list_embedded_subtitles("ffprobe", &mkv).await.unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].language, "ita");
        assert!(tracks[0].text_based);
        let preview = preview_embedded_subtitle("ffmpeg", "ffprobe", &mkv, tracks[0].index)
            .await
            .unwrap();
        assert!(preview.contains("Ciao mondo!"));
        assert!(!directory.path().join("preview.srt").exists());
        assert!(
            preview_embedded_subtitle("ffmpeg", "ffprobe", &mkv, 99)
                .await
                .is_err()
        );
        let output = directory.path().join("result.srt");
        extract_embedded_subtitle("ffmpeg", "ffprobe", &mkv, tracks[0].index, &output)
            .await
            .unwrap();
        let before = std::fs::read_to_string(&output).unwrap();
        assert!(before.contains("Ciao mondo!"));
        assert!(
            extract_embedded_subtitle("ffmpeg", "ffprobe", &mkv, 99, &output)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(output).unwrap(), before);
    }
    #[test]
    fn bitmap_subtitles_are_explicitly_not_srt_convertible() {
        assert!(!text_codec("hdmv_pgs_subtitle"));
        assert!(!text_codec("dvd_subtitle"));
        assert!(text_codec("ass"));
        assert!(text_codec("subrip"));
    }
}
