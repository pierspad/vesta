use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;

use vesta_sync::SyncEngine;

use crate::media_stream::MediaAllowlist;
use crate::state::AppSyncState;

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
    pub can_undo: bool,
    pub can_redo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorInfo {
    pub subtitle_id: u32,
    pub original_time_ms: i64,
    pub corrected_time_ms: i64,
    pub offset_ms: i64,
    pub is_manual: bool,
}

#[tauri::command]
pub fn sync_load_srt(state: State<'_, AppSyncState>, path: String) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = SyncEngine::new(&path).map_err(|e| format!("Failed to load SRT: {}", e))?;

    let status = get_status_from_engine(&engine);
    sync_state.replace_engine(Some(engine))?;

    Ok(status)
}

/// Sets the video file path on the active synchronization engine.
#[tauri::command]
pub fn sync_set_video(state: State<'_, AppSyncState>, path: String) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    sync_state.set_video_path(&path)?;

    let engine = sync_state.engine.as_mut().ok_or("No SRT file loaded")?;

    Ok(get_status_from_engine(engine))
}

#[tauri::command]
pub fn sync_get_status(state: State<'_, AppSyncState>) -> Result<SyncStatus, String> {
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
            can_undo: false,
            can_redo: false,
        }),
    }
}

use std::collections::HashSet;

fn map_subtitle_to_info(
    engine: &SyncEngine,
    sub: &vesta_parser::Subtitle,
    anchor_ids: &HashSet<u32>,
) -> Option<SubtitleInfo> {
    let (synced_start_ms, synced_end_ms, offset_ms) = engine.get_synced_info_for(sub);

    Some(SubtitleInfo {
        id: sub.id,
        start_ms: sub.start.milliseconds,
        end_ms: sub.end.milliseconds,
        text: sub.text.clone(),
        synced_start_ms,
        synced_end_ms,
        offset_ms,
        is_anchor: anchor_ids.contains(&sub.id),
    })
}

#[tauri::command]
pub fn sync_get_subtitles_range(
    state: State<'_, AppSyncState>,
    start_id: u32,
    count: usize,
) -> Result<Vec<SubtitleInfo>, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref().ok_or("No SRT file loaded")?;

    let anchors = engine.get_anchors();
    let anchor_ids: HashSet<u32> = anchors.iter().map(|a| a.subtitle_index).collect();

    let all_subs = engine.get_all_subtitles();

    // Binary search for starting index based on subtitle ID in O(log N)
    let start_idx = all_subs.partition_point(|s| s.id < start_id);

    let subtitles: Vec<SubtitleInfo> = all_subs
        .iter()
        .skip(start_idx)
        .take(count)
        .filter_map(|sub| map_subtitle_to_info(engine, sub, &anchor_ids))
        .collect();

    Ok(subtitles)
}

/// Returns a specific subtitle with its synchronized timing information.
#[tauri::command]
pub fn sync_get_subtitle(state: State<'_, AppSyncState>, id: u32) -> Result<SubtitleInfo, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref().ok_or("No SRT file loaded")?;

    let sub = engine
        .get_subtitle(id)
        .ok_or(format!("Subtitle {} not found", id))?;

    let anchors = engine.get_anchors();
    let anchor_ids: HashSet<u32> = anchors.iter().map(|a| a.subtitle_index).collect();

    map_subtitle_to_info(engine, sub, &anchor_ids)
        .ok_or(format!("Failed to synchronize subtitle {}", id))
}

#[tauri::command]
pub fn sync_add_anchor(
    state: State<'_, AppSyncState>,
    subtitle_id: u32,
    corrected_time_ms: i64,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    sync_state.operation.ensure_idle()?;

    let engine = sync_state.engine.as_mut().ok_or("No SRT file loaded")?;

    engine
        .add_anchor(subtitle_id, corrected_time_ms, true)
        .map_err(|e| format!("Failed to add anchor: {}", e))?;

    Ok(get_status_from_engine(engine))
}

/// Removes a synchronization anchor.
#[tauri::command]
pub fn sync_remove_anchor(
    state: State<'_, AppSyncState>,
    subtitle_id: u32,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    sync_state.operation.ensure_idle()?;

    let engine = sync_state.engine.as_mut().ok_or("No SRT file loaded")?;

    engine.remove_anchor(subtitle_id);

    Ok(get_status_from_engine(engine))
}

