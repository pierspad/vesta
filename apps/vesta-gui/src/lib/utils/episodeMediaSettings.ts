import type { EpisodeMediaOverrides, EpisodeMediaOverrideKey } from "$lib/types/flashcardMediaTypes";
export const audioOverrideKeys: EpisodeMediaOverrideKey[] = [
    "generateAudio",
    "audioBitrate",
    "audioTrackIndex",
    "normalizeAudio",
    "audioBoost",
    "audioGainDb",
    "audioPadStart",
    "audioPadEnd",
  ];
export const snapshotOverrideKeys: EpisodeMediaOverrideKey[] = [
    "generateSnapshots",
    "snapshotWidth",
    "snapshotHeight",
    "cropBottom",
    // Codec choice is deck-wide (a deck mixing webp and jpg helps nobody);
    // quality and resolution are per-episode, since source quality varies.
    "snapshotQuality",
  ];
export const videoOverrideKeys: EpisodeMediaOverrideKey[] = [
    "generateVideoClips",
    "videoCodec",
    "h264Preset",
    "videoBitrate",
    "videoAudioBitrate",
    "videoPadStart",
    "videoPadEnd",
    "videoWidth",
    "videoHeight",
  ];


export function mediaSettingChanged(key: EpisodeMediaOverrideKey, settings: Required<EpisodeMediaOverrides>, defaults: Required<EpisodeMediaOverrides>, autoAudioTrack?: number | null): boolean {
  if (key === "audioTrackIndex" && defaults.audioTrackIndex === null && autoAudioTrack !== undefined && settings.audioTrackIndex === autoAudioTrack) return false;
  return settings[key] !== defaults[key];
}
export function episodeMediaDiff(settings: Required<EpisodeMediaOverrides>, defaults: Required<EpisodeMediaOverrides>, autoAudioTrack?: number | null): EpisodeMediaOverrides {
  const diff: EpisodeMediaOverrides = {};
  for (const key of [...audioOverrideKeys, ...snapshotOverrideKeys, ...videoOverrideKeys]) {
    if (mediaSettingChanged(key, settings, defaults, autoAudioTrack)) diff[key] = settings[key] as never;
  }
  return diff;
}
