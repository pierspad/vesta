/**
 * Persistence for the movie-mode media settings.
 *
 * Before this module only `snapshotWidth` and `snapshotHeight` survived a
 * restart, each through its own `vestaConfig` key and its own `$effect`.
 * Everything else — bitrate, padding, h264 preset, crop — silently reset to a
 * hardcoded default on every launch, and adding formats and quality would have
 * meant six keys and six effects. One blob, one sanitizer, one effect instead.
 */

import * as vestaConfig from "$lib/config/vestaConfig";
import {
  DEFAULT_QUALITY_STEP,
  DEFAULT_RESOLUTION,
  DEFAULT_VIDEO_QUALITY_STEP,
  type AudioFormat,
  type EpisodeMediaOverrides,
  type SnapshotFormat,
} from "$lib/types/flashcardMediaTypes";

export const MEDIA_SETTINGS_KEY = "vesta-flashcards-media-settings";

/** Legacy single-value keys, read once during migration then left alone. */
const LEGACY_WIDTH_KEY = "vesta-flashcards-media-width";
const LEGACY_HEIGHT_KEY = "vesta-flashcards-media-height";

export type MediaSettings = Required<EpisodeMediaOverrides>;

export const defaultMediaSettings: MediaSettings = {
  generateAudio: true,
  audioBitrate: 128,
  audioTrackIndex: null,
  normalizeAudio: true,
  audioBoost: false,
  audioPadStart: 0,
  audioPadEnd: 0,
  audioFormat: "mp3",
  generateSnapshots: true,
  snapshotWidth: DEFAULT_QUALITY_STEP.snapshotWidth,
  snapshotHeight: DEFAULT_QUALITY_STEP.snapshotHeight,
  cropBottom: 0,
  snapshotFormat: "webp",
  snapshotQuality: DEFAULT_QUALITY_STEP.snapshotQuality,
  generateVideoClips: false,
  videoCodec: "h264",
  h264Preset: DEFAULT_VIDEO_QUALITY_STEP.h264Preset,
  videoBitrate: DEFAULT_VIDEO_QUALITY_STEP.videoBitrate,
  videoAudioBitrate: DEFAULT_VIDEO_QUALITY_STEP.videoAudioBitrate,
  videoPadStart: 250,
  videoPadEnd: 50,
  videoWidth: DEFAULT_VIDEO_QUALITY_STEP.width,
  videoHeight: DEFAULT_VIDEO_QUALITY_STEP.height,
};

const SNAPSHOT_FORMATS: SnapshotFormat[] = ["jpeg", "webp", "avif"];
const AUDIO_FORMATS: AudioFormat[] = ["mp3", "opus"];

function bool(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function int(value: unknown, fallback: number, min: number, max: number): number {
  const n = typeof value === "number" ? value : Number.parseInt(String(value ?? ""), 10);
  if (!Number.isFinite(n)) return fallback;
  return Math.min(Math.max(Math.round(n), min), max);
}

function oneOf<T extends string>(value: unknown, allowed: T[], fallback: T): T {
  return allowed.includes(value as T) ? (value as T) : fallback;
}

/**
 * Clamp every field and drop anything unrecognised, so a blob written by an
 * older or newer Vesta — or a hand-edited one — cannot leave the tab in an
 * unusable state. Each field falls back independently: one bad value does not
 * discard the rest of the user's settings.
 */
export function sanitizeMediaSettings(raw: unknown): MediaSettings {
  const d = defaultMediaSettings;
  if (!raw || typeof raw !== "object") return { ...d };
  const o = raw as Record<string, unknown>;

  const trackIndex =
    o.audioTrackIndex === null || o.audioTrackIndex === undefined
      ? null
      : int(o.audioTrackIndex, 0, 0, 255);

  return {
    generateAudio: bool(o.generateAudio, d.generateAudio),
    audioBitrate: int(o.audioBitrate, d.audioBitrate, 8, 512),
    audioTrackIndex: trackIndex,
    normalizeAudio: bool(o.normalizeAudio, d.normalizeAudio),
    audioBoost: bool(o.audioBoost, d.audioBoost),
    audioPadStart: int(o.audioPadStart, d.audioPadStart, 0, 60_000),
    audioPadEnd: int(o.audioPadEnd, d.audioPadEnd, 0, 60_000),
    audioFormat: oneOf(o.audioFormat, AUDIO_FORMATS, d.audioFormat),
    generateSnapshots: bool(o.generateSnapshots, d.generateSnapshots),
    snapshotWidth: int(o.snapshotWidth, d.snapshotWidth, 16, 7680),
    snapshotHeight: int(o.snapshotHeight, d.snapshotHeight, 16, 4320),
    cropBottom: int(o.cropBottom, d.cropBottom, 0, 4320),
    snapshotFormat: oneOf(o.snapshotFormat, SNAPSHOT_FORMATS, d.snapshotFormat),
    snapshotQuality: int(o.snapshotQuality, d.snapshotQuality, 0, 100),
    generateVideoClips: bool(o.generateVideoClips, d.generateVideoClips),
    videoCodec: typeof o.videoCodec === "string" ? o.videoCodec : d.videoCodec,
    h264Preset: typeof o.h264Preset === "string" ? o.h264Preset : d.h264Preset,
    videoBitrate: int(o.videoBitrate, d.videoBitrate, 50, 50_000),
    videoAudioBitrate: int(o.videoAudioBitrate, d.videoAudioBitrate, 8, 512),
    videoPadStart: int(o.videoPadStart, d.videoPadStart, 0, 60_000),
    videoPadEnd: int(o.videoPadEnd, d.videoPadEnd, 0, 60_000),
    videoWidth: int(o.videoWidth, typeof o.snapshotWidth === "number" ? o.snapshotWidth : d.videoWidth, 16, 7680),
    videoHeight: int(o.videoHeight, typeof o.snapshotHeight === "number" ? o.snapshotHeight : d.videoHeight, 16, 4320),
  };
}

/**
 * Pick up the two pre-blob dimension keys so an existing user's snapshot size
 * is not silently replaced by the new 16:9 default. Returns a partial to layer
 * under the blob, which always wins when present.
 */
export function readLegacyDimensions(
  get: (key: string) => string | null,
): Partial<MediaSettings> {
  const out: Partial<MediaSettings> = {};
  const w = Number.parseInt(get(LEGACY_WIDTH_KEY) || "", 10);
  const h = Number.parseInt(get(LEGACY_HEIGHT_KEY) || "", 10);
  if (Number.isFinite(w) && w > 0) out.snapshotWidth = w;
  if (Number.isFinite(h) && h > 0) out.snapshotHeight = h;
  return out;
}

export function loadMediaSettings(): MediaSettings {
  try {
    const raw = vestaConfig.getItem(MEDIA_SETTINGS_KEY);
    if (raw) return sanitizeMediaSettings(JSON.parse(raw));
    // First run after the upgrade: inherit whatever the old keys held.
    return sanitizeMediaSettings({
      ...defaultMediaSettings,
      ...readLegacyDimensions((k) => vestaConfig.getItem(k)),
    });
  } catch {
    return { ...defaultMediaSettings };
  }
}

export function saveMediaSettings(settings: MediaSettings): void {
  try {
    vestaConfig.setItem(MEDIA_SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    /* storage unavailable — settings simply do not persist this session */
  }
}
