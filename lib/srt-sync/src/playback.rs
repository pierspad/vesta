use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

static PREPARING: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn run_ffmpeg(command: &mut Command) -> anyhow::Result<Output> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_4000);
    }
    let mut child = command
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stderr = child.stderr.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > Duration::from_secs(300) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            anyhow::bail!(
                "Audio preparation timed out after 5 minutes; try extracting an audio file first"
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stderr = reader
        .join()
        .map_err(|_| anyhow::anyhow!("Cannot read ffmpeg output"))??;
    Ok(Output {
        status,
        stdout: Vec::new(),
        stderr,
    })
}

// The sync player only needs audio. Never pass video containers through: a
// hidden video element still decodes their video track and may fail in WebKit.
const NATIVE_PLAYBACK_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "m4a", "aac", "opus", "flac"];

pub fn is_natively_playable(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    NATIVE_PLAYBACK_EXTENSIONS.contains(&ext.as_str())
}

fn stable_path_hash(input: &str) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let hash = input.bytes().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(FNV_PRIME)
    });

    format!("{hash:016x}")
}

pub fn transcode_for_playback(
    source: &Path,
    cache_dir: &Path,
    ffmpeg_cmd: &str,
) -> anyhow::Result<PathBuf> {
    if is_natively_playable(source) {
        return Ok(source.to_path_buf());
    }

    let _guard = PREPARING.lock().unwrap_or_else(|e| e.into_inner());
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| anyhow::anyhow!("Cannot create cache dir: {e}"))?;

    let hash = stable_path_hash(&source.to_string_lossy());
    let output_path = cache_dir.join(format!("{hash}.wav"));

    if std::fs::metadata(&output_path).is_ok_and(|m| m.len() > 0) {
        let source_modified = std::fs::metadata(source).and_then(|m| m.modified()).ok();
        let cache_modified = std::fs::metadata(&output_path)
            .and_then(|m| m.modified())
            .ok();
        if let (Some(src_time), Some(cache_time)) = (source_modified, cache_modified)
            && cache_time >= src_time
        {
            return Ok(output_path);
        }
    }

    // PCM avoids lossy encoding work and needs no optional compressed-audio decoder.
    let temporary_path = cache_dir.join(format!("{hash}.partial.wav"));
    let output = run_ffmpeg(
        Command::new(ffmpeg_cmd)
            .args(["-nostdin", "-loglevel", "error", "-y", "-i"])
            .arg(source)
            .args([
                "-map",
                "0:a:0",
                "-vn",
                "-sn",
                "-dn",
                "-c:a",
                "pcm_s16le",
                "-ar",
                "16000",
                "-ac",
                "1",
            ])
            .arg(&temporary_path),
    );
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            let _ = std::fs::remove_file(&temporary_path);
            return Err(error);
        }
    };
    if !output.status.success() {
        let _ = std::fs::remove_file(&temporary_path);
        anyhow::bail!(
            "ffmpeg audio preparation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if output_path.exists() {
        std::fs::remove_file(&output_path)?;
    }
    std::fs::rename(&temporary_path, &output_path)?;

    eprintln!(
        "[sync] Transcoded '{}' -> '{}'",
        source.display(),
        output_path.display()
    );

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_formats_are_recognised() {
        for ext in ["mp3", "wav", "ogg", "m4a", "aac", "opus", "flac"] {
            assert!(
                is_natively_playable(Path::new(&format!("clip.{ext}"))),
                "{ext} should be native"
            );
        }

        assert!(is_natively_playable(Path::new("clip.MP3")));
    }

    #[test]
    fn non_native_formats_need_transcoding() {
        for ext in [
            "mp4", "m4v", "webm", "mkv", "avi", "mov", "flv", "ogm", "vob", "wma", "m4b",
        ] {
            assert!(
                !is_natively_playable(Path::new(&format!("clip.{ext}"))),
                "{ext} should not be native"
            );
        }
    }

    #[test]
    fn transcode_is_a_no_op_for_native_formats() {
        let source = Path::new("/does/not/exist/clip.mp3");
        let result = transcode_for_playback(
            source,
            Path::new("/does/not/exist/cache"),
            "ffmpeg-not-on-path",
        );
        assert_eq!(result.unwrap(), source.to_path_buf());
    }

    #[test]
    fn stable_path_hash_is_deterministic_and_low_collision() {
        assert_eq!(
            stable_path_hash("/a/b/movie.mkv"),
            stable_path_hash("/a/b/movie.mkv")
        );
        assert_ne!(
            stable_path_hash("/a/b/movie.mkv"),
            stable_path_hash("/a/b/other.mkv")
        );
    }
    #[cfg(unix)]
    #[test]
    fn preparation_reuses_complete_cache_and_ignores_partial_files() {
        use std::os::unix::fs::PermissionsExt;
        let folder =
            std::env::temp_dir().join(format!("vesta-playback-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let source = folder.join("clip.mp4");
        std::fs::write(&source, "source").unwrap();
        let fake = folder.join("ffmpeg");
        std::fs::write(
            &fake,
            "#!/bin/sh\nfor arg do out=\"$arg\"; done\nprintf 'OggScomplete' > \"$out\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
        let cache = folder.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let hash = stable_path_hash(&source.to_string_lossy());
        std::fs::write(cache.join(format!("{hash}.partial.wav")), "incomplete").unwrap();
        std::fs::write(cache.join(format!("{hash}.wav")), "").unwrap();
        let output = transcode_for_playback(&source, &cache, fake.to_str().unwrap()).unwrap();
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "OggScomplete");
        assert!(!cache.join(format!("{hash}.partial.wav")).exists());
        let cached = transcode_for_playback(&source, &cache, "missing-ffmpeg").unwrap();
        assert_eq!(output, cached);
        std::fs::remove_dir_all(&folder).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn failed_preparation_removes_partial_output() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("clip.mp4");
        std::fs::write(&source, "source").unwrap();
        let fake = directory.path().join("ffmpeg");
        std::fs::write(
            &fake,
            "#!/bin/sh\nfor arg do out=\"$arg\"; done\nprintf failed > \"$out\"\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
        let cache = directory.path().join("cache");
        assert!(transcode_for_playback(&source, &cache, fake.to_str().unwrap()).is_err());
        assert_eq!(std::fs::read_dir(cache).unwrap().count(), 0);
    }

    #[test]
    fn real_preview_is_audio_only_pcm_and_reuses_cache() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("clip.mp4");
        let status = Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=size=16x16:duration=1",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
                "-shortest",
            ])
            .arg(&source)
            .status()
            .unwrap();
        assert!(status.success());
        let cache = directory.path().join("cache");
        let preview = transcode_for_playback(&source, &cache, "ffmpeg").unwrap();
        let probe = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_name,codec_type,sample_rate,channels",
                "-of",
                "csv=p=0",
            ])
            .arg(&preview)
            .output()
            .unwrap();
        assert!(probe.status.success());
        let streams = String::from_utf8(probe.stdout).unwrap();
        assert_eq!(streams.lines().count(), 1);
        assert!(streams.contains("pcm_s16le,audio,16000,1"), "{streams}");
        assert_eq!(
            transcode_for_playback(&source, &cache, "missing-ffmpeg").unwrap(),
            preview
        );
    }
}
