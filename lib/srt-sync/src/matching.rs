//! Euristiche GUI-agnostiche per abbinare un file di sottotitoli al suo media
//! o a un sottotitolo "companion" (altra lingua/ruolo) nella stessa cartella.
//!
//! Logica pura (solo `std`): nessun accoppiamento con Tauri o con la GUI, così
//! può essere usata sia dall'app desktop sia da test e tool headless.

use std::path::{Path, PathBuf};

// ─── Public API ────────────────────────────────────────────────────────────

/// Suggerisce, in modo best-effort, il file media più probabile per `srt_path`
/// cercando nella stessa cartella. Restituisce `None` se nessun candidato è
/// abbastanza convincente. Propaga gli errori di lettura della cartella.
pub fn suggest_media_for_srt(srt_path: &Path) -> std::io::Result<Option<PathBuf>> {
    let Some(parent) = srt_path.parent() else {
        return Ok(None);
    };
    let Some(srt_stem) = srt_path.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty())
    else {
        return Ok(None);
    };

    let srt_tokens = normalized_tokens(srt_stem);
    let srt_ep = extract_episode_number(srt_stem);
    let srt_joined = srt_tokens.join(" ");

    let mut candidates: Vec<(PathBuf, i32)> = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_file() || !is_media_path(&path) {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let media_tokens = normalized_tokens(stem);
        if media_tokens.is_empty() {
            continue;
        }

        let mut score: i32 = 0;
        if stem.eq_ignore_ascii_case(srt_stem) {
            score += 100;
        }

        let media_joined = media_tokens.join(" ");
        if !srt_joined.is_empty()
            && !media_joined.is_empty()
            && (media_joined.contains(&srt_joined) || srt_joined.contains(&media_joined))
        {
            score += 40;
        }

        score += token_overlap_score(&srt_tokens, &media_tokens);

        match (srt_ep, extract_episode_number(stem)) {
            (Some(a), Some(b)) if a == b => score += 35,
            (Some(_), Some(_)) => score -= 20,
            _ => {}
        }

        candidates.push((path, score));
    }

    Ok(best_confident(candidates, 12))
}

/// Suggerisce, in modo best-effort, un sottotitolo "companion" (es. lingua o
/// ruolo diverso) nella stessa cartella di `srt_path`. Restituisce `None` se
/// nessun candidato è abbastanza convincente.
pub fn suggest_companion_subtitle_for_srt(srt_path: &Path) -> std::io::Result<Option<PathBuf>> {
    let Some(parent) = srt_path.parent() else {
        return Ok(None);
    };
    let Some(srt_stem) = srt_path.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty())
    else {
        return Ok(None);
    };

    let source_tokens = normalized_tokens(srt_stem);
    let source_joined = source_tokens.join(" ");
    let source_ep = extract_episode_number(srt_stem);
    let source_lang = extract_lang_code(srt_stem);
    let source_role = extract_subtitle_role(srt_stem);
    let source_stem_simplified = simplify_subtitle_stem(srt_stem);

    let mut candidates: Vec<(PathBuf, i32)> = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_file() || !is_subtitle_path(&path) || path == srt_path {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let candidate_tokens = normalized_tokens(stem);
        if candidate_tokens.is_empty() {
            continue;
        }
        let candidate_joined = candidate_tokens.join(" ");

        let mut score: i32 = 0;
        if stem.eq_ignore_ascii_case(srt_stem) {
            score += 90;
        }

        let candidate_stem_simplified = simplify_subtitle_stem(stem);
        if !source_stem_simplified.is_empty()
            && source_stem_simplified.eq_ignore_ascii_case(&candidate_stem_simplified)
        {
            score += 80;
        }
        if !source_joined.is_empty()
            && !candidate_joined.is_empty()
            && (candidate_joined.contains(&source_joined)
                || source_joined.contains(&candidate_joined))
        {
            score += 35;
        }

        score += token_overlap_score(&source_tokens, &candidate_tokens);

        match (source_ep, extract_episode_number(stem)) {
            (Some(a), Some(b)) if a == b => score += 30,
            (Some(_), Some(_)) => score -= 20,
            _ => {}
        }

        match (source_lang.as_deref(), extract_lang_code(stem).as_deref()) {
            (Some(a), Some(b)) if a != b => score += 22,
            (Some(a), Some(b)) if a == b => score -= 12,
            _ => {}
        }

        match (source_role, extract_subtitle_role(stem)) {
            (Some("original"), Some("reference")) | (Some("reference"), Some("original")) => {
                score += 25
            }
            (Some(a), Some(b)) if a == b => score -= 8,
            _ => {}
        }

        candidates.push((path, score));
    }

    Ok(best_confident(candidates, 10))
}

