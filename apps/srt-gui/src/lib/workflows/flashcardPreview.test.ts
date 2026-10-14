import { expect, it, vi } from "vitest";
import { FlashcardPreviewController, type FlashcardPreviewOptions } from "./flashcardPreview";
import type { PreviewLine } from "$lib/stores/previewStore.svelte";

const lines = [
  { index: 1, start_ms: 0, end_ms: 1000, subs1_text: "One", subs2_text: null, active: true, leading_context: [], trailing_context: [] },
  { index: 2, start_ms: 1000, end_ms: 2000, subs1_text: "Two", subs2_text: null, active: false, leading_context: [], trailing_context: [] },
] satisfies PreviewLine[];

function options(): FlashcardPreviewOptions {
  return {
    isBusy: () => false,
    canPreview: () => true,
    isLoading: () => false,
    buildConfig: vi.fn(() => ({ target_subs_path: "episode.srt" }) as ReturnType<FlashcardPreviewOptions["buildConfig"]>),
    load: vi.fn(async (_config, onLoaded) => onLoaded(lines)),
    close: vi.fn(),
    setError: vi.fn(),
    missingRequirementsMessage: () => "Required fields are missing",
    addLog: vi.fn(),
  };
}

it("guards busy, loading, and incomplete requests before building a config", async () => {
  for (const guard of ["busy", "loading"] as const) {
    const deps = options();
    if (guard === "busy") deps.isBusy = () => true;
    else deps.isLoading = () => true;
    await new FlashcardPreviewController(deps).load();
    expect(deps.buildConfig).not.toHaveBeenCalled();
    expect(deps.load).not.toHaveBeenCalled();
  }

  const invalid = options();
  invalid.canPreview = () => false;
  await new FlashcardPreviewController(invalid).load();
  expect(invalid.setError).toHaveBeenCalledExactlyOnceWith("Required fields are missing");
  expect(invalid.buildConfig).not.toHaveBeenCalled();
});

it("clears the prior error, loads the built config, and logs active counts", async () => {
  const deps = options();
  await new FlashcardPreviewController(deps).load();

  expect(deps.setError).toHaveBeenCalledExactlyOnceWith(null);
  expect(deps.load).toHaveBeenCalledWith({ target_subs_path: "episode.srt" }, expect.any(Function));
  expect(deps.addLog).toHaveBeenCalledExactlyOnceWith("Preview: 2 total, 1 active", "info");
});

it("surfaces a current preview failure to the tab", async () => {
  const deps = options();
  deps.load = vi.fn().mockRejectedValue(new Error("bad subtitles"));
  await new FlashcardPreviewController(deps).load();
  expect(deps.setError).toHaveBeenLastCalledWith("Preview error: Error: bad subtitles");
});

it("disposal closes the owned preview and suppresses late logs and errors", async () => {
  const deps = options();
  let rejectLoad!: (error: Error) => void;
  let onLoaded!: (loaded: PreviewLine[]) => void;
  deps.load = vi.fn((_config, callback) => {
    onLoaded = callback;
    return new Promise<void>((_resolve, reject) => { rejectLoad = reject; });
  });
  const controller = new FlashcardPreviewController(deps);
  const pending = controller.load();
  controller.dispose();
  controller.dispose();
  onLoaded(lines);
  rejectLoad(new Error("late failure"));
  await pending;

  expect(deps.close).toHaveBeenCalledOnce();
  expect(deps.addLog).not.toHaveBeenCalled();
  expect(deps.setError).toHaveBeenCalledExactlyOnceWith(null);
});

it("does not start work after disposal", async () => {
  const deps = options();
  const controller = new FlashcardPreviewController(deps);
  controller.dispose();
  await controller.load();
  expect(deps.buildConfig).not.toHaveBeenCalled();
  expect(deps.load).not.toHaveBeenCalled();
});
