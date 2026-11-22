import { describe, it, expect, beforeEach } from "vitest";
import { episodeMediaEditorStore } from "./episodeMediaEditorStore.svelte";
import { defaultMediaSettings } from "$lib/utils/mediaSettings";

describe("episodeMediaEditorStore", () => {
  beforeEach(() => {
    episodeMediaEditorStore.close();
  });

  it("begins session and initializes state", () => {
    const episode = {
      mediaPath: "/path/to/ep01.mp4",
      mediaType: "video" as const,
      targetSubsPath: "/path/to/ep01.srt",
    };

    episodeMediaEditorStore.begin(0, episode, { ...defaultMediaSettings });

    expect(episodeMediaEditorStore.episodeIndex).toBe(0);
    expect(episodeMediaEditorStore.episode).toEqual(episode);
    expect(episodeMediaEditorStore.audioTracksLoading).toBe(true);
    expect(episodeMediaEditorStore.overrides).toEqual(defaultMediaSettings);
  });

  it("tracks dirty state after capturing baseline", () => {
    const episode = {
      mediaPath: "/path/to/ep01.mp4",
      mediaType: "video" as const,
      targetSubsPath: "/path/to/ep01.srt",
    };

    episodeMediaEditorStore.begin(0, episode, { ...defaultMediaSettings });
    episodeMediaEditorStore.captureBaseline();
    expect(episodeMediaEditorStore.isDirty).toBe(false);

    episodeMediaEditorStore.update("videoBitrate", 1200);
    expect(episodeMediaEditorStore.isDirty).toBe(true);
    expect(episodeMediaEditorStore.overrides?.videoBitrate).toBe(1200);

    // Reverting back restores non-dirty state
    episodeMediaEditorStore.update("videoBitrate", defaultMediaSettings.videoBitrate);
    expect(episodeMediaEditorStore.isDirty).toBe(false);
  });

  it("sets audio tracks and turns off loading", () => {
    const tracks = [
      {
        index: 0,
        stream_index: 1,
        codec: "aac",
        language: "jpn",
        title: "Japanese Audio",
        channels: 2,
      },
    ];

    episodeMediaEditorStore.setAudioTracks(tracks);
    expect(episodeMediaEditorStore.audioTracks).toEqual(tracks);
    expect(episodeMediaEditorStore.audioTracksLoading).toBe(false);
  });

  it("cleans up state on close", () => {
    episodeMediaEditorStore.begin(
      0,
      { mediaPath: "/path", mediaType: "video", targetSubsPath: "/path.srt" },
      { ...defaultMediaSettings },
    );
    episodeMediaEditorStore.close();

    expect(episodeMediaEditorStore.episodeIndex).toBeNull();
    expect(episodeMediaEditorStore.episode).toBeNull();
    expect(episodeMediaEditorStore.overrides).toBeNull();
    expect(episodeMediaEditorStore.audioTracks).toEqual([]);
    expect(episodeMediaEditorStore.audioTracksLoading).toBe(false);
  });
});
