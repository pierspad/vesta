//! Preparazione di un file media per la riproduzione nel player embedded.
//!
//! Whitelist dei formati nativamente supportati, transcodifica via ffmpeg
//! (Opus con fallback Vorbis se `libopus` non e' disponibile) e caching su
//! disco con invalidazione basata su mtime. GUI-agnostico (niente Tauri,
//! niente runtime async assunto): vedi REFACTOR-PLAN.md §1.2 — prima questa
//! logica viveva solo nel comando Tauri `sync_prepare_media_for_playback`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Estensioni riproducibili nativamente dal player embedded (WebKitGTK /
/// GStreamer) senza passare da ffmpeg.
const NATIVE_PLAYBACK_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "webm", "mp3", "wav", "ogg", "m4a", "aac", "opus", "flac",
];

/// Vero se `path` ha un'estensione riproducibile nativamente, quindi non
/// richiede transcodifica.
pub fn is_natively_playable(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    NATIVE_PLAYBACK_EXTENSIONS.contains(&ext.as_str())
}

/// Hash stabile ma **non crittografico** del path sorgente, usato solo per
/// generare un nome di file deterministico in cache. Il nome storico di
/// questa funzione era `sha1_hash` pur usando `DefaultHasher`: non solo non
/// è SHA-1, ma lo standard library non garantisce nemmeno che l'algoritmo di
/// `DefaultHasher` resti invariato tra versioni del compilatore, il che
/// invaliderebbe silenziosamente l'intera cache ad ogni aggiornamento di
/// rustc. FNV-1a è un algoritmo pubblico e a specifica fissa: stesso input,
/// stesso hash, per sempre.
fn stable_path_hash(input: &str) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let hash = input.bytes().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(FNV_PRIME)
    });

    format!("{hash:016x}")
}

/// Prepara `source` per la riproduzione nel player embedded.
///
/// Se il formato è già nativamente supportato, restituisce `source`
/// invariato. Altrimenti transcodifica l'audio in OGG (Opus, con fallback
/// Vorbis se `libopus` non è disponibile) dentro `cache_dir`, riusando il
/// file già in cache se più recente della sorgente.
///
/// Sincrona (usa `std::process::Command`, non `tokio`): la libreria non
/// assume un runtime async, sta al chiamante decidere come schedularla (es.
/// `tokio::task::spawn_blocking` da un comando Tauri `async`).
pub fn transcode_for_playback(
    source: &Path,
    cache_dir: &Path,
    ffmpeg_cmd: &str,
) -> anyhow::Result<PathBuf> {
    if is_natively_playable(source) {
        return Ok(source.to_path_buf());
    }

    std::fs::create_dir_all(cache_dir)
        .map_err(|e| anyhow::anyhow!("Cannot create cache dir: {e}"))?;

    let hash = stable_path_hash(&source.to_string_lossy());
    let output_path = cache_dir.join(format!("{hash}.ogg"));

    if output_path.exists() {
        let source_modified = std::fs::metadata(source).and_then(|m| m.modified()).ok();
        let cache_modified = std::fs::metadata(&output_path)
            .and_then(|m| m.modified())
            .ok();
        if let (Some(src_time), Some(cache_time)) = (source_modified, cache_modified)
            && cache_time > src_time
        {
            return Ok(output_path);
        }
    }

    eprintln!(
        "[sync] Transcoding '{}' to OGG for browser playback...",
        source.display()
    );

    let output = Command::new(ffmpeg_cmd)
        .args(["-nostdin", "-loglevel", "error", "-y"])
        .arg("-i")
        .arg(source)
        .args([
            "-vn", "-sn", "-dn", "-c:a", "libopus", "-b:a", "128k", "-ar", "48000", "-ac", "2",
        ])
        .arg(&output_path)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run ffmpeg: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        eprintln!("[sync] libopus failed, trying libvorbis fallback: {stderr}");

        let output2 = Command::new(ffmpeg_cmd)
            .args(["-nostdin", "-loglevel", "error", "-y"])
            .arg("-i")
            .arg(source)
            .args([
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
            .arg(&output_path)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run ffmpeg (vorbis): {e}"))?;

        if !output2.status.success() {
            let stderr2 = String::from_utf8_lossy(&output2.stderr).into_owned();
            anyhow::bail!(
                "ffmpeg transcoding failed: {}",
                if stderr2.is_empty() { stderr } else { stderr2 }
            );
        }
    }

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
        // Case-insensitive extension matching.
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
        // Non deve nemmeno toccare cache_dir/ffmpeg per un formato già nativo.
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
}
