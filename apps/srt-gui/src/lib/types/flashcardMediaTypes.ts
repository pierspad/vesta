/** Per-episode overrides for the movie-mode media settings (audio/snapshot/
 * video generation). Every field mirrors one primitive of `mediaSettings` in
 * FlashcardsTab.svelte — an episode only stores the keys that actually
 * differ from the generic settings (see buildEpisodeMediaOverrideDiff). */
export type SnapshotFormat = "jpeg" | "webp" | "avif";
export type AudioFormat = "mp3" | "opus";

export interface EpisodeMediaOverrides {
  generateAudio?: boolean;
  audioBitrate?: number;
  audioTrackIndex?: number | null;
  normalizeAudio?: boolean;
  audioBoost?: boolean;
  audioPadStart?: number;
  audioPadEnd?: number;
  audioFormat?: AudioFormat;
  generateSnapshots?: boolean;
  snapshotWidth?: number;
  snapshotHeight?: number;
  cropBottom?: number;
  snapshotFormat?: SnapshotFormat;
  /** User-facing 0-100 scale; the backend maps it onto each codec's own. */
  snapshotQuality?: number;
  generateVideoClips?: boolean;
  videoCodec?: string;
  h264Preset?: string;
  videoBitrate?: number;
  videoAudioBitrate?: number;
  videoPadStart?: number;
  videoPadEnd?: number;
  videoWidth?: number;
  videoHeight?: number;
}

export type EpisodeMediaOverrideKey = keyof EpisodeMediaOverrides;

export interface AudioTrackInfo {
  index: number;
  stream_index: number;
  codec: string | null;
  language: string | null;
  title: string | null;
  channels: number | null;
}

export function formatAudioTrackLabel(track: AudioTrackInfo): string {
  const parts = [`#${track.index + 1}`];
  if (track.language) parts.push(track.language.toUpperCase());
  if (track.title) parts.push(track.title);
  if (track.codec) parts.push(track.codec);
  if (track.channels) parts.push(`${track.channels} ch`);
  return parts.join(" - ");
}

/** Whether an episode has any per-episode media overrides set (drives the
 * violet "overridden" dot in the episode table's media-file cell). */
export function hasMediaOverrides(overrides: EpisodeMediaOverrides | undefined): boolean {
  return Boolean(overrides && Object.keys(overrides).length > 0);
}

/* ── Resolution presets ──────────────────────────────────────────────────
 * Tom's feedback: "non-tech people won't calculate the x and y of their
 * favorite sizes". All 16:9, matching the source material — the old 240x160
 * default was 3:2. */

export interface ResolutionPreset {
  id: string;
  label: string;
  width: number;
  height: number;
}

export const RESOLUTION_PRESETS: ResolutionPreset[] = [
  { id: "144p", label: "144p", width: 256, height: 144 },
  { id: "240p", label: "240p", width: 426, height: 240 },
  { id: "360p", label: "360p", width: 640, height: 360 },
  { id: "480p", label: "480p", width: 854, height: 480 },
];

/* 144p is the default rather than 240p: it holds the same pixel budget as the
 * old 240x160, so WebP's ~43% saving actually reaches the user instead of being
 * eaten by a resolution bump. It is also genuinely 16:9 -- the old 240x160
 * forced 3:2 on 16:9 sources, squashing every snapshot. */
export const DEFAULT_RESOLUTION = RESOLUTION_PRESETS[0];

/** The preset matching these dimensions, or `null` when the user has typed
 *  their own — which is what makes the select show "Custom". */
export function matchResolutionPreset(width: number, height: number): ResolutionPreset | null {
  return RESOLUTION_PRESETS.find((p) => p.width === width && p.height === height) ?? null;
}

/* ── Quality steps ───────────────────────────────────────────────────────
 * Easy mode picks one of three; expert mode edits the underlying numbers
 * directly. Presets *write into* those numbers rather than living beside
 * them, so the two can never disagree. */

export interface QualityStep {
  id: "light" | "balanced" | "high";
  snapshotQuality: number;
  opusBitrate: number;
  snapshotWidth: number;
  snapshotHeight: number;
}

export const QUALITY_STEPS: QualityStep[] = [
  { id: "light", snapshotQuality: 60, opusBitrate: 48, snapshotWidth: 256, snapshotHeight: 144 },
  { id: "balanced", snapshotQuality: 80, opusBitrate: 64, snapshotWidth: 426, snapshotHeight: 240 },
  { id: "high", snapshotQuality: 92, opusBitrate: 96, snapshotWidth: 640, snapshotHeight: 360 },
];

export interface VideoQualityStep {
  id: "light" | "balanced" | "high";
  videoBitrate: number;
  videoAudioBitrate: number;
  h264Preset: string;
  width: number;
  height: number;
}

export const VIDEO_QUALITY_STEPS: VideoQualityStep[] = [
  { id: "light", videoBitrate: 400, videoAudioBitrate: 64, h264Preset: "fast", width: 256, height: 144 },
  { id: "balanced", videoBitrate: 800, videoAudioBitrate: 128, h264Preset: "medium", width: 426, height: 240 },
  { id: "high", videoBitrate: 1500, videoAudioBitrate: 192, h264Preset: "medium", width: 640, height: 360 },
];

export const DEFAULT_QUALITY_STEP = QUALITY_STEPS[0];
export const DEFAULT_VIDEO_QUALITY_STEP = VIDEO_QUALITY_STEPS[0];

export function matchQualityStep(
  snapshotQuality: number,
  snapshotWidth?: number,
  snapshotHeight?: number,
): QualityStep | null {
  return (
    QUALITY_STEPS.find(
      (s) =>
        s.snapshotQuality === snapshotQuality &&
        (snapshotWidth === undefined || s.snapshotWidth === snapshotWidth) &&
        (snapshotHeight === undefined || s.snapshotHeight === snapshotHeight)
    ) ?? null
  );
}

export function matchVideoQualityStep(
  videoBitrate: number,
  width?: number,
  height?: number,
  videoAudioBitrate?: number,
  h264Preset?: string,
): VideoQualityStep | null {
  return (
    VIDEO_QUALITY_STEPS.find(
      (s) =>
        s.videoBitrate === videoBitrate &&
        (width === undefined || s.width === width) &&
        (height === undefined || s.height === height) &&
        (videoAudioBitrate === undefined || s.videoAudioBitrate === videoAudioBitrate) &&
        (h264Preset === undefined || s.h264Preset === h264Preset)
    ) ?? null
  );
}

/** Bitrates offered for Opus. Speech needs far less than the MP3 ladder
 *  suggests, so reusing 128/192/256 there would just waste space. */
export const OPUS_BITRATES = [32, 48, 64, 96, 128];
export const MP3_BITRATES = [64, 96, 128, 192, 256];

export function bitratesFor(format: AudioFormat): number[] {
  return format === "opus" ? OPUS_BITRATES : MP3_BITRATES;
}

/** Sensible bitrate when switching format: 128k MP3 and 128k Opus are not the
 *  same trade-off, so carry the *intent* across rather than the number. */
export function equivalentBitrate(from: AudioFormat, to: AudioFormat, bitrate: number): number {
  if (from === to) return bitrate;
  const source = bitratesFor(from);
  const target = bitratesFor(to);
  const rank = source.indexOf(bitrate);
  return rank >= 0 ? target[rank] : (to === "opus" ? 64 : 128);
}

/** Human-readable byte count for the generation result. */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
