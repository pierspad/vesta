use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

static PREPARING: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn run_ffmpeg(command: &mut Command) -> anyhow::Result<Output> {
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

const NATIVE_PLAYBACK_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "webm", "mp3", "wav", "ogg", "m4a", "aac", "opus", "flac",
];

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

    let _guard = PREPARING
        .lock()
        .map_err(|_| anyhow::anyhow!("Audio preparation lock poisoned"))?;
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| anyhow::anyhow!("Cannot create cache dir: {e}"))?;

    let hash = stable_path_hash(&source.to_string_lossy());
    let output_path = cache_dir.join(format!("{hash}.ogg"));

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

    // Publish only complete files; simultaneous preview requests reuse the cache.
    let temporary_path = cache_dir.join(format!("{hash}.partial.ogg"));
    let output = run_ffmpeg(
        Command::new(ffmpeg_cmd)
            .args(["-nostdin", "-loglevel", "error", "-y", "-i"])
            .arg(source)
            .args(["-map", "0:a:0", "-vn", "-sn", "-dn", "-c:a", "copy"])
            .arg(&temporary_path),
    )?;
    if output.status.success() {
        if output_path.exists() {
            std::fs::remove_file(&output_path)?;
        }
        std::fs::rename(&temporary_path, &output_path)?;
        return Ok(output_path);
    }

    eprintln!(
        "[sync] Transcoding '{}' to OGG for browser playback...",
        source.display()
    );

    let output = run_ffmpeg(
        Command::new(ffmpeg_cmd)
            .args(["-nostdin", "-loglevel", "error", "-y"])
            .arg("-i")
            .arg(source)
            .args([
                "-map",
                "0:a:0",
                "-vn",
                "-sn",
                "-dn",
                "-c:a",
                "libopus",
                "-b:a",
                "64k",
                "-compression_level",
                "0",
                "-ar",
                "48000",
                "-ac",
                "1",
            ])
            .arg(&temporary_path),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run ffmpeg: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        eprintln!("[sync] libopus failed, trying libvorbis fallback: {stderr}");

        let output2 = run_ffmpeg(
            Command::new(ffmpeg_cmd)
                .args(["-nostdin", "-loglevel", "error", "-y"])
                .arg("-i")
                .arg(source)
                .args([
                    "-map",
                    "0:a:0",
                    "-vn",
                    "-sn",
                    "-dn",
                    "-c:a",
                    "libvorbis",
                    "-b:a",
                    "128k",
                    "-ar",
                    "44100",
                    "-ac",
                    "2",
                ])
                .arg(&temporary_path),
        )
        .map_err(|e| anyhow::anyhow!("Failed to run ffmpeg (vorbis): {e}"))?;

        if !output2.status.success() {
            let stderr2 = String::from_utf8_lossy(&output2.stderr).into_owned();
            anyhow::bail!(
                "ffmpeg transcoding failed: {}",
                if stderr2.is_empty() { stderr } else { stderr2 }
            );
        }
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
        for ext in [
            "mp4", "m4v", "webm", "mp3", "wav", "ogg", "m4a", "aac", "opus", "flac",
        ] {
            assert!(
                is_natively_playable(Path::new(&format!("clip.{ext}"))),
                "{ext} should be native"
            );
        }

        assert!(is_natively_playable(Path::new("clip.MP3")));
    }

    #[test]
    fn non_native_formats_need_transcoding() {
        for ext in ["mkv", "avi", "mov", "flv", "ogm", "vob", "wma", "m4b"] {
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
        let source = folder.join("clip.mkv");
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
        std::fs::write(cache.join(format!("{hash}.partial.ogg")), "incomplete").unwrap();
        std::fs::write(cache.join(format!("{hash}.ogg")), "").unwrap();
        let output = transcode_for_playback(&source, &cache, fake.to_str().unwrap()).unwrap();
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "OggScomplete");
        assert!(!cache.join(format!("{hash}.partial.ogg")).exists());
        let cached = transcode_for_playback(&source, &cache, "missing-ffmpeg").unwrap();
        assert_eq!(output, cached);
        std::fs::remove_dir_all(&folder).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn unsupported_copy_falls_back_without_publishing_failed_output() {
        use std::os::unix::fs::PermissionsExt;
        let folder = std::env::temp_dir().join(format!(
            "vesta-playback-fallback-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let source = folder.join("clip.mkv");
        std::fs::write(&source, "source").unwrap();
        let fake = folder.join("ffmpeg");
        std::fs::write(&fake, "#!/bin/sh\ncopy=0\nfor arg do [ \"$arg\" = copy ] && copy=1; out=\"$arg\"; done\nif [ $copy = 1 ]; then printf failed > \"$out\"; exit 1; fi\nprintf 'OggSconverted' > \"$out\"\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output =
            transcode_for_playback(&source, &folder.join("cache"), fake.to_str().unwrap()).unwrap();
        assert_eq!(std::fs::read_to_string(output).unwrap(), "OggSconverted");
        std::fs::remove_dir_all(folder).unwrap();
    }
}
