//! # srt-condense
//!
//! Motore headless per il **condensed audio**: estrae dal media solo i
//! segmenti parlati e li concatena in un unico file audio — il classico
//! strumento da language learning (riascolti un episodio in metà del tempo,
//! senza silenzi né musica).
//!
//! Due strategie di rilevamento dei segmenti:
//! - [`CondenseMode::Subtitles`] — i timestamp di un file SRT (veloce,
//!   nessun modello richiesto);
//! - [`CondenseMode::Vad`] — Silero VAD via `srt-transcribe` (non richiede
//!   sottotitoli; serve il modello VAD installato).
//!
//! Come gli altri engine del workspace: nessun accoppiamento GUI, progresso
//! via callback, cancellazione via `CancellationToken`, errori `String`
//! già presentabili all'utente.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use srt_parser::SrtParser;
use tokio_util::sync::CancellationToken;

// ─── Configuration ───────────────────────────────────────────────────────────

/// Strategia di rilevamento dei segmenti parlati.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CondenseMode {
    /// Usa i timestamp di un file SRT.
    Subtitles { srt_path: String },
    /// Usa Silero VAD sull'audio (richiede il modello VAD installato).
    Vad,
}

/// Configurazione di un run di condensazione.
#[derive(Debug, Clone, Deserialize)]
pub struct CondenseConfig {
    /// File media di ingresso (video o audio).
    pub media_path: String,
    /// File audio di uscita (`.mp3`).
    pub output_path: String,
    pub mode: CondenseMode,
    /// Padding attorno a ogni segmento (ms). Default consigliato: 150.
    #[serde(default = "default_pad_ms")]
    pub pad_ms: i64,
    /// Gap massimo fra segmenti da fondere in uno solo (ms). Default: 1500.
    #[serde(default = "default_merge_gap_ms")]
    pub merge_gap_ms: i64,
    /// Bitrate MP3 di uscita (kb/s). Default: 128.
    #[serde(default = "default_bitrate")]
    pub bitrate_kbps: u32,
    /// Traccia audio da usare (indice 0-based), `None` = default del file.
    #[serde(default)]
    pub audio_track_index: Option<usize>,
    /// Thread per la VAD (`None` = automatico).
    #[serde(default)]
    pub n_threads: Option<usize>,
}

fn default_pad_ms() -> i64 {
    150
}
fn default_merge_gap_ms() -> i64 {
    1_500
}
fn default_bitrate() -> u32 {
    128
}

/// Evento di progresso.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CondenseProgress {
    /// "detect" | "extract" | "concat"
    pub stage: String,
    pub message: String,
    pub current: usize,
    pub total: usize,
    pub percentage: f64,
}

/// Risultato finale.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CondenseResult {
    pub success: bool,
    pub message: String,
    pub output_path: String,
    /// Numero di segmenti concatenati.
    pub spans: usize,
    /// Durata dell'audio condensato (somma dei segmenti, ms).
    pub output_duration_ms: i64,
    /// Durata del media originale (ms, 0 = sconosciuta).
    pub input_duration_ms: i64,
}

pub type CondenseCallback<'a> = &'a (dyn Fn(CondenseProgress) + Send + Sync);

// ─── Engine ──────────────────────────────────────────────────────────────────

