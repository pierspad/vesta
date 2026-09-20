import { describe, it, expect, vi } from "vitest";

// Mock vestaConfig used by loadCardTemplates
const storage = new Map<string, string>();
vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => {
    storage.set(key, val);
  },
  removeItem: (key: string) => {
    storage.delete(key);
  },
}));

import { buildFlashcardConfig, type FlashcardConfigInputs } from "./flashcardConfig";
import { defaultMediaSettings } from "./mediaSettings";
import { predefinedNoteTypeForLanguage } from "$lib/types/noteTypes";

describe("buildFlashcardConfig", () => {
  const baseInputs: FlashcardConfigInputs = {
    targetSubsPath: "/path/to/target.srt",
    nativeSubsPath: "/path/to/native.srt",
    videoPath: "/path/to/video.mp4",
    audioPath: null,
    outputDir: "/path/to/output",
    cardFilters: {
      enabled: false,
      minChars: 3,
      minCharsEnabled: true,
      maxChars: 100,
      maxCharsEnabled: true,
      minDurationMs: 500,
      minDurationEnabled: true,
      maxDurationMs: 15000,
      maxDurationEnabled: true,
      combineSentences: true,
      continuationChars: "...",
    },
    media: {
      ...defaultMediaSettings,
      snapshotWidth: 640,
      snapshotHeight: 360,
      videoWidth: 426,
      videoHeight: 240,
    },
    generateAudio: true,
    generateSnapshots: true,
    generateVideoClips: false,
    audioTrackIndex: null,
    videoHwAccel: "auto",
    deckName: "SampleDeck",
    episodeNumber: 1,
    exportFormat: "apkg",
    noteType: predefinedNoteTypeForLanguage(""),
    cpuCores: 4,
    targetLanguage: "zh",
    autoCardFont: true,
  };

  it("builds a complete config matching domain invariants", () => {
    const config = buildFlashcardConfig(baseInputs);

    expect(config.target_subs_path).toBe("/path/to/target.srt");
    expect(config.native_subs_path).toBe("/path/to/native.srt");
    expect(config.video_path).toBe("/path/to/video.mp4");
    expect(config.output_dir).toBe("/path/to/output");
    expect(config.generate_audio).toBe(true);
    expect(config.generate_snapshots).toBe(true);
    expect(config.generate_video_clips).toBe(false);
    expect(config.deck_name).toBe("SampleDeck");
    expect(config.target_language).toBe("zh");
    expect(config.auto_card_font).toBe(true);
  });

  it("decouples snapshot dimensions from video dimensions", () => {
    const config = buildFlashcardConfig(baseInputs);

    expect(config.snapshot_width).toBe(640);
    expect(config.snapshot_height).toBe(360);
    expect(config.video_width).toBe(426);
    expect(config.video_height).toBe(240);
  });

  it("respects cardFilters.enabled master switch", () => {
    // When master filter switch is disabled, filter thresholds should be null
    const configDisabled = buildFlashcardConfig({
      ...baseInputs,
      cardFilters: {
        ...baseInputs.cardFilters,
        enabled: false,
      },
    });

    expect(configDisabled.filters.min_chars).toBeNull();
    expect(configDisabled.filters.max_chars).toBeNull();
    expect(configDisabled.filters.min_duration_ms).toBeNull();
    expect(configDisabled.filters.max_duration_ms).toBeNull();
    expect(configDisabled.combine_sentences).toBe(false);

    // When enabled, active sub-filters should pass their numerical thresholds
    const configEnabled = buildFlashcardConfig({
      ...baseInputs,
      cardFilters: {
        ...baseInputs.cardFilters,
        enabled: true,
      },
    });

    expect(configEnabled.filters.min_chars).toBe(3);
    expect(configEnabled.filters.max_chars).toBe(100);
    expect(configEnabled.filters.min_duration_ms).toBe(500);
    expect(configEnabled.filters.max_duration_ms).toBe(15000);
    expect(configEnabled.combine_sentences).toBe(true);
  });

  it("handles nullable video/audio paths gracefully", () => {
    const noMedia = buildFlashcardConfig({
      ...baseInputs,
      videoPath: null,
      audioPath: null,
      generateSnapshots: false,
      generateAudio: false,
    });

    expect(noMedia.video_path).toBeNull();
    expect(noMedia.audio_path).toBeNull();
    expect(noMedia.generate_snapshots).toBe(false);
    expect(noMedia.generate_audio).toBe(false);
  });

  it("generates correct note type output fields mapping", () => {
    const defaultNote = buildFlashcardConfig({
      ...baseInputs,
      noteType: predefinedNoteTypeForLanguage(""),
    });
    expect(defaultNote.output_fields.include_subs1).toBe(true);
    expect(defaultNote.output_fields.include_audio).toBe(true);
    expect(defaultNote.output_fields.include_snapshot).toBe(true);
  });
});
