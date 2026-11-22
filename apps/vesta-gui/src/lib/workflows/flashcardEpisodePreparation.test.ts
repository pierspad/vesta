import { expect, it, vi } from "vitest";
import { defaultMediaSettings } from "$lib/utils/mediaSettings";
import { predefinedNoteTypeForLanguage } from "$lib/types/noteTypes";
import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";
import { prepareFlashcardEpisodeConfig } from "./flashcardEpisodePreparation";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => { resolve = res; });
  return { promise, resolve };
}

it("snapshots the complete episode config before asynchronous track discovery", async () => {
  const episode: EpisodeEntry = {
    id: 1, targetSubsPath: "episode.it.srt", nativeSubsPath: "native.srt", mediaPath: "episode.mkv", mediaType: "video",
  };
  const mediaSettings = { ...defaultMediaSettings, audioBitrate: 128 };
  let preferredLanguage = "it";
  let pickedLanguage = "";
  const track = deferred<number | null>();
  const filters = { enabled: true, minChars: 3, minCharsEnabled: true, maxChars: 100, maxCharsEnabled: false, minDurationMs: 500, minDurationEnabled: false, maxDurationMs: 15000, maxDurationEnabled: false, combineSentences: false, continuationChars: "..." };
  const noteType = predefinedNoteTypeForLanguage("");

  const preparation = prepareFlashcardEpisodeConfig({
    episode,
    episodeNumber: 1,
    outputDir: "/out",
    seriesOutputMode: "single",
    deckName: "Deck",
    exportFormat: "apkg",
    cpuCores: 2,
    targetLanguage: "en",
    cardFilters: filters,
    videoHwAccel: "auto",
    activeNoteType: noteType,
    autoCardFont: false,
    embedCardFont: false,
    getMediaSettings: vi.fn(() => ({ ...mediaSettings })),
    getAudioTrackLanguage: () => preferredLanguage,
    pickAudioTrackIndex: (_episode, language) => {
      pickedLanguage = language;
      episode.targetSubsPath = "changed.es.srt";
      episode.nativeSubsPath = "changed-native.srt";
      episode.mediaPath = "changed.mp4";
      episode.mediaType = "audio";
      episode.mediaOverrides = { audioTrackIndex: 9 };
      filters.minChars = 20;
      noteType.name = "Changed note type";
      noteType.fields.expression = "ChangedExpression";
      mediaSettings.audioBitrate = 320;
      preferredLanguage = "ja";
      return track.promise;
    },
  });

  track.resolve(4);
  const config = await preparation;
  expect(pickedLanguage).toBe("it");
  expect(config).toMatchObject({
    output_dir: "/out/episodes/episode-1",
    audio_bitrate: 128,
    audio_track_index: 4,
    video_path: "episode.mkv",
    audio_path: null,
    target_subs_path: "episode.it.srt",
    native_subs_path: "native.srt",
    deck_name: "Deck",
    note_type_name: "Default_Vesta",
    field_names: expect.objectContaining({ expression: "Expression" }),
    filters: expect.objectContaining({ min_chars: 3 }),
  });
});
