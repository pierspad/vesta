//! Comandi Tauri per la sincronizzazione di sottotitoli.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
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

fn map_subtitle_to_info(
    engine: &SyncEngine,
    sub: &srt_parser::Subtitle,
    anchor_ids: &[u32],
) -> Option<SubtitleInfo> {
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
        .filter_map(|sub| map_subtitle_to_info(engine, sub, &anchor_ids))
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
        .filter_map(|sub| map_subtitle_to_info(engine, sub, &anchor_ids))
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

    let anchors = engine.get_anchors();
    let anchor_ids: Vec<u32> = anchors.iter().map(|a| a.subtitle_index).collect();

    map_subtitle_to_info(engine, sub, &anchor_ids)
        .ok_or(format!("Impossibile sincronizzare sottotitolo {}", id))
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
/// Suggerisce in modo best-effort il file media più probabile per un SRT.
/// La logica di scoring vive in [`srt_sync::matching`] (GUI-agnostica, testabile);
/// qui resta solo l'adattamento dei tipi per Tauri.
#[tauri::command]
pub fn sync_suggest_media_for_srt(srt_path: String) -> Result<Option<String>, String> {
    srt_sync::suggest_media_for_srt(Path::new(&srt_path))
        .map(|opt| opt.map(|p| p.to_string_lossy().into_owned()))
        .map_err(|e| e.to_string())
}

/// Suggerisce in modo best-effort un file sottotitoli "companion"
/// nella stessa cartella del file sorgente (es. lingua diversa).
#[tauri::command]
pub fn sync_suggest_companion_subtitle_for_srt(srt_path: String) -> Result<Option<String>, String> {
    srt_sync::suggest_companion_subtitle_for_srt(Path::new(&srt_path))
        .map(|opt| opt.map(|p| p.to_string_lossy().into_owned()))
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSuggestSubtitlesResult {
    pub target: Option<String>,
    pub native: Option<String>,
}

/// Suggerisce in modo best-effort i file sottotitoli target e native per un
/// dato media. Lo scoring dei candidati e l'assegnazione dei ruoli vivono in
/// [`srt_sync::matching`] (GUI-agnostica, testabile); qui resta solo il
/// filtro di confidenza (punteggio >= 45) e l'adattamento dei tipi per Tauri.
#[tauri::command]
pub fn sync_suggest_subtitles_for_media(
    media_path: String,
    default_target_lang: Option<String>,
    default_native_lang: Option<String>,
) -> Result<SyncSuggestSubtitlesResult, String> {
    let candidates = srt_sync::matching::suggest_subtitles_for_media(Path::new(&media_path))
        .map_err(|e| e.to_string())?;

    let candidates: Vec<_> = candidates.into_iter().filter(|c| c.1 >= 45).collect();

    let (target, native) = srt_sync::matching::suggest_target_native_subtitles(
        &candidates,
        default_target_lang.as_deref(),
        default_native_lang.as_deref(),
    );

    Ok(SyncSuggestSubtitlesResult {
        target: target.map(|p| p.to_string_lossy().into_owned()),
        native: native.map(|p| p.to_string_lossy().into_owned()),
    })
}


/// Prepara un file media per la riproduzione nel browser.
/// Per formati non nativamente supportati da WebKitGTK (MKV, AVI, FLV, OGM, VOB),
/// usa ffmpeg per estrarre l'audio in formato OGG (Opus) nella cache dell'app.
/// Restituisce il percorso del file da riprodurre (originale o transcodificato).
///
/// La whitelist dei formati, l'invocazione ffmpeg (con fallback opus->vorbis)
/// e la cache su disco vivono in [`srt_sync::playback`] (GUI-agnostica,
/// sincrona); qui restano solo la risoluzione di `app_cache_dir()`/ffmpeg e lo
/// scheduling su un thread bloccante (la funzione di libreria usa
/// `std::process::Command`, non `tokio`).
#[tauri::command]
pub async fn sync_prepare_media_for_playback(
    app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
    use tauri::Manager;

    let ffmpeg_cmd = super::flashcards::media::resolve_ffmpeg_path(Some(&app)).await;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Cannot get cache dir: {}", e))?
        .join("media_cache");

    let source = Path::new(&path).to_path_buf();
    let output_path = tokio::task::spawn_blocking(move || {
        srt_sync::playback::transcode_for_playback(&source, &cache_dir, &ffmpeg_cmd)
    })
    .await
    .map_err(|e| format!("Transcoding task panicked: {}", e))?
    .map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
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
