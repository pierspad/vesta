import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";

export interface TranscriptionProgress { stage: string; message: string; percentage: number }
export interface TranscriptionSegment { start_ms: number; end_ms: number; text: string }
export type TranscriptionListen = <T>(name: string, handler: EventCallback<T>) => Promise<UnlistenFn>;

/** Owns both native subscriptions, including registrations that settle after unmount. */
export function subscribeTranscriptionEvents(options: {
  acceptingResults: () => boolean;
  downloadingModel: () => boolean;
  onProgress: (progress: TranscriptionProgress) => void;
  onDownload: (progress: TranscriptionProgress) => void;
  onSegment: (segment: TranscriptionSegment) => void;
  onError?: (error: unknown) => void;
}, subscribe: TranscriptionListen = listen): () => void {
  let active = true;
  const cleanups: UnlistenFn[] = [];
  function register<T>(name: string, handler: EventCallback<T>) {
    subscribe<T>(name, event => { if (active) handler(event); })
      .then(cleanup => { if (active) cleanups.push(cleanup); else cleanup(); })
      .catch(options.onError ?? console.error);
  }
  register<TranscriptionProgress>("transcribe-progress", event => {
    const progress = event.payload;
    if (progress.stage === "download" && options.downloadingModel()) {
      options.onDownload(progress);
      return;
    }
    if (!options.acceptingResults() || progress.stage === "download") return;
    options.onProgress(progress);
  });
  register<TranscriptionSegment>("transcribe-segment", event => {
    if (options.acceptingResults()) options.onSegment(event.payload);
  });
  return () => {
    if (!active) return;
    active = false;
    for (const cleanup of cleanups.splice(0)) cleanup();
  };
}