#[tauri::command]
pub fn sync_undo(state: State<'_, AppSyncState>) -> Result<SyncStatus, String> {
    change_history(&state, false)
}

#[tauri::command]
pub fn sync_redo(state: State<'_, AppSyncState>) -> Result<SyncStatus, String> {
    change_history(&state, true)
}

fn change_history(state: &AppSyncState, redo: bool) -> Result<SyncStatus, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.operation.ensure_idle()?;
    let engine = state.engine.as_mut().ok_or("No SRT file loaded")?;
    if redo {
        engine.redo();
    } else {
        engine.undo();
    }
    Ok(get_status_from_engine(engine))
}

/// Returns all synchronization anchors.
#[tauri::command]
pub fn sync_get_anchors(state: State<'_, AppSyncState>) -> Result<Vec<AnchorInfo>, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref().ok_or("No SRT file loaded")?;

    let anchors: Vec<AnchorInfo> = engine
        .get_anchors()
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

#[tauri::command]
pub fn sync_save_file(
    state: State<'_, AppSyncState>,
    output_path: String,
) -> Result<String, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref().ok_or("No SRT file loaded")?;

    engine
        .save_synced_file(&output_path)
        .map_err(|e| format!("Failed to save synchronized SRT: {}", e))?;

    Ok(output_path)
}

/// Saves the synchronization session state to disk.
#[tauri::command]
pub fn sync_save_session(
    state: State<'_, AppSyncState>,
    session_path: String,
) -> Result<String, String> {
    let sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = sync_state.engine.as_ref().ok_or("No SRT file loaded")?;

    engine
        .save_session(&session_path)
        .map_err(|e| format!("Failed to save session: {}", e))?;

    Ok(session_path)
}

#[tauri::command]
pub fn sync_load_session(
    state: State<'_, AppSyncState>,
    session_path: String,
) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    let engine = SyncEngine::load_session(&session_path)
        .map_err(|e| format!("Failed to load session: {}", e))?;

    let status = get_status_from_engine(&engine);
    sync_state.replace_engine(Some(engine))?;

    Ok(status)
}

/// Resets synchronization state (unloads engine, SRT, and video).
#[tauri::command]
pub fn sync_reset(state: State<'_, AppSyncState>) -> Result<SyncStatus, String> {
    let mut sync_state = state.lock().map_err(|e| e.to_string())?;

    sync_state.replace_engine(None)?;

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
        can_undo: false,
        can_redo: false,
    })
}

