/** Per-episode overrides for the movie-mode media settings (audio/snapshot/
 * video generation). Every field mirrors one primitive of `mediaSettings` in
 * FlashcardsTab.svelte — an episode only stores the keys that actually
 * differ from the generic settings (see buildEpisodeMediaOverrideDiff). */
export interface EpisodeMediaOverrides {
  generateAudio?: boolean;
  audioBitrate?: number;
  audioTrackIndex?: number | null;
  normalizeAudio?: boolean;
  audioPadStart?: number;
  audioPadEnd?: number;
  generateSnapshots?: boolean;
  snapshotWidth?: number;
  snapshotHeight?: number;
  cropBottom?: number;
  generateVideoClips?: boolean;
  videoCodec?: string;
  h264Preset?: string;
  videoBitrate?: number;
  videoAudioBitrate?: number;
  videoPadStart?: number;
  videoPadEnd?: number;
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