/// Costruisce il condensed audio secondo `config`.
///
/// `ffmpeg_cmd` può essere un comando in `PATH` ("ffmpeg") o un percorso
/// assoluto a un binario bundled.
pub async fn condense(
    config: CondenseConfig,
    ffmpeg_cmd: &str,
    progress: CondenseCallback<'_>,
    token: CancellationToken,
) -> Result<CondenseResult, String> {
    if !Path::new(&config.media_path).exists() {
        return Err(format!("File media non trovato: {}", config.media_path));
    }

    let emit = |stage: &str, message: String, current: usize, total: usize| {
        let percentage = if total > 0 {
            current as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        progress(CondenseProgress {
            stage: stage.to_string(),
            message,
            current,
            total,
            percentage,
        });
    };

    // ── Stage 1: speech spans ────────────────────────────────────────────────
    emit("detect", "Rilevamento segmenti parlati...".to_string(), 0, 0);

    let (raw_spans, input_duration_ms) = match &config.mode {
        CondenseMode::Subtitles { srt_path } => {
            let subs = SrtParser::parse_file(srt_path)
                .map_err(|e| format!("Errore parsing SRT: {e}"))?;
            let mut spans: Vec<(i64, i64)> = subs
                .values()
                .map(|s| (s.start.milliseconds as i64, s.end.milliseconds as i64))
                .filter(|(s, e)| e > s)
                .collect();
            spans.sort_unstable();
            (spans, 0)
        }
        CondenseMode::Vad => {
            // Usa sempre la variante di default: la selezione multi-modello/
            // custom (Settings → Whisper) è oggi cablata solo nella pipeline
            // di trascrizione, non nel condense.
            let model_path =
                srt_transcribe::model::vad_model_path(srt_transcribe::model::DEFAULT_VAD_MODEL_ID)
                    .map_err(|e| format!("Modello VAD: {e}"))?;
            if !model_path.exists() {
                return Err(
                    "Modello Silero VAD non installato. Scaricalo da Impostazioni → Whisper."
                        .to_string(),
                );
            }

            // Decodifica a WAV 16 kHz mono (il formato richiesto da Silero).
            let temp = tempfile::tempdir().map_err(|e| format!("Directory temporanea: {e}"))?;
            let wav_path = temp.path().join("condense_input.wav");
            srt_transcribe::audio::convert_to_wav(
                ffmpeg_cmd,
                Path::new(&config.media_path),
                &wav_path,
                Some(&token),
            )
            .await
            .map_err(|e| format!("Decodifica audio fallita: {e}"))?;

            if token.is_cancelled() {
                return Err("Operazione annullata".to_string());
            }

            let samples = srt_transcribe::audio::read_wav_to_f32(&wav_path)
                .map_err(|e| format!("Lettura WAV fallita: {e}"))?;
            let duration_ms = (samples.len() as i64) / 16; // 16 campioni/ms
            let n_threads = config
                .n_threads
                .unwrap_or_else(srt_transcribe::transcribe::default_n_threads);
            let pad = config.pad_ms;
            let gap = config.merge_gap_ms;

            // La VAD è CPU-bound: fuori dal runtime async.
            let spans = tokio::task::spawn_blocking(move || {
                srt_transcribe::transcribe::vad_speech_spans_ms(
                    &model_path,
                    &samples,
                    n_threads,
                    pad,
                    gap,
                )
            })
            .await
            .map_err(|e| format!("Task VAD fallito: {e}"))?
            .map_err(|e| format!("VAD fallita: {e}"))?;

            (spans, duration_ms)
        }
    };

    if token.is_cancelled() {
        return Err("Operazione annullata".to_string());
    }

    let spans = merge_spans(&raw_spans, config.pad_ms, config.merge_gap_ms);
    if spans.is_empty() {
        return Err("Nessun segmento parlato rilevato".to_string());
    }
    let output_duration_ms: i64 = spans.iter().map(|(s, e)| e - s).sum();

    // ── Stage 2: parallel segment extraction ─────────────────────────────────
    let temp = tempfile::tempdir().map_err(|e| format!("Directory temporanea: {e}"))?;
    let total = spans.len();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .saturating_sub(1)
        .max(1);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(workers));
    let ffmpeg: Arc<str> = Arc::from(ffmpeg_cmd);
    let media: Arc<str> = Arc::from(config.media_path.as_str());

    let mut tasks: tokio::task::JoinSet<Result<usize, String>> = tokio::task::JoinSet::new();
    let mut segment_paths: Vec<PathBuf> = Vec::with_capacity(total);

    for (idx, &(start_ms, end_ms)) in spans.iter().enumerate() {
        let seg_path = temp.path().join(format!("seg_{idx:05}.mp3"));
        segment_paths.push(seg_path.clone());

        let semaphore = semaphore.clone();
        let ffmpeg = ffmpeg.clone();
        let media = media.clone();
        let token = token.clone();
        let bitrate = config.bitrate_kbps;
        let track = config.audio_track_index;

        tasks.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .map_err(|_| "semaforo chiuso".to_string())?;
            if token.is_cancelled() {
                return Err("Operazione annullata".to_string());
            }
            extract_segment(&ffmpeg, &media, &seg_path, start_ms, end_ms, bitrate, track)
                .await
                .map_err(|e| format!("Segmento {}: {e}", idx + 1))?;
            Ok(idx)
        });
    }

    let mut done = 0usize;
    while let Some(joined) = tasks.join_next().await {
        if token.is_cancelled() {
            tasks.abort_all();
            return Err("Operazione annullata".to_string());
        }
        joined.map_err(|e| format!("Task estrazione fallito: {e}"))??;
        done += 1;
        emit(
            "extract",
            format!("Estrazione segmenti {done}/{total}"),
            done,
            total,
        );
    }

    // ── Stage 3: concat ──────────────────────────────────────────────────────
    emit("concat", "Concatenazione...".to_string(), total, total);

    let list_path = temp.path().join("concat.txt");
    let list_content: String = segment_paths
        .iter()
        .map(|p| format!("file '{}'\n", p.display()))
        .collect();
    std::fs::write(&list_path, list_content).map_err(|e| format!("Scrittura lista: {e}"))?;

    let output = tokio::process::Command::new(ffmpeg_cmd)
        .args(["-nostdin", "-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args(["-c", "copy"])
        .arg(&config.output_path)
        .output()
        .await
        .map_err(|e| format!("Avvio ffmpeg (concat) fallito: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Concatenazione fallita: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(CondenseResult {
        success: true,
        message: format!("Condensed audio creato ({} segmenti)", spans.len()),
        output_path: config.output_path,
        spans: spans.len(),
        output_duration_ms,
        input_duration_ms,
    })
}

/// Pad + merge dei segmenti: ordina, applica `pad_ms` a entrambi i lati e
/// fonde ogni coppia più vicina di `merge_gap_ms`.
fn merge_spans(spans: &[(i64, i64)], pad_ms: i64, merge_gap_ms: i64) -> Vec<(i64, i64)> {
    let mut padded: Vec<(i64, i64)> = spans
        .iter()
        .map(|&(s, e)| ((s - pad_ms).max(0), e + pad_ms))
        .collect();
    padded.sort_unstable();

    let mut merged: Vec<(i64, i64)> = Vec::with_capacity(padded.len());
    for (start, end) in padded {
        match merged.last_mut() {
            Some((_, last_end)) if start - *last_end <= merge_gap_ms => {
                *last_end = end.max(*last_end);
            }
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// Estrae un singolo segmento audio come MP3.
async fn extract_segment(
    ffmpeg_cmd: &str,
    media_path: &str,
    output_path: &Path,
    start_ms: i64,
    end_ms: i64,
    bitrate_kbps: u32,
    audio_track_index: Option<usize>,
) -> anyhow::Result<()> {
    let mut cmd = tokio::process::Command::new(ffmpeg_cmd);
    cmd.args([
        "-nostdin",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &ms_to_ts(start_ms),
        "-t",
        &ms_to_ts(end_ms - start_ms),
        "-i",
        media_path,
        "-vn",
        "-sn",
        "-dn",
    ]);
    if let Some(track) = audio_track_index {
        cmd.args(["-map", &format!("0:a:{track}")]);
    }
    cmd.args([
        "-ac",
        "2",
        "-ar",
        "44100",
        "-b:a",
        &format!("{bitrate_kbps}k"),
        "-f",
        "mp3",
    ]);
    cmd.arg(output_path);

    let output = cmd.output().await?;
    if !output.status.success() {
        anyhow::bail!("ffmpeg: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

/// Format milliseconds as ffmpeg timestamp HH:MM:SS.mmm
fn ms_to_ts(ms: i64) -> String {
    let ms = ms.max(0);
    let total_secs = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        total_secs / 3600,
        (total_secs / 60) % 60,
        total_secs % 60,
        ms % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_spans_pads_and_merges() {
        // Two spans 1000ms apart with gap 1500 → merged into one.
        let spans = vec![(1_000, 2_000), (3_000, 4_000)];
        let merged = merge_spans(&spans, 100, 1_500);
        assert_eq!(merged, vec![(900, 4_100)]);

        // Gap larger than merge threshold → stay separate.
        let spans = vec![(1_000, 2_000), (10_000, 11_000)];
        let merged = merge_spans(&spans, 0, 1_500);
        assert_eq!(merged, vec![(1_000, 2_000), (10_000, 11_000)]);
    }

    #[test]
    fn merge_spans_clamps_at_zero_and_sorts() {
        let spans = vec![(5_000, 6_000), (50, 400)];
        let merged = merge_spans(&spans, 100, 500);
        assert_eq!(merged, vec![(0, 500), (4_900, 6_100)]);
    }

    #[test]
    fn ms_to_ts_formats() {
        assert_eq!(ms_to_ts(0), "00:00:00.000");
        assert_eq!(ms_to_ts(3_661_042), "01:01:01.042");
    }
}
