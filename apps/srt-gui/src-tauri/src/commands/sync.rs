//! Comandi Tauri per la sincronizzazione di sottotitoli.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

use srt_sync::{SamplerStrategy, SyncEngine};

use crate::state::AppSyncState;

/// Informazioni su un sottotitolo per il frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleInfo {
    pub id: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub synced_start_ms: u64,
    pub synced_end_ms: u64,
    pub offset_ms: i64,
    pub is_anchor: bool,
}

/// Stato della sincronizzazione per il frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub is_loaded: bool,
    pub srt_path: Option<String>,
    pub video_path: Option<String>,
    pub total_subtitles: usize,
    pub anchor_count: usize,
    pub checked_count: usize,
    pub completion_percentage: f64,
    pub average_offset_ms: f64,
    pub suggested_next_id: Option<u32>,
}

/// Info ancora serializzata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorInfo {
    pub subtitle_id: u32,
    pub original_time_ms: i64,
    pub corrected_time_ms: i64,
    pub offset_ms: i64,
    pub is_manual: bool,
}

/// Carica un file SRT per la sincronizzazione
#[tauri::command]
pub fn sync_load_srt(
    state: State<'_, AppSyncState>,
    path: String,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = SyncEngine::new(&path)
        .map_err(|e| format!("Errore caricamento SRT: {}", e))?;

    let status = get_status_from_engine(&engine);
    sync_state.engine = Some(engine);

    Ok(status)
}

/// Imposta il percorso del video
#[tauri::command]
pub fn sync_set_video(
    state: State<'_, AppSyncState>,
    path: String,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_mut()
        .ok_or("Nessun file SRT caricato")?;

    engine.set_video_path(&path);

    Ok(get_status_from_engine(engine))
}

/// Ottiene lo stato corrente della sincronizzazione
#[tauri::command]
pub fn sync_get_status(
    state: State<'_, AppSyncState>,
) -> Result<SyncStatus, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    match &sync_state.engine {
        Some(engine) => Ok(get_status_from_engine(engine)),
        None => Ok(SyncStatus {
            is_loaded: false,
            srt_path: None,
            video_path: None,
            total_subtitles: 0,
            anchor_count: 0,
            checked_count: 0,
            completion_percentage: 0.0,
            average_offset_ms: 0.0,
            suggested_next_id: None,
        }),
    }
}

/// Ottiene tutti i sottotitoli con info di sync
#[tauri::command]
pub fn sync_get_subtitles(
    state: State<'_, AppSyncState>,
) -> Result<Vec<SubtitleInfo>, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    let anchors = engine.get_anchors();
    let anchor_ids: Vec<u32> = anchors.iter().map(|a| a.subtitle_index).collect();

    let subtitles: Vec<SubtitleInfo> = engine.get_all_subtitles()
        .iter()
        .filter_map(|sub| {
            let synced = engine.get_synced_subtitle(sub.id)?;
            let offset = engine.get_current_offset(sub.id).unwrap_or(0);
            
            Some(SubtitleInfo {
                id: sub.id,
                start_ms: sub.start.milliseconds,
                end_ms: sub.end.milliseconds,
                text: sub.text.clone(),
                synced_start_ms: synced.start.milliseconds,
                synced_end_ms: synced.end.milliseconds,
                offset_ms: offset,
                is_anchor: anchor_ids.contains(&sub.id),
            })
        })
        .collect();

    Ok(subtitles)
}

/// Ottiene sottotitoli paginati (per lazy loading)
#[tauri::command]
pub fn sync_get_subtitles_range(
    state: State<'_, AppSyncState>,
    start_id: u32,
    count: usize,
) -> Result<Vec<SubtitleInfo>, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    let anchors = engine.get_anchors();
    let anchor_ids: Vec<u32> = anchors.iter().map(|a| a.subtitle_index).collect();

    let all_subs = engine.get_all_subtitles();
    
    // Find starting index based on subtitle ID
    let start_idx = all_subs.iter().position(|s| s.id >= start_id).unwrap_or(0);
    
    let subtitles: Vec<SubtitleInfo> = all_subs
        .iter()
        .skip(start_idx)
        .take(count)
        .filter_map(|sub| {
            let synced = engine.get_synced_subtitle(sub.id)?;
            let offset = engine.get_current_offset(sub.id).unwrap_or(0);
            
            Some(SubtitleInfo {
                id: sub.id,
                start_ms: sub.start.milliseconds,
                end_ms: sub.end.milliseconds,
                text: sub.text.clone(),
                synced_start_ms: synced.start.milliseconds,
                synced_end_ms: synced.end.milliseconds,
                offset_ms: offset,
                is_anchor: anchor_ids.contains(&sub.id),
            })
        })
        .collect();

    Ok(subtitles)
}

