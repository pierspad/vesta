import type { AudioTrackInfo } from "$lib/types/flashcardMediaTypes";
import { detectMediaType, pickBestAudioTrackIndex } from "$lib/utils/seriesFileMatching";
import { getFileName } from "$lib/utils/models";

/** Owns the current media probe; late results cannot change another file's selection. */
export class FlashcardAudioSelection {
  tracks = $state<AudioTrackInfo[]>([]);
  loading = $state(false);
  automatic = $state(true);
  private request = 0;
  private disposed = false;

  constructor(
    private readonly probe: (path: string) => Promise<AudioTrackInfo[]>,
    private readonly language: () => string,
    private readonly select: (index: number | null) => void,
    private readonly onError: (error: unknown) => void,
  ) {}

  reset() {
    this.request += 1;
    this.tracks = [];
    this.loading = false;
    this.automatic = true;
    this.select(null);
  }

  async load(path: string) {
    if (this.disposed) return;
    this.reset();
    const request = this.request;
    if (detectMediaType(getFileName(path)) !== "video") return;
    this.loading = true;
    try {
      const tracks = await this.probe(path);
      if (this.disposed || request !== this.request) return;
      this.tracks = tracks;
      this.select(tracks.length > 1 ? pickBestAudioTrackIndex(tracks, this.language()) : null);
    } catch (error) {
      if (!this.disposed && request === this.request) this.onError(error);
    } finally {
      if (!this.disposed && request === this.request) this.loading = false;
    }
  }

  updateLanguage() {
    if (!this.disposed && this.automatic && this.tracks.length > 1) {
      this.select(pickBestAudioTrackIndex(this.tracks, this.language()));
    }
  }

  dispose() {
    this.disposed = true;
    this.request += 1;
  }
}
