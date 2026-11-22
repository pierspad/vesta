//! Model-free WebRTC VAD, operating on the pipeline's mono 16 kHz PCM.
use anyhow::Result;
use std::ops::Range;
use tokio_util::sync::CancellationToken;
use webrtc_vad::{SampleRate, Vad, VadMode};

pub(crate) fn speech_spans(
    samples: &[f32],
    pad_ms: i64,
    merge_gap_ms: i64,
    cancel: Option<&CancellationToken>,
) -> Result<Vec<Range<usize>>> {
    // Quality mode favours recall: more aggressive modes can discard quiet speech.
    let mut vad = Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, VadMode::Quality);
    const FRAME: usize = 320; // WebRTC accepts 10/20/30 ms frames.
    let padding = pad_ms.max(0) as usize * 16;
    let gap = merge_gap_ms.max(0) as usize * 16;
    let mut spans: Vec<Range<usize>> = Vec::new();
    let mut pcm = [0i16; FRAME];
    for (index, chunk) in samples.chunks(FRAME).enumerate() {
        if cancel.is_some_and(CancellationToken::is_cancelled) {
            anyhow::bail!("Transcription cancelled");
        }
        pcm.fill(0);
        for (out, sample) in pcm.iter_mut().zip(chunk) {
            *out = if sample.is_finite() {
                (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
            } else {
                0
            };
        }
        let speech = vad
            .is_voice_segment(&pcm)
            .map_err(|_| anyhow::anyhow!("WebRTC VAD rejected a 20 ms frame"))?;
        if speech {
            let start = (index * FRAME).saturating_sub(padding);
            let end = ((index * FRAME) + chunk.len() + padding).min(samples.len());
            if let Some(previous) = spans.last_mut()
                && start <= previous.end.saturating_add(gap)
            {
                previous.end = previous.end.max(end);
            } else {
                spans.push(start..end);
            }
        }
    }
    Ok(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_empty_and_partial_frames_do_not_produce_speech() {
        for length in [0, 1, 319, 320, 321, 16_000] {
            assert!(
                speech_spans(&vec![0.0; length], 150, 2000, None)
                    .unwrap()
                    .is_empty()
            );
        }
    }
    #[test]
    fn voiced_signal_produces_bounded_padded_spans() {
        let mut samples = vec![0.0; 16_000];
        samples.extend((0..16_000).map(|index| {
            let t = index as f32 / 16_000.0;
            (std::f32::consts::TAU * 120.0 * t).sin() * 0.3
                + (std::f32::consts::TAU * 800.0 * t).sin() * 0.15
                + (std::f32::consts::TAU * 1400.0 * t).sin() * 0.1
        }));
        samples.extend(vec![0.0; 16_001]);
        let spans = speech_spans(&samples, 150, 2000, None).unwrap();
        assert!(!spans.is_empty());
        for span in spans {
            assert!(span.start < span.end);
            assert!(span.end <= samples.len());
            assert!(span.start >= 16_000 - 2400);
        }
    }

    #[test]
    fn cancellation_is_honoured_before_processing() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(speech_spans(&vec![0.0; 320], 150, 2000, Some(&cancel)).is_err());
    }
}
