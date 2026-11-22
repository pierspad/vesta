import type { AudioTrackInfo, EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";

interface EditableEpisode {
  mediaPath: string;
  mediaType: "none" | "video" | "audio";
  /** Needed to infer the episode's audio-track language preference. */
  targetSubsPath: string;
}

/**
 * Transient state for the "per-episode media settings" dialog (series mode):
 * which episode is being edited, its working copy of overrides, and the
 * audio-track list fetched for it. Ownership of the `episodes` array itself
 * stays in FlashcardsTab.svelte — this store only holds what's being edited
 * right now, mirroring previewStore's split (dialog state here, domain data
 * and its persistence in the parent).
 */
class EpisodeMediaEditorStore {
  episodeIndex = $state<number | null>(null);
  episode = $state<EditableEpisode | null>(null);
  overrides = $state<Required<EpisodeMediaOverrides> | null>(null);
  audioTracks = $state<AudioTrackInfo[]>([]);
  audioTracksLoading = $state(false);

  private initialOverridesStr = $state("");

  isDirty = $derived(this.overrides !== null && JSON.stringify(this.overrides) !== this.initialOverridesStr);

  begin(idx: number, episode: EditableEpisode, overrides: Required<EpisodeMediaOverrides>) {
    this.episodeIndex = idx;
    this.episode = episode;
    this.audioTracks = [];
    this.audioTracksLoading = episode.mediaType === "video";
    this.overrides = overrides;
    this.initialOverridesStr = "";
  }

  setAudioTracks(tracks: AudioTrackInfo[]) {
    this.audioTracks = tracks;
    this.audioTracksLoading = false;
  }

  /** Call once any auto-picked defaults (e.g. audio track) have settled, so
   * `isDirty` compares against the right baseline instead of flagging the
   * dialog as dirty the instant it opens. */
  captureBaseline() {
    if (this.overrides) this.initialOverridesStr = JSON.stringify(this.overrides);
  }

  update<K extends keyof EpisodeMediaOverrides>(key: K, value: EpisodeMediaOverrides[K]) {
    if (!this.overrides) return;
    this.overrides = { ...this.overrides, [key]: value };
  }

  close() {
    this.episodeIndex = null;
    this.episode = null;
    this.overrides = null;
    this.audioTracks = [];
    this.audioTracksLoading = false;
    this.initialOverridesStr = "";
  }
}

export const episodeMediaEditorStore = new EpisodeMediaEditorStore();
