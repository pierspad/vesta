use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

static PREPARING: std::sync::Mutex<()> = std::sync::Mutex::new(());

// Drain the complete pipe, retaining only the diagnostic tail.
const STDERR_LIMIT: usize = 64 * 1024;

fn read_stderr_tail(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut tail = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(tail);
        }
        let excess = (tail.len() + count).saturating_sub(STDERR_LIMIT);
        tail.drain(..excess);
        tail.extend_from_slice(&buffer[..count]);
    }
}

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
    let stderr = child.stderr.take().unwrap();
    let reader = std::thread::spawn(move || read_stderr_tail(stderr));
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

fn has_fresh_cache(source: &Path, output: &Path) -> bool {
    let Ok(cached) = std::fs::metadata(output) else {
        return false;
    };
    if !cached.is_file() || cached.len() == 0 {
        return false;
    }
    let source_modified = std::fs::metadata(source).and_then(|m| m.modified()).ok();
    let cache_modified = cached.modified().ok();
    matches!((source_modified, cache_modified), (Some(source), Some(cache)) if cache >= source)
}

pub fn transcode_for_playback(
    source: &Path,
    cache_dir: &Path,
    ffmpeg_cmd: &str,
) -> anyhow::Result<PathBuf> {
    if is_natively_playable(source) {
        return Ok(source.to_path_buf());
    }

    let hash = stable_path_hash(&source.to_string_lossy());
    let output_path = cache_dir.join(format!("{hash}.wav"));
    // Published cache entries are complete: readers need not wait for unrelated conversions.
    if has_fresh_cache(source, &output_path) {
        return Ok(output_path);
    }

    let _guard = PREPARING.lock().unwrap_or_else(|e| e.into_inner());
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| anyhow::anyhow!("Cannot create cache dir: {e}"))?;
    // Another caller may have filled this cache while we waited for the lock.
    if has_fresh_cache(source, &output_path) {
        return Ok(output_path);
    }

    // PCM avoids lossy encoding work and needs no optional compressed-audio decoder.
    let temporary_path = tempfile::Builder::new()
        .prefix(".vesta-preview-")
        .suffix(".wav")
        .tempfile_in(cache_dir)?
        .into_temp_path();
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
    )?;
    if !output.status.success() {
        anyhow::bail!(
            "ffmpeg audio preparation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    anyhow::ensure!(
        std::fs::metadata(&temporary_path)?.len() > 0,
        "FFmpeg audio preparation produced an empty preview"
    );
    // Replace in one filesystem operation; failed publication preserves the old cache.
    temporary_path.persist(&output_path).map_err(|e| e.error)?;

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
    fn stderr_is_fully_drained_but_only_the_tail_is_retained() {
        let bytes: Vec<u8> = (0..STDERR_LIMIT * 3 + 123)
            .map(|i| (i % 251) as u8)
            .collect();
        let mut reader = std::io::Cursor::new(&bytes);
        let tail = read_stderr_tail(&mut reader).unwrap();
        assert_eq!(reader.position() as usize, bytes.len());
        assert_eq!(tail, bytes[bytes.len() - STDERR_LIMIT..]);
        assert_eq!(
            read_stderr_tail(&b"short error"[..]).unwrap(),
            b"short error"
        );
    }

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
        assert_eq!(
            std::fs::read_to_string(cache.join(format!("{hash}.partial.wav"))).unwrap(),
            "incomplete"
        );
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 2);
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

    #[cfg(unix)]
    #[test]
    fn failed_and_empty_refreshes_preserve_old_cache_and_clean_temporary_files() {
        use std::os::unix::fs::PermissionsExt;
        for script in ["exit 1", "for arg do out=\"$arg\"; done; : > \"$out\""] {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("clip.mp4");
            std::fs::write(&source, "source").unwrap();
            let cache = directory.path().join("cache");
            std::fs::create_dir(&cache).unwrap();
            let output = cache.join(format!(
                "{}.wav",
                stable_path_hash(&source.to_string_lossy())
            ));
            std::fs::write(&output, "previous complete preview").unwrap();
            let old = std::time::SystemTime::now() - Duration::from_secs(60);
            std::fs::File::open(&output)
                .unwrap()
                .set_times(std::fs::FileTimes::new().set_modified(old))
                .unwrap();
            let fake = directory.path().join("ffmpeg");
            std::fs::write(&fake, format!("#!/bin/sh\n{script}\n")).unwrap();
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
            assert!(transcode_for_playback(&source, &cache, fake.to_str().unwrap()).is_err());
            assert_eq!(
                std::fs::read_to_string(output).unwrap(),
                "previous complete preview"
            );
            assert_eq!(std::fs::read_dir(cache).unwrap().count(), 1);
        }
    }

    #[cfg(unix)]
    #[test]
    fn failed_publication_cleans_temporary_file_without_removing_destination() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("clip.mp4");
        std::fs::write(&source, "source").unwrap();
        let cache = directory.path().join("cache");
        let output = cache.join(format!(
            "{}.wav",
            stable_path_hash(&source.to_string_lossy())
        ));
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(output.join("keep"), "untouched").unwrap();
        let fake = directory.path().join("ffmpeg");
        std::fs::write(
            &fake,
            "#!/bin/sh\nfor arg do out=\"$arg\"; done\nprintf complete > \"$out\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(transcode_for_playback(&source, &cache, fake.to_str().unwrap()).is_err());
        assert_eq!(
            std::fs::read_to_string(output.join("keep")).unwrap(),
            "untouched"
        );
        assert_eq!(std::fs::read_dir(cache).unwrap().count(), 1);
    }

    #[test]
    fn fresh_cache_does_not_wait_for_an_unrelated_conversion() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("clip.mp4");
        std::fs::write(&source, "source").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(60);
        std::fs::File::open(&source)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(old))
            .unwrap();
        let cache = directory.path().join("cache");
        std::fs::create_dir(&cache).unwrap();
        let output = cache.join(format!(
            "{}.wav",
            stable_path_hash(&source.to_string_lossy())
        ));
        std::fs::write(&output, "cached preview").unwrap();
        let guard = PREPARING.lock().unwrap_or_else(|e| e.into_inner());
        let (send, receive) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            send.send(transcode_for_playback(&source, &cache, "missing-ffmpeg"))
                .unwrap();
        });
        let result = receive.recv_timeout(Duration::from_secs(2));
        drop(guard);
        worker.join().unwrap();
        assert_eq!(
            result
                .expect("cache hit waited for conversion lock")
                .unwrap(),
            output
        );
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
