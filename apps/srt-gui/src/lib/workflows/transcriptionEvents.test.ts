import { expect, it, vi } from "vitest";
import { subscribeTranscriptionEvents, type TranscriptionListen } from "./transcriptionEvents";
function setup(late = false) {
  const handlers = new Map<string, (event: any) => void>();
  const cleanups = [vi.fn(), vi.fn()];
  const pending: Array<() => void> = [];
  const subscribe: TranscriptionListen = (name, handler) => {
    handlers.set(name, handler);
    const cleanup = cleanups[handlers.size - 1];
    return late ? new Promise(resolve => { pending.push(() => resolve(cleanup)); }) : Promise.resolve(cleanup);
  };
  let running = true, downloading = false;
  const options = { acceptingResults: () => running, downloadingModel: () => downloading,
    onProgress: vi.fn(), onDownload: vi.fn(), onSegment: vi.fn(), onError: vi.fn() };
  const dispose = subscribeTranscriptionEvents(options, subscribe);
  return { options, handlers, dispose, cleanups, pending,
    running: (v: boolean) => { running = v; }, downloading: (v: boolean) => { downloading = v; },
    emit: (name: string, payload: unknown) => handlers.get(name)!({ payload }),
  };
}
it("routes download progress separately and accepts results only for the current run", () => {
  const s = setup(); const progress = { stage: "transcribe", percentage: 50, message: "working" };
  s.emit("transcribe-progress", progress); expect(s.options.onProgress).toHaveBeenCalledWith(progress);
  s.emit("transcribe-progress", { ...progress, stage: "download" }); expect(s.options.onProgress).toHaveBeenCalledOnce();
  s.running(false); s.downloading(true);
  s.emit("transcribe-progress", { ...progress, stage: "download" }); expect(s.options.onDownload).toHaveBeenCalledOnce();
  s.emit("transcribe-segment", { text: "late" }); expect(s.options.onSegment).not.toHaveBeenCalled();
});
it("disposes both subscriptions once and rejects events after disposal", async () => {
  const s = setup(); await Promise.resolve(); s.dispose(); s.dispose();
  expect(s.cleanups.every(fn => fn.mock.calls.length === 1)).toBe(true);
  s.emit("transcribe-segment", { text: "late" }); expect(s.options.onSegment).not.toHaveBeenCalled();
});
it("cleans registrations that finish after unmount", async () => {
  const s = setup(true); s.dispose(); s.pending.forEach(resolve => resolve()); await Promise.resolve();
  expect(s.cleanups.every(fn => fn.mock.calls.length === 1)).toBe(true);
});
it("reports registration failures without preventing cleanup of another subscription", async () => {
  const cleanup = vi.fn(); const error = vi.fn(); let count = 0;
  const dispose = subscribeTranscriptionEvents({ acceptingResults: () => true, downloadingModel: () => false,
    onProgress: vi.fn(), onDownload: vi.fn(), onSegment: vi.fn(), onError: error,
  }, () => ++count === 1 ? Promise.reject(new Error("listen failed")) : Promise.resolve(cleanup));
  await Promise.resolve(); await Promise.resolve(); dispose();
  expect(error).toHaveBeenCalledOnce(); expect(cleanup).toHaveBeenCalledOnce();
});
