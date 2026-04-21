import re

with open('apps/srt-gui/src-tauri/src/commands/auto_sync.rs', 'r') as f:
    text = f.read()

# Replace the loop to be inside a spawn_blocking block

pattern = r"    let temp_dir = tempfile::tempdir\(\)\.map_err\(\|e\| e\.to_string\(\)\)\?;\n    let mut all_matches: Vec<MatchCandidate> = Vec::new\(\);\n    let total_segments = sample_positions\.len\(\);\n\n    for \(idx, &start_pos\) in sample_positions\.iter\(\)\.enumerate\(\) \{.*?let mut best_offset"

# We'll use regex.dotall to match everything until `let mut best_offset`

def repl(m):
    return """
    let app_clone = app.clone();
    let model_path_str = model_path.to_string_lossy().to_string();
    let media_path_str = media_path.clone();
    let language_clone = language.clone();
    let token_clone = token.clone();
    let subtitle_infos_clone = subtitle_infos.clone();
    let temp_dir_path = tempfile::tempdir().map_err(|e| e.to_string())?.into_path();
    
    let total_segments = sample_positions.len();

    let spawn_res = tokio::task::spawn_blocking(move || -> Result<(Vec<MatchCandidate>, usize, bool), String> {
        let mut all_matches: Vec<MatchCandidate> = Vec::new();
        
        let ctx = whisper_rs::WhisperContext::new_with_params(
            &model_path_str,
            whisper_rs::WhisperContextParameters::default(),
        ).map_err(|e| format!("Failed to load Whisper model: {:?}", e))?;

        for (idx, &start_pos) in sample_positions.iter().enumerate() {
            if token_clone.is_cancelled() {
                emit_auto_sync_progress(
                    &app_clone,
                    "cancelled",
                    "Auto-sync cancelled by user.".to_string(),
                    100.0,
                    Some("sync.autoSyncProgress.cancelled"),
                    None,
                );
                return Ok((all_matches, idx, true));
            }

            let progress = (idx as f64 / total_segments as f64) * 80.0 + 10.0;

            let start_label = format_mm_ss(start_pos);
            let end_label = format_mm_ss(start_pos + segment_duration);
            emit_auto_sync_progress(
                &app_clone,
                "transcribe",
                format!(
                    "Analyzing segment {}/{} - media {} -> {} ({}s)",
                    idx + 1,
                    total_segments,
                    start_label,
                    end_label,
                    segment_duration.round() as i64
                ),
                progress,
                Some("sync.autoSyncProgress.transcribingSegment"),
                Some(std::collections::HashMap::from([
                    ("current".to_string(), (idx + 1).to_string()),
                    ("total".to_string(), total_segments.to_string()),
                    ("start".to_string(), start_label),
                    ("end".to_string(), end_label),
                    (
                        "duration".to_string(),
                        format!("{}s", segment_duration.round() as i64),
                    ),
                ])),
            );

            let wav_path = temp_dir_path.join(format!("segment_{}.wav", idx));
            let wav_str = wav_path.to_string_lossy().to_string();

            if let Err(e) = extract_audio_segment(&media_path_str, start_pos, segment_duration, &wav_str) {
                eprintln!("[auto-sync] Segment {} extraction failed: {}", idx, e);
                continue;
            }

            let audio_data = match read_wav_to_f32(&wav_path) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("[auto-sync] Failed to read segment {}: {}", idx, e);
                    continue;
                }
            };

            if token_clone.is_cancelled() { break; }

            let transcribed = match transcribe_segment_with_ctx(&ctx, &audio_data, language_clone.as_deref()) {
                Ok(segs) => segs,
                Err(e) => {
                    eprintln!("[auto-sync] Segment {} transcription failed: {}", idx, e);
                    continue;
                }
            };

            let adjusted_segments: Vec<TranscribedSegment> = transcribed.into_iter()
                .map(|mut seg| {
                    seg.start_ms += (start_pos * 1000.0) as i64;
                    seg.end_ms += (start_pos * 1000.0) as i64;
                    seg
                })
                .collect();

            for tseg in &adjusted_segments {
                for &(sub_id, sub_start_ms, ref sub_text) in &subtitle_infos_clone {
                    let time_diff = (sub_start_ms - tseg.start_ms).abs();
                    if time_diff > 120_000 {
                        continue;
                    }

                    let sim = text_similarity(&tseg.text, sub_text);
                    if sim > 0.4 {
                        all_matches.push(MatchCandidate {
                            subtitle_id: sub_id,
                            original_start_ms: sub_start_ms,
                            transcribed_start_ms: tseg.start_ms,
                            similarity: sim,
                        });
                    }
                }
            }
            let _ = std::fs::remove_file(&wav_path);
        }
        
        Ok((all_matches, total_segments, token_clone.is_cancelled()))
    }).await.map_err(|e| format!("Task panic: {:?}", e))?;
    
    let (all_matches, segments_analyzed, is_cancelled) = spawn_res?;
    let _ = std::fs::remove_dir_all(&temp_dir_path);

    if is_cancelled {
        return Ok(AutoSyncResult {
            success: false,
            cancelled: true,
            anchors_created: 0,
            segments_analyzed,
            message: "Auto-sync was cancelled.".to_string(),
        });
    }

    let mut best_offset"""

modified = re.sub(pattern, repl, text, flags=re.DOTALL)

with open('apps/srt-gui/src-tauri/src/commands/auto_sync.rs', 'w') as f:
    f.write(modified)
