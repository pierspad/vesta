import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";
import type { EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";
import { loadCardTemplates, noteTypeOutputFields, type NoteTypeDef } from "$lib/types/noteTypes";

/**
 * Single source of truth for the payload of the `flashcard_generate` Tauri
 * command. Movie mode and series mode (per-episode) both build their config
 * through here — before this module existed FlashcardsTab kept two nearly
 * identical ~60-line object literals that drifted apart silently.
 */
export interface FlashcardConfigInputs {
  targetSubsPath: string;
  nativeSubsPath: string | null;
  videoPath: string | null;
  audioPath: string | null;
  outputDir: string;
  cardFilters: CardFilterSettings;
  /** Effective media settings for this run (generic or per-episode merge). */
  media: Required<EpisodeMediaOverrides>;
  /** Already gated by media availability at the call site. */
  generateAudio: boolean;
  generateSnapshots: boolean;
  generateVideoClips: boolean;
  audioTrackIndex: number | null;
  videoHwAccel: string;
  deckName: string;
  episodeNumber: number;
  exportFormat: string;
  noteType: NoteTypeDef;
  cpuCores: number;
}

export function buildFlashcardConfig(i: FlashcardConfigInputs) {
  const templates = loadCardTemplates();
  const f = i.cardFilters;
  return {
    target_subs_path: i.targetSubsPath,
    native_subs_path: i.nativeSubsPath,
    video_path: i.videoPath,
    audio_path: i.audioPath,
    output_dir: i.outputDir,
    use_timings_from: "target",
    span_start_ms: null,
    span_end_ms: null,
    time_shift_target_ms: 0,
    time_shift_native_ms: 0,
    filters: {
      include_words: null,
      exclude_words: null,
      exclude_duplicates_subs1: false,
      exclude_duplicates_subs2: false,
      min_chars: f.enabled && f.minCharsEnabled ? f.minChars : null,
      max_chars: f.enabled && f.maxCharsEnabled ? f.maxChars : null,
      min_duration_ms: f.enabled && f.minDurationEnabled ? f.minDurationMs : null,
      max_duration_ms: f.enabled && f.maxDurationEnabled ? f.maxDurationMs : null,
      exclude_styled: false,
      actor_filter: null,
      only_cjk: false,
      remove_no_match: false,
    },
    context: {
      leading: 0,
      trailing: 0,
      max_gap_seconds: 15.0,
    },
    combine_sentences: f.enabled && f.combineSentences,
    continuation_chars: f.continuationChars,
    generate_audio: i.generateAudio,
    audio_bitrate: i.media.audioBitrate,
    audio_track_index: i.audioTrackIndex,
    normalize_audio: i.media.normalizeAudio,
    audio_pad_start_ms: i.media.audioPadStart,
    audio_pad_end_ms: i.media.audioPadEnd,
    generate_snapshots: i.generateSnapshots,
    snapshot_width: i.media.snapshotWidth,
    snapshot_height: i.media.snapshotHeight,
    crop_bottom: i.media.cropBottom,
    generate_video_clips: i.generateVideoClips,
    video_codec: i.media.videoCodec,
    h264_preset: i.media.h264Preset,
    video_hw_accel: i.videoHwAccel,
    video_bitrate: i.media.videoBitrate,
    video_audio_bitrate: i.media.videoAudioBitrate,
    video_pad_start_ms: i.media.videoPadStart,
    video_pad_end_ms: i.media.videoPadEnd,
    deck_name: i.deckName,
    episode_number: i.episodeNumber,
    export_format: i.exportFormat,
    note_type_name: i.noteType.name,
    field_names: i.noteType.fields,
    output_fields: noteTypeOutputFields(i.noteType),
    cpu_cores: i.cpuCores,
    card_front_html: templates.frontHtml,
    card_back_html: templates.backHtml,
    card_css: templates.css,
  };
}
