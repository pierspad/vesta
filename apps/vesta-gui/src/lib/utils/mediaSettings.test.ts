import { describe, expect, it } from "vitest";
import {
  defaultMediaSettings,
  readLegacyDimensions,
  sanitizeMediaSettings,
} from "./mediaSettings";
import {
  bitratesFor,
  equivalentBitrate,
  matchQualityStep,
  matchVideoQualityStep,
} from "$lib/types/flashcardMediaTypes";

describe("sanitizeMediaSettings", () => {
  it("returns the defaults for an empty or non-object blob", () => {
    expect(sanitizeMediaSettings(undefined)).toEqual(defaultMediaSettings);
    expect(sanitizeMediaSettings(null)).toEqual(defaultMediaSettings);
    expect(sanitizeMediaSettings("nonsense")).toEqual(defaultMediaSettings);
    expect(sanitizeMediaSettings({})).toEqual(defaultMediaSettings);
  });

  it("keeps valid values untouched", () => {
    const s = sanitizeMediaSettings({
      ...defaultMediaSettings,
      snapshotFormat: "avif",
      snapshotQuality: 42,
      audioFormat: "opus",
      audioBitrate: 64,
    });
    expect(s.snapshotFormat).toBe("avif");
    expect(s.snapshotQuality).toBe(42);
    expect(s.audioFormat).toBe("opus");
    expect(s.audioBitrate).toBe(64);
  });

  it("clamps out-of-range numbers instead of accepting them", () => {
    const s = sanitizeMediaSettings({
      snapshotQuality: 900,
      snapshotWidth: -5,
      cropBottom: 99999,
      audioPadStart: -100,
    });
    expect(s.snapshotQuality).toBe(100);
    expect(s.snapshotWidth).toBe(16);
    expect(s.cropBottom).toBe(4320);
    expect(s.audioPadStart).toBe(0);
  });

  it("falls back per field, so one bad value does not discard the rest", () => {
    const s = sanitizeMediaSettings({
      snapshotFormat: "tiff",
      snapshotQuality: 55,
      audioFormat: "flac",
      audioBitrate: 96,
    });
    expect(s.snapshotFormat).toBe(defaultMediaSettings.snapshotFormat);
    expect(s.audioFormat).toBe(defaultMediaSettings.audioFormat);
    expect(s.snapshotQuality).toBe(55);
    expect(s.audioBitrate).toBe(96);
  });

  it("drops unknown keys rather than passing them to the backend", () => {
    const s = sanitizeMediaSettings({ somethingNew: true, snapshotQuality: 70 });
    expect(s).not.toHaveProperty("somethingNew");
    expect(Object.keys(s).sort()).toEqual(Object.keys(defaultMediaSettings).sort());
  });

  it("treats a null audio track as 'auto' and never as index 0", () => {
    expect(sanitizeMediaSettings({ audioTrackIndex: null }).audioTrackIndex).toBeNull();
    expect(sanitizeMediaSettings({}).audioTrackIndex).toBeNull();
    expect(sanitizeMediaSettings({ audioTrackIndex: 2 }).audioTrackIndex).toBe(2);
  });

  it("recovers numbers stored as strings", () => {
    expect(sanitizeMediaSettings({ snapshotWidth: "640" }).snapshotWidth).toBe(640);
  });
});

describe("readLegacyDimensions", () => {
  it("carries the pre-blob snapshot size across the upgrade", () => {
    const store: Record<string, string> = {
      "vesta-flashcards-media-width": "240",
      "vesta-flashcards-media-height": "160",
    };
    expect(readLegacyDimensions((k) => store[k] ?? null)).toEqual({
      snapshotWidth: 240,
      snapshotHeight: 160,
    });
  });

  it("ignores missing or malformed legacy keys", () => {
    expect(readLegacyDimensions(() => null)).toEqual({});
    expect(readLegacyDimensions(() => "abc")).toEqual({});
    expect(readLegacyDimensions(() => "0")).toEqual({});
  });
});

describe("resolution and quality presets", () => {
  it("maps a quality number and dimensions back to its step", () => {
    expect(matchQualityStep(80)?.id).toBe("balanced");
    expect(matchQualityStep(60)?.id).toBe("light");
    expect(matchQualityStep(80, 426, 240)?.id).toBe("balanced");
    expect(matchQualityStep(80, 854, 480)).toBeNull();
    expect(matchQualityStep(81)).toBeNull();
  });

  it("maps video quality settings back to video quality step", () => {
    expect(matchVideoQualityStep(800, 426, 240)?.id).toBe("balanced");
    expect(matchVideoQualityStep(400, 256, 144)?.id).toBe("light");
    expect(matchVideoQualityStep(1500, 640, 360)?.id).toBe("high");
    expect(matchVideoQualityStep(800, 854, 480)).toBeNull();
    expect(matchVideoQualityStep(999, 426, 240)).toBeNull();
  });
});

describe("audio bitrate ladders", () => {
  it("offers speech-appropriate rates for Opus, not the MP3 ladder", () => {
    expect(bitratesFor("opus")).toContain(48);
    expect(bitratesFor("opus")).not.toContain(256);
    expect(bitratesFor("mp3")).toContain(128);
  });

  it("carries the quality intent across a format switch, not the number", () => {
    // 128k MP3 is the middle of its ladder; the Opus equivalent is 64k.
    expect(equivalentBitrate("mp3", "opus", 128)).toBe(64);
    expect(equivalentBitrate("opus", "mp3", 64)).toBe(128);
    expect(equivalentBitrate("mp3", "mp3", 192)).toBe(192);
  });

  it("falls back to a sane rate for a value outside either ladder", () => {
    expect(equivalentBitrate("mp3", "opus", 111)).toBe(64);
    expect(equivalentBitrate("opus", "mp3", 111)).toBe(128);
  });
});