#[tauri::command]
pub fn sync_suggest_media_for_srt(srt_path: String) -> Result<Option<String>, String> {
    vesta_sync::suggest_media_for_srt(Path::new(&srt_path))
        .map(|opt| opt.map(|p| p.to_string_lossy().into_owned()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_suggest_companion_subtitle_for_srt(srt_path: String) -> Result<Option<String>, String> {
    vesta_sync::suggest_companion_subtitle_for_srt(Path::new(&srt_path))
        .map(|opt| opt.map(|p| p.to_string_lossy().into_owned()))
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSuggestSubtitlesResult {
    pub target: Option<String>,
    pub native: Option<String>,
}

#[tauri::command]
pub fn sync_suggest_subtitles_for_media(
    media_path: String,
    default_target_lang: Option<String>,
    default_native_lang: Option<String>,
) -> Result<SyncSuggestSubtitlesResult, String> {
    let candidates = vesta_sync::matching::suggest_subtitles_for_media(Path::new(&media_path))
        .map_err(|e| e.to_string())?;

    let candidates: Vec<_> = candidates.into_iter().filter(|c| c.1 >= 45).collect();

    let (target, native) = vesta_sync::matching::suggest_target_native_subtitles(
        &candidates,
        default_target_lang.as_deref(),
        default_native_lang.as_deref(),
    );

    Ok(SyncSuggestSubtitlesResult {
        target: target.map(|p| p.to_string_lossy().into_owned()),
        native: native.map(|p| p.to_string_lossy().into_owned()),
    })
}

fn allow_playback_media_path(allowlist: Option<&MediaAllowlist>, path: &Path) {
    if let Some(allowlist) = allowlist {
        let _ = allowlist.allow_file(path);
    }
}

#[tauri::command]
pub async fn sync_prepare_media_for_playback(
    app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
    use tauri::Manager;

    let allowlist = app.try_state::<MediaAllowlist>();
    if vesta_sync::playback::is_natively_playable(Path::new(&path)) {
        allow_playback_media_path(allowlist.as_deref(), Path::new(&path));
        return Ok(path);
    }
    let ffmpeg_cmd = super::media::resolve_ffmpeg_path(Some(&app)).await;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Cannot get cache dir: {}", e))?
        .join("media_cache");

    let source = Path::new(&path).to_path_buf();
    let output_path = tokio::task::spawn_blocking(move || {
        vesta_sync::playback::transcode_for_playback(&source, &cache_dir, &ffmpeg_cmd)
    })
    .await
    .map_err(|e| format!("Transcoding task panicked: {}", e))?
    .map_err(|e| e.to_string())?;

    allow_playback_media_path(allowlist.as_deref(), &output_path);
    Ok(output_path.to_string_lossy().to_string())
}

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
        can_undo: engine.can_undo(),
        can_redo: engine.can_redo(),
    }
}

#[cfg(test)]
mod history_tests {
    use super::*;
    use crate::state::OperationGuard;

    #[test]
    fn history_is_locked_while_an_operation_owns_the_document() {
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
        let mut engine = SyncEngine::new(source.path()).unwrap();
        engine.add_anchor(1, 1_200, true).unwrap();
        let state = AppSyncState::default();
        state.lock().unwrap().replace_engine(Some(engine)).unwrap();
        let guard = OperationGuard::begin(&state, "busy").unwrap();
        assert!(change_history(&state, false).is_err());
        assert_eq!(
            state
                .lock()
                .unwrap()
                .engine
                .as_ref()
                .unwrap()
                .anchor_count(),
            1
        );
        drop(guard);
        let undone = change_history(&state, false).unwrap();
        assert_eq!(undone.anchor_count, 0);
        assert!(undone.can_redo);
        assert_eq!(change_history(&state, true).unwrap().anchor_count, 1);
    }

    #[test]
    fn sync_registers_video_session_and_playback_paths_in_media_allowlist() {
        let dir = tempfile::tempdir().unwrap();
        let srt_path = dir.path().join("subs.srt");
        let video_path = dir.path().join("episode.mp4");
        let transcoded_path = dir.path().join("cached.wav");
        let secret_path = dir.path().join("secret.txt");
        std::fs::write(&srt_path, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
        std::fs::write(&video_path, b"mp4-bytes").unwrap();
        std::fs::write(&transcoded_path, b"wav-bytes").unwrap();
        std::fs::write(&secret_path, b"secret").unwrap();

        let allowlist = MediaAllowlist::default();
        let mut sync_state = crate::state::SyncState::with_allowlist(allowlist.clone());
        let engine = SyncEngine::new(&srt_path).unwrap();
        sync_state.replace_engine(Some(engine)).unwrap();
        sync_state
            .set_video_path(video_path.to_str().unwrap())
            .unwrap();
        assert!(allowlist.is_allowed(&video_path));
        assert!(!allowlist.is_allowed(&secret_path));
        assert!(!allowlist.is_allowed(Path::new("/etc/passwd")));

        // Session reload restores video_path into a fresh allowlist.
        let session_path = dir.path().join("session.json");
        sync_state
            .engine
            .as_ref()
            .unwrap()
            .save_session(&session_path)
            .unwrap();
        let reloaded_allowlist = MediaAllowlist::default();
        let mut reloaded_state =
            crate::state::SyncState::with_allowlist(reloaded_allowlist.clone());
        let loaded_engine = SyncEngine::load_session(&session_path).unwrap();
        reloaded_state.replace_engine(Some(loaded_engine)).unwrap();
        assert!(reloaded_allowlist.is_allowed(&video_path));
        assert!(!reloaded_allowlist.is_allowed(&secret_path));

        // Playback preparation registers native and transcoded files in allowlist.
        allow_playback_media_path(Some(&reloaded_allowlist), &transcoded_path);
        assert!(reloaded_allowlist.is_allowed(&transcoded_path));
    }
}