/// Ottiene un sottotitolo specifico
#[tauri::command]
pub fn sync_get_subtitle(
    state: State<'_, AppSyncState>,
    id: u32,
) -> Result<SubtitleInfo, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    let sub = engine.get_subtitle(id)
        .ok_or(format!("Sottotitolo {} non trovato", id))?;

    let synced = engine.get_synced_subtitle(id)
        .ok_or(format!("Impossibile sincronizzare sottotitolo {}", id))?;

    let offset = engine.get_current_offset(id).unwrap_or(0);
    let anchors = engine.get_anchors();
    let is_anchor = anchors.iter().any(|a| a.subtitle_index == id);

    Ok(SubtitleInfo {
        id: sub.id,
        start_ms: sub.start.milliseconds,
        end_ms: sub.end.milliseconds,
        text: sub.text.clone(),
        synced_start_ms: synced.start.milliseconds,
        synced_end_ms: synced.end.milliseconds,
        offset_ms: offset,
        is_anchor,
    })
}

/// Trova il sottotitolo al tempo video specificato
#[tauri::command]
pub fn sync_find_subtitle_at_time(
    state: State<'_, AppSyncState>,
    time_ms: u64,
) -> Result<Option<u32>, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    Ok(engine.find_subtitle_at_time(time_ms))
}

/// Trova il sottotitolo più vicino al tempo video
#[tauri::command]
pub fn sync_find_nearest_subtitle(
    state: State<'_, AppSyncState>,
    time_ms: u64,
) -> Result<Option<u32>, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    Ok(engine.find_nearest_subtitle(time_ms))
}

/// Aggiunge un'ancora di sincronizzazione
#[tauri::command]
pub fn sync_add_anchor(
    state: State<'_, AppSyncState>,
    subtitle_id: u32,
    corrected_time_ms: i64,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_mut()
        .ok_or("Nessun file SRT caricato")?;

    engine.add_anchor(subtitle_id, corrected_time_ms, true)
        .map_err(|e| format!("Errore aggiunta ancora: {}", e))?;

    Ok(get_status_from_engine(engine))
}

/// Rimuove un'ancora di sincronizzazione
#[tauri::command]
pub fn sync_remove_anchor(
    state: State<'_, AppSyncState>,
    subtitle_id: u32,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_mut()
        .ok_or("Nessun file SRT caricato")?;

    engine.remove_anchor(subtitle_id);

    Ok(get_status_from_engine(engine))
}

/// Ottiene tutte le ancore
#[tauri::command]
pub fn sync_get_anchors(
    state: State<'_, AppSyncState>,
) -> Result<Vec<AnchorInfo>, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    let anchors: Vec<AnchorInfo> = engine.get_anchors()
        .iter()
        .map(|a| AnchorInfo {
            subtitle_id: a.subtitle_index,
            original_time_ms: a.original_time_ms,
            corrected_time_ms: a.corrected_time_ms,
            offset_ms: a.offset(),
            is_manual: a.is_manual,
        })
        .collect();

    Ok(anchors)
}

/// Suggerisce il prossimo sottotitolo da controllare
#[tauri::command]
pub fn sync_suggest_next(
    state: State<'_, AppSyncState>,
) -> Result<Option<u32>, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    Ok(engine.suggest_next_index())
}

/// Imposta la strategia di campionamento
#[tauri::command]
pub fn sync_set_strategy(
    state: State<'_, AppSyncState>,
    strategy: String,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_mut()
        .ok_or("Nessun file SRT caricato")?;

    let strat = match strategy.to_lowercase().as_str() {
        "binary" | "binarysearch" => SamplerStrategy::BinarySearch,
        "uncertainty" | "maxuncertainty" => SamplerStrategy::MaxUncertainty,
        "uniform" | "uniformtime" => SamplerStrategy::UniformTime,
        "sequential" => SamplerStrategy::Sequential,
        _ => return Err(format!("Strategia non supportata: {}", strategy)),
    };

    engine.set_sampling_strategy(strat);

    Ok(get_status_from_engine(engine))
}

/// Salva il file SRT sincronizzato
#[tauri::command]
pub fn sync_save_file(
    state: State<'_, AppSyncState>,
    output_path: String,
) -> Result<String, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    engine.save_synced_file(&output_path)
        .map_err(|e| format!("Errore salvataggio: {}", e))?;

    Ok(output_path)
}

/// Salva la sessione di sincronizzazione
#[tauri::command]
pub fn sync_save_session(
    state: State<'_, AppSyncState>,
    session_path: String,
) -> Result<String, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref()
        .ok_or("Nessun file SRT caricato")?;

    engine.save_session(&session_path)
        .map_err(|e| format!("Errore salvataggio sessione: {}", e))?;

    Ok(session_path)
}