// ─── Scoring / selezione ─────────────────────────────────────────────────────

/// Sceglie il candidato col punteggio massimo se "abbastanza convincente":
/// almeno 45 punti e o stacca il secondo di `lead` punti o il secondo è < 40.
fn best_confident(mut candidates: Vec<(PathBuf, i32)>, lead: i32) -> Option<PathBuf> {
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by_key(|c| std::cmp::Reverse(c.1));

    let (best_path, best_score) = &candidates[0];
    let confident = match candidates.get(1) {
        Some((_, second)) => {
            *best_score >= 45 && (*best_score >= second.saturating_add(lead) || *second < 40)
        }
        None => *best_score >= 45,
    };
    confident.then(|| best_path.clone())
}

// ─── Euristiche pure ─────────────────────────────────────────────────────────

fn is_media_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    matches!(
        ext.as_str(),
        "mp4" | "mkv" | "avi" | "webm" | "mov" | "m4v" | "m2ts" | "mpeg" | "mpg" | "mp3" | "wav"
            | "ogg" | "flac" | "m4a" | "aac" | "wma" | "opus" | "m4b"
    )
}

fn is_subtitle_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    matches!(ext.as_str(), "srt" | "ass" | "ssa" | "vtt")
}

/// Spezza un nome file in token alfanumerici minuscoli, scartando le stopword
/// tipiche dei nomi di release (codec, risoluzioni, tag lingua, ecc.).
fn normalized_tokens(name: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "srt", "sub", "subs", "subtitle", "subtitles", "eng", "en", "ita", "it", "jpn", "ja",
        "spa", "es", "fra", "fr", "ger", "de", "rus", "ru", "v2", "v3", "x264", "x265", "h264",
        "h265", "hevc", "1080p", "720p", "2160p", "480p", "webrip", "bluray", "brrip", "dvdrip",
        "aac",
    ];

    let mut token = String::new();
    let mut tokens = Vec::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            token.push(ch.to_ascii_lowercase());
        } else if !token.is_empty() {
            tokens.push(std::mem::take(&mut token));
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }

    tokens
        .into_iter()
        .filter(|t| !STOPWORDS.contains(&t.as_str()))
        .collect()
}

/// Punteggio di sovrapposizione (0-50) tra due insiemi di token, normalizzato
/// sul più lungo dei due.
fn token_overlap_score(a: &[String], b: &[String]) -> i32 {
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let common = a.iter().filter(|t| b.contains(t)).count() as i32;
    let denom = a.len().max(b.len()) as i32;
    (common * 50) / denom
}

/// Estrae il numero di episodio: prima il pattern `SxxEyy`, poi come fallback il
/// primo token isolato di 1-3 cifre.
fn extract_episode_number(name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    let bytes = lower.as_bytes();

    // Pattern SxxEyy
    for i in 0..bytes.len() {
        if bytes[i] == b's' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'e' {
                let start = j + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
                if end > start {
                    if let Ok(v) = lower[start..end].parse::<u32>() {
                        return Some(v);
                    }
                }
            }
        }
    }

    // Fallback: primo token isolato di 1-3 cifre
    for part in lower.split(|c: char| !c.is_ascii_alphanumeric()) {
        if (1..=3).contains(&part.len())
            && part.chars().all(|c| c.is_ascii_digit())
        {
            if let Ok(v) = part.parse::<u32>() {
                return Some(v);
            }
        }
    }

    None
}

fn extract_lang_code(name: &str) -> Option<String> {
    const LANG_CODES: &[&str] = &[
        "it", "ita", "en", "eng", "ja", "jpn", "es", "spa", "fr", "fra", "de", "ger", "ru", "rus",
        "pt", "por", "zh", "zho", "ko", "kor",
    ];

    let lower = name.to_ascii_lowercase();
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find(|token| LANG_CODES.contains(token))
        .map(str::to_string)
}

fn extract_subtitle_role(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if ["native", "original", "orig", "source"]
        .iter()
        .any(|k| lower.contains(k))
    {
        return Some("original");
    }
    if ["translated", "translation", "tradotto", "traduzione", "reference", "ref"]
        .iter()
        .any(|k| lower.contains(k))
    {
        return Some("reference");
    }
    None
}