/// Carica una sessione salvata
#[tauri::command]
pub fn sync_load_session(
    state: State<'_, AppSyncState>,
    session_path: String,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = SyncEngine::load_session(&session_path)
        .map_err(|e| format!("Errore caricamento sessione: {}", e))?;

    let status = get_status_from_engine(&engine);
    sync_state.engine = Some(engine);

    Ok(status)
}

/// Resetta la sincronizzazione (rimuove completamente engine, SRT e video)
#[tauri::command]
pub fn sync_reset(
    state: State<'_, AppSyncState>,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    // Rimuovi completamente l'engine per liberare SRT e video
    sync_state.engine = None;

    Ok(SyncStatus {
        is_loaded: false,
        srt_path: None,
        video_path: None,
        total_subtitles: 0,
        anchor_count: 0,
        checked_count: 0,
        completion_percentage: 0.0,
        average_offset_ms: 0.0,
        suggested_next_id: None,
    })
}

/// Suggerisce in modo best-effort un file media nella stessa cartella del file SRT.
/// Restituisce `None` quando il matching non e' sufficientemente affidabile.
#[tauri::command]
pub fn sync_suggest_media_for_srt(srt_path: String) -> Result<Option<String>, String> {
    let srt = Path::new(&srt_path);
    let parent = match srt.parent() {
        Some(p) => p,
        None => return Ok(None),
    };
    let srt_stem = match srt.file_stem().and_then(|s| s.to_str()) {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(None),
    };

    let mut candidates: Vec<(PathBuf, i32)> = Vec::new();
    let srt_tokens = normalized_tokens(srt_stem);
    let srt_ep = extract_episode_number(srt_stem);

    let entries = fs::read_dir(parent).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };
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

        let srt_joined = srt_tokens.join(" ");
        let media_joined = media_tokens.join(" ");
        if !srt_joined.is_empty() && !media_joined.is_empty() {
            if media_joined.contains(&srt_joined) || srt_joined.contains(&media_joined) {
                score += 40;
            }
        }

        let overlap = token_overlap_score(&srt_tokens, &media_tokens);
        score += overlap;

        let media_ep = extract_episode_number(stem);
        match (srt_ep, media_ep) {
            (Some(a), Some(b)) if a == b => score += 35,
            (Some(_), Some(_)) => score -= 20,
            _ => {}
        }

        candidates.push((path, score));
    }

    if candidates.is_empty() {
        return Ok(None);
    }

    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    let (best_path, best_score) = &candidates[0];
    let confident = if let Some((_, second_score)) = candidates.get(1) {
        *best_score >= 45
            && (*best_score >= second_score.saturating_add(12) || *second_score < 40)
    } else {
        *best_score >= 45
    };

    if confident {
        Ok(Some(best_path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

/// Suggerisce in modo best-effort un file sottotitoli "companion"
/// nella stessa cartella del file sorgente (es. lingua diversa).
#[tauri::command]
pub fn sync_suggest_companion_subtitle_for_srt(srt_path: String) -> Result<Option<String>, String> {
    let srt = Path::new(&srt_path);
    let parent = match srt.parent() {
        Some(p) => p,
        None => return Ok(None),
    };
    let srt_stem = match srt.file_stem().and_then(|s| s.to_str()) {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(None),
    };

    let source_tokens = normalized_tokens(srt_stem);
    let source_joined = source_tokens.join(" ");
    let source_ep = extract_episode_number(srt_stem);
    let source_lang = extract_lang_code(srt_stem);
    let source_role = extract_subtitle_role(srt_stem);
    let source_stem_simplified = simplify_subtitle_stem(srt_stem);

    let mut candidates: Vec<(PathBuf, i32)> = Vec::new();
    let entries = fs::read_dir(parent).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() || !is_subtitle_path(&path) {
            continue;
        }
        if path == srt {
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
        let candidate_ep = extract_episode_number(stem);
        let candidate_lang = extract_lang_code(stem);
        let candidate_role = extract_subtitle_role(stem);
        let candidate_stem_simplified = simplify_subtitle_stem(stem);

        let mut score: i32 = 0;

        if stem.eq_ignore_ascii_case(srt_stem) {
            score += 90;
        }
        if !source_stem_simplified.is_empty()
            && source_stem_simplified.eq_ignore_ascii_case(&candidate_stem_simplified)
        {
            score += 80;
        }
        if !source_joined.is_empty() && !candidate_joined.is_empty() {
            if candidate_joined.contains(&source_joined) || source_joined.contains(&candidate_joined) {
                score += 35;
            }
        }

        score += token_overlap_score(&source_tokens, &candidate_tokens);

        match (source_ep, candidate_ep) {
            (Some(a), Some(b)) if a == b => score += 30,
            (Some(_), Some(_)) => score -= 20,
            _ => {}
        }

        match (source_lang.as_deref(), candidate_lang.as_deref()) {
            (Some(a), Some(b)) if a != b => score += 22,
            (Some(a), Some(b)) if a == b => score -= 12,
            _ => {}
        }

        match (source_role.as_deref(), candidate_role.as_deref()) {
            (Some("original"), Some("reference")) | (Some("reference"), Some("original")) => {
                score += 25
            }
            (Some(a), Some(b)) if a == b => score -= 8,
            _ => {}
        }

        candidates.push((path, score));
    }

    if candidates.is_empty() {
        return Ok(None);
    }

    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    let (best_path, best_score) = &candidates[0];
    let confident = if let Some((_, second_score)) = candidates.get(1) {
        *best_score >= 45
            && (*best_score >= second_score.saturating_add(10) || *second_score < 40)
    } else {
        *best_score >= 45
    };

    if confident {
        Ok(Some(best_path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

fn is_media_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    matches!(
        ext.as_str(),
        "mp4"
            | "mkv"
            | "avi"
            | "webm"
            | "mov"
            | "m4v"
            | "m2ts"
            | "mpeg"
            | "mpg"
            | "mp3"
            | "wav"
            | "ogg"
            | "flac"
            | "m4a"
            | "aac"
            | "wma"
            | "opus"
            | "m4b"
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

fn normalized_tokens(name: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "srt", "sub", "subs", "subtitle", "subtitles", "eng", "en", "ita", "it", "jpn", "ja",
        "spa", "es", "fra", "fr", "ger", "de", "rus", "ru", "v2", "v3", "x264", "x265", "h264",
        "h265", "hevc", "1080p", "720p", "2160p", "480p", "webrip", "bluray", "brrip", "dvdrip", "aac",
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

fn token_overlap_score(a: &[String], b: &[String]) -> i32 {
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let common = a.iter().filter(|t| b.contains(t)).count() as i32;
    let denom = a.len().max(b.len()) as i32;
    (common * 50) / denom
}

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

    // Fallback: first isolated 1-3 digit token
    for part in lower.split(|c: char| !c.is_ascii_alphanumeric()) {
        if (1..=3).contains(&part.len()) && part.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(v) = part.parse::<u32>() {
                return Some(v);
            }
        }
    }

    None
}

fn extract_lang_code(name: &str) -> Option<String> {
    const LANG_CODES: &[&str] = &[
        "it", "ita", "en", "eng", "ja", "jpn", "es", "spa", "fr", "fra", "de", "ger", "ru",
        "rus", "pt", "por", "zh", "zho", "ko", "kor",
    ];

    let lower = name.to_ascii_lowercase();
    for token in lower.split(|c: char| !c.is_ascii_alphanumeric()) {
        if LANG_CODES.contains(&token) {
            return Some(token.to_string());
        }
    }
    None
}

fn extract_subtitle_role(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    let has_original = ["native", "original", "orig", "source"]
        .iter()
        .any(|k| lower.contains(k));
    if has_original {
        return Some("original");
    }

    let has_reference = ["translated", "translation", "tradotto", "traduzione", "reference", "ref"]
        .iter()
        .any(|k| lower.contains(k));
    if has_reference {
        return Some("reference");
    }

    None
}

fn simplify_subtitle_stem(name: &str) -> String {
    let tokens: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .filter(|t| {
            !matches!(
                t.as_str(),
                "native"
                    | "original"
                    | "orig"
                    | "source"
                    | "translated"
                    | "translation"
                    | "tradotto"
                    | "traduzione"
                    | "reference"
                    | "ref"
                    | "srt"
                    | "sub"
                    | "subs"
                    | "subtitle"
                    | "subtitles"
                    | "it"
                    | "ita"
                    | "en"
                    | "eng"
                    | "ja"
                    | "jpn"
                    | "es"
                    | "spa"
                    | "fr"
                    | "fra"
                    | "de"
                    | "ger"
                    | "ru"
                    | "rus"
            )
        })
        .collect();

    tokens.join(" ")
}

/// Helper per estrarre lo stato dall'engine
fn get_status_from_engine(engine: &SyncEngine) -> SyncStatus {
    SyncStatus {
        is_loaded: true,
        srt_path: Some(engine.export_state().srt_path),
        video_path: engine.get_video_path().map(String::from),
        total_subtitles: engine.total_subtitles(),
        anchor_count: engine.anchor_count(),
        checked_count: engine.checked_count(),
        completion_percentage: engine.completion_percentage(),
        average_offset_ms: engine.get_average_offset(),
        suggested_next_id: engine.suggest_next_index(),
    }
}