/// Riduce un nome a una "spina dorsale" comparabile: token alfanumerici minuscoli
/// senza marcatori di ruolo/lingua, così due sottotitoli companion collassano
/// sullo stesso stem.
fn simplify_subtitle_stem(name: &str) -> String {
    const NOISE: &[&str] = &[
        "native", "original", "orig", "source", "translated", "translation", "tradotto",
        "traduzione", "reference", "ref", "srt", "sub", "subs", "subtitle", "subtitles", "it",
        "ita", "en", "eng", "ja", "jpn", "es", "spa", "fr", "fra", "de", "ger", "ru", "rus",
    ];

    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .filter(|t| !NOISE.contains(&t.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Suggerisce in modo best-effort i file sottotitoli associati a un file media.
/// Restituisce la lista di candidati ordinati per punteggio decrescente.
pub fn suggest_subtitles_for_media(media_path: &Path) -> std::io::Result<Vec<(PathBuf, i32)>> {
    let Some(parent) = media_path.parent() else {
        return Ok(Vec::new());
    };
    let Some(media_stem) = media_path.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty())
    else {
        return Ok(Vec::new());
    };

    let media_tokens = normalized_tokens(media_stem);
    let media_ep = extract_episode_number(media_stem);
    let media_joined = media_tokens.join(" ");

    let mut candidates: Vec<(PathBuf, i32)> = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_file() || !is_subtitle_path(&path) {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let srt_tokens = normalized_tokens(stem);
        if srt_tokens.is_empty() {
            continue;
        }
        let srt_joined = srt_tokens.join(" ");

        let mut score: i32 = 0;
        if stem.eq_ignore_ascii_case(media_stem) {
            score += 100;
        }

        let srt_stem_simplified = simplify_subtitle_stem(stem);
        let media_stem_simplified = simplify_subtitle_stem(media_stem);
        if !srt_stem_simplified.is_empty() && srt_stem_simplified.eq_ignore_ascii_case(&media_stem_simplified) {
            score += 80;
        }

        if !media_joined.is_empty()
            && !srt_joined.is_empty()
            && (srt_joined.contains(&media_joined) || media_joined.contains(&srt_joined))
        {
            score += 40;
        }

        score += token_overlap_score(&media_tokens, &srt_tokens);

        match (media_ep, extract_episode_number(stem)) {
            (Some(a), Some(b)) if a == b => score += 35,
            (Some(_), Some(_)) => score -= 20,
            _ => {}
        }

        candidates.push((path, score));
    }

    candidates.sort_by_key(|c| std::cmp::Reverse(c.1));

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_episode_number() {
        // Pattern SxxEyy ha priorità.
        assert_eq!(extract_episode_number("Show.S01E04.1080p"), Some(4));
        // Fallback: primo token isolato di 1-3 cifre.
        assert_eq!(extract_episode_number("Anime - 07 [1080p]"), Some(7));
        // "parte2" non è un token di sole cifre, quindi nessun numero.
        assert_eq!(extract_episode_number("Detour_parte2"), None);
        assert_eq!(extract_episode_number("NoNumbersHere"), None);
    }

    #[test]
    fn detects_media_and_subtitle_extensions() {
        assert!(is_media_path(Path::new("a/b/movie.mkv")));
        assert!(is_media_path(Path::new("song.MP3")));
        assert!(!is_media_path(Path::new("notes.txt")));
        assert!(is_subtitle_path(Path::new("x.srt")));
        assert!(!is_subtitle_path(Path::new("x.mkv")));
    }

    #[test]
    fn token_overlap_is_normalised_to_longest_set() {
        let a = vec!["detour".to_string(), "parte1".to_string()];
        let b = vec!["detour".to_string(), "parte1".to_string()];
        assert_eq!(token_overlap_score(&a, &b), 50);
        assert_eq!(token_overlap_score(&a, &[]), 0);
    }

    #[test]
    fn lang_and_role_extraction() {
        assert_eq!(extract_lang_code("Movie.ita.srt").as_deref(), Some("ita"));
        assert_eq!(extract_lang_code("Movie.srt"), None);
        assert_eq!(extract_subtitle_role("Movie.original"), Some("original"));
        assert_eq!(extract_subtitle_role("Movie.translated"), Some("reference"));
        assert_eq!(extract_subtitle_role("Movie"), None);
    }

    #[test]
    fn simplify_strips_role_and_lang_markers() {
        assert_eq!(
            simplify_subtitle_stem("Detour-en-original"),
            simplify_subtitle_stem("Detour.ita.translated")
        );
    }

    #[test]
    fn best_confident_requires_clear_lead() {
        // Vincitore netto: 60 vs 20 → confident.
        let c = vec![(PathBuf::from("a"), 60), (PathBuf::from("b"), 20)];
        assert_eq!(best_confident(c, 12), Some(PathBuf::from("a")));
        // Punteggio massimo troppo basso → None.
        let c = vec![(PathBuf::from("a"), 30)];
        assert_eq!(best_confident(c, 12), None);
        // Due candidati vicini e secondo ≥ 40 → non confident.
        let c = vec![(PathBuf::from("a"), 50), (PathBuf::from("b"), 46)];
        assert_eq!(best_confident(c, 12), None);
    }
}
