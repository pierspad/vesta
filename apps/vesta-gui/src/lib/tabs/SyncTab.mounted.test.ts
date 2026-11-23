// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import SyncTab from "./SyncTab.svelte";
import SyncTabActiveHarness from "$lib/test/fixtures/SyncTabActiveHarness.svelte";
import { whisperModelsStore } from "$lib/stores/whisperModelsStore.svelte";
import { aiStore } from "$lib/stores/aiStore.svelte";
import { invokeCommand } from "$lib/services/tauriClient";
import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
import { snackbar } from "$lib/stores/snackbarStore.svelte";

const mocks = vi.hoisted(() => ({
  onDrop: undefined as ((paths: string[]) => void) | undefined,
}));

vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn().mockResolvedValue([]) }));
vi.mock("$lib/utils/dragDrop", () => ({
  setupWebviewDragDrop: vi.fn((options: { onDrop: (paths: string[]) => void }) => {
    mocks.onDrop = options.onDrop;
    return () => { mocks.onDrop = undefined; };
  }),
}));
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn(), guardedSave: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

let previousScrollIntoView: typeof HTMLElement.prototype.scrollIntoView | undefined;

afterEach(() => {
  vi.restoreAllMocks();
  vi.mocked(invokeCommand).mockReset();
  vi.mocked(invokeCommand).mockResolvedValue([]);
  mocks.onDrop = undefined;
  if (previousScrollIntoView) {
    HTMLElement.prototype.scrollIntoView = previousScrollIntoView;
    previousScrollIntoView = undefined;
  }
  document.body.replaceChildren();
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const makeStatus = (overrides: Record<string, unknown> = {}) => ({
  is_loaded: true,
  srt_path: "film.srt",
  video_path: null,
  total_subtitles: 5,
  anchor_count: 1,
  checked_count: 1,
  completion_percentage: 20,
  average_offset_ms: 0,
  suggested_next_id: 1,
  can_undo: true,
  can_redo: false,
  ...overrides,
});

const makeSubtitle = (id = 1) => ({
  id,
  start_ms: 1000,
  end_ms: 2000,
  text: "Hello",
  synced_start_ms: 1000,
  synced_end_ms: 2000,
  offset_ms: 0,
  is_anchor: false,
});

const makeAnchor = (subtitleId = 1) => ({
  subtitle_id: subtitleId,
  original_time_ms: 1000,
  corrected_time_ms: 1000,
  offset_ms: 0,
  is_manual: true,
});

const defaultSubtitles = () => [1, 2, 3, 4, 5].map((id) => makeSubtitle(id));

function setupSyncInvokeMock(overrides: Partial<Record<string, () => unknown>> = {}) {
  const defaults: Record<string, () => unknown> = {
    transcribe_list_models: () => [],
    sync_load_srt: () => makeStatus(),
    sync_get_subtitles_range: () => defaultSubtitles(),
    sync_get_anchors: () => [],
    sync_suggest_media_for_srt: () => null,
    sync_prepare_media_for_playback: () => "/tmp/film.mp3",
    get_media_server_info: () => [4567, "test-token"],
    sync_set_video: () => makeStatus({ video_path: "film.mp3" }),
    sync_get_subtitle: () => makeSubtitle(1),
    sync_undo: () => makeStatus({ can_undo: true, can_redo: true }),
    ...overrides,
  };
  vi.mocked(invokeCommand).mockImplementation(async (command: string) => {
    const handler = defaults[command];
    if (!handler) throw new Error(`Unexpected command: ${command}`);
    return handler();
  });
}

async function flushAsyncAndTimers(ms = 75) {
  await Promise.resolve();
  await tick();
  await new Promise((resolve) => setTimeout(resolve, ms));
  await tick();
}

async function mountActiveHarness() {
  vi.spyOn(whisperModelsStore, "refreshModels").mockResolvedValue();
  previousScrollIntoView = HTMLElement.prototype.scrollIntoView;
  const scrollIntoViewSpy = vi.fn();
  HTMLElement.prototype.scrollIntoView = scrollIntoViewSpy;

  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(SyncTabActiveHarness, { target, intro: false });
  await tick();
  return { target, component, scrollIntoViewSpy };
}

async function dropSrt(srtPath = "film.srt") {
  mocks.onDrop!([srtPath]);
  await vi.waitFor(() => {
    expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_suggest_media_for_srt", { srtPath });
  });
  await flushAsyncAndTimers(60);
}

async function dropAudio(audioPath = "film.mp3") {
  mocks.onDrop!([audioPath]);
  await vi.waitFor(() => {
    expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_set_video", { path: audioPath });
  });
  await tick();
}

it("consumes model-update notifications without restarting discovery and releases its listeners", async () => {
  const refresh = vi.spyOn(whisperModelsStore, "refreshModels").mockResolvedValue();
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(SyncTab, { target, props: { active: true }, intro: false });
  try {
    await tick();
    expect(refresh).toHaveBeenCalledOnce();
    window.dispatchEvent(new CustomEvent("vesta-whisper-models-updated", { detail: { models: [] } }));
    window.dispatchEvent(new CustomEvent("whisper-model-updated", { detail: "small" }));
    await tick();
    expect(refresh).toHaveBeenCalledOnce();
  } finally {
    await unmount(component);
  }
  vi.mocked(invokeCommand).mockClear();
  window.dispatchEvent(new CustomEvent("vesta-whisper-models-updated"));
  await tick();
  expect(invokeCommand).not.toHaveBeenCalled();
});

it("pauses active media playback when the tab becomes hidden", async () => {
  vi.spyOn(whisperModelsStore, "refreshModels").mockResolvedValue();
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(SyncTabActiveHarness, { target, intro: false });
  try {
    await tick();
    const video = target.querySelector("video")!;
    Object.defineProperty(video, "paused", { value: false, configurable: true });
    const pauseSpy = vi.spyOn(video, "pause").mockImplementation(() => {});

    target.querySelector<HTMLButtonElement>('[data-testid="deactivate-tab"]')!.click();
    await tick();
    expect(pauseSpy).toHaveBeenCalledOnce();
  } finally {
    aiStore.killSwitchActive = false;
    await unmount(component);
  }
});

it("does not play audio or scroll when deactivated while goToCheckpoint is awaiting subtitle load", async () => {
  let pendingSubtitle: ReturnType<typeof deferred<ReturnType<typeof makeSubtitle>>> | null = null;
  let pendingSubtitleRequested = false;

  setupSyncInvokeMock({
    sync_get_subtitle: () => {
      if (pendingSubtitle) {
        pendingSubtitleRequested = true;
        return pendingSubtitle.promise;
      }
      return makeSubtitle(1);
    },
  });

  const { target, component, scrollIntoViewSpy } = await mountActiveHarness();
  try {
    const video = target.querySelector("video")!;
    const playSpy = vi.spyOn(video, "play").mockResolvedValue(undefined);

    await dropSrt("film.srt");
    await dropAudio("film.mp3");
    expect(video.getAttribute("src")).toContain("http://127.0.0.1:");

    // Positive control: when active with audio loaded, goToCheckpoint reaches video.play().
    mocks.onDrop!(["film-active.srt"]);
    await vi.waitFor(() => {
      expect(playSpy).toHaveBeenCalledOnce();
    });
    await flushAsyncAndTimers(60);

    playSpy.mockClear();
    scrollIntoViewSpy.mockClear();

    // Simulate tab deactivation while sync_get_subtitle is still in flight.
    pendingSubtitle = deferred<ReturnType<typeof makeSubtitle>>();
    mocks.onDrop!(["film-race.srt"]);
    await vi.waitFor(() => {
      expect(pendingSubtitleRequested).toBe(true);
    });

    scrollIntoViewSpy.mockClear();
    target.querySelector<HTMLButtonElement>('[data-testid="deactivate-tab"]')!.click();
    await tick();

    pendingSubtitle.resolve(makeSubtitle(1));
    await flushAsyncAndTimers();

    expect(playSpy).not.toHaveBeenCalled();
    expect(scrollIntoViewSpy).not.toHaveBeenCalled();
  } finally {
    aiStore.killSwitchActive = false;
    await unmount(component);
  }
});

it("does not autoplay or scroll if deactivated while goToCheckpoint reloads subtitles", async () => {
  let pendingRange: ReturnType<typeof deferred<ReturnType<typeof makeSubtitle>[]>> | null = null;
  let pendingRangeRequested = false;

  setupSyncInvokeMock({
    sync_get_subtitles_range: () => {
      if (pendingRange) {
        pendingRangeRequested = true;
        return pendingRange.promise;
      }
      return defaultSubtitles();
    },
  });

  const { target, component, scrollIntoViewSpy } = await mountActiveHarness();
  try {
    const video = target.querySelector("video")!;
    const playSpy = vi.spyOn(video, "play").mockResolvedValue(undefined);

    await dropSrt("film.srt");
    await dropAudio("film.mp3");

    playSpy.mockClear();
    scrollIntoViewSpy.mockClear();

    // Trigger goToCheckpoint via Ctrl+G while loadSubtitlesAround's sync_get_subtitles_range is pending.
    pendingRange = deferred<ReturnType<typeof makeSubtitle>[]>();
    target.dispatchEvent(new KeyboardEvent("keydown", {
      key: "g",
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    }));
    await vi.waitFor(() => {
      expect(pendingRangeRequested).toBe(true);
    });

    target.querySelector<HTMLButtonElement>('[data-testid="deactivate-tab"]')!.click();
    await tick();

    pendingRange.resolve(defaultSubtitles());
    await flushAsyncAndTimers();

    expect(playSpy).not.toHaveBeenCalled();
    expect(scrollIntoViewSpy).not.toHaveBeenCalledWith({ behavior: "smooth", block: "center" });
  } finally {
    aiStore.killSwitchActive = false;
    await unmount(component);
  }
});

it("ignores window keydown shortcuts such as Ctrl+Z once the tab is deactivated", async () => {
  setupSyncInvokeMock({
    sync_get_anchors: () => [makeAnchor(1)],
  });

  const { target, component } = await mountActiveHarness();
  try {
    await dropSrt("film.srt");

    // Positive control: Ctrl+Z bubbling to window dispatches sync_undo while active.
    target.dispatchEvent(new KeyboardEvent("keydown", {
      key: "z",
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    }));
    await vi.waitFor(() => {
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_undo");
    });
    await flushAsyncAndTimers(60);

    vi.mocked(invokeCommand).mockClear();
    target.querySelector<HTMLButtonElement>('[data-testid="deactivate-tab"]')!.click();
    await tick();

    const inactiveUndo = new KeyboardEvent("keydown", {
      key: "z",
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    });
    window.dispatchEvent(inactiveUndo);
    await tick();
    await Promise.resolve();

    expect(inactiveUndo.defaultPrevented).toBe(false);
    expect(vi.mocked(invokeCommand).mock.calls.map(([cmd]) => cmd)).not.toContain("sync_undo");
    expect(invokeCommand).not.toHaveBeenCalled();
  } finally {
    aiStore.killSwitchActive = false;
    await unmount(component);
  }
});

it("hides the speech engine selector in SyncFooterBar when AI Kill Switch is active", async () => {
  setupSyncInvokeMock();
  aiStore.killSwitchActive = false;
  const { target, component } = await mountActiveHarness();
  try {
    await flushAsyncAndTimers();
    const selector = 'input[role="combobox"][aria-label="Transcription engine"]';
    expect(target.querySelector(selector)).not.toBeNull();

    aiStore.killSwitchActive = true;
    await flushAsyncAndTimers();
    expect(target.querySelector(selector)).toBeNull();

    aiStore.killSwitchActive = false;
    await flushAsyncAndTimers();
    expect(target.querySelector(selector)).not.toBeNull();
  } finally {
    aiStore.killSwitchActive = false;
    await unmount(component);
  }
});

it("shows the pending media path, disables Browse, and overlays extracting status without removing the slider", async () => {
  let resolvePrepare!: (value: string) => void;
  const preparePromise = new Promise<string>((resolve) => {
    resolvePrepare = resolve;
  });
  setupSyncInvokeMock({
    sync_suggest_media_for_srt: () => "/films/Detour(1945).mp4",
    sync_prepare_media_for_playback: () => preparePromise,
    sync_set_video: () => makeStatus({ video_path: "/films/Detour(1945).mp4" }),
  });

  const { target, component } = await mountActiveHarness();
  try {
    mocks.onDrop!(["/films/Detour-en.srt"]);
    await vi.waitFor(() => {
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_prepare_media_for_playback", {
        path: "/films/Detour(1945).mp4",
      });
    });
    await tick();

    // Top-right media picker immediately shows the detected film and disables Browse while loading.
    const busyField = target.querySelector<HTMLButtonElement>('button[aria-busy="true"]');
    expect(busyField).not.toBeNull();
    expect(busyField!.textContent).toContain("/films/Detour(1945).mp4");
    const browseMedia = target.querySelector<HTMLButtonElement>('button[title="Load video file"]');
    expect(browseMedia).not.toBeNull();
    expect(browseMedia!.disabled).toBe(true);

    // Wizard overlays the extracting status while keeping the playback range slider mounted.
    expect(target.textContent).toContain("Extracting audio track...");
    expect(target.querySelector('input[type="range"]')).not.toBeNull();

    resolvePrepare("/cache/detour.wav");
    await flushAsyncAndTimers();

    expect(target.querySelector('button[aria-busy="true"]')).toBeNull();
    expect(browseMedia!.disabled).toBe(false);
    expect(target.textContent).not.toContain("Extracting audio track...");

    // Wizard subtitle box uses fixed height and no inline shortcut hint strip.
    const subBox = target.querySelector('[data-testid="wizard-subtitle-box"]');
    expect(subBox).not.toBeNull();
    expect(subBox!.className).toContain("h-[136px]");
    expect(target.querySelectorAll("kbd")).toHaveLength(0);
  } finally {
    await unmount(component);
  }
});

it("saves synced SRT files and handles sync_save_file errors gracefully", async () => {
  let failSave = false;
  const snackbarSpy = vi.spyOn(snackbar, "show");
  setupSyncInvokeMock({
    sync_save_file: () => {
      if (failSave) throw new Error("Disk read-only");
      return "/out/synced.srt";
    },
  });
  vi.mocked(guardedSave).mockResolvedValue("/out/synced.srt");

  const { target, component } = await mountActiveHarness();
  try {
    await dropSrt("film.srt");
    const saveBtn = Array.from(target.querySelectorAll("button")).find((b) =>
      b.textContent?.includes("Save File"),
    )!;
    expect(saveBtn).toBeDefined();
    expect(saveBtn.disabled).toBe(false);

    saveBtn.click();
    await vi.waitFor(() => {
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_save_file", {
        outputPath: "/out/synced.srt",
      });
    });

    failSave = true;
    saveBtn.click();
    await vi.waitFor(() => {
      expect(snackbarSpy.mock.calls.some(([msg, level]) => String(msg).includes("Disk read-only") && level === "error")).toBe(true);
    });
  } finally {
    vi.mocked(guardedSave).mockReset();
    await unmount(component);
  }
});

it("persists and reloads sync sessions via Ctrl+Shift+S and Load Session without triggering audio autoplay", async () => {
  setupSyncInvokeMock({
    sync_save_session: () => "/out/session.json",
    sync_load_session: () => makeStatus({ srt_path: "restored.srt", anchor_count: 1 }),
    sync_get_anchors: () => [makeAnchor(1)],
  });
  vi.mocked(guardedSave).mockResolvedValue("/out/session.json");
  vi.mocked(guardedOpen).mockResolvedValue("/out/session.json");

  const { target, component } = await mountActiveHarness();
  try {
    const video = target.querySelector("video")!;
    const playSpy = vi.spyOn(video, "play").mockResolvedValue(undefined);

    await dropSrt("film.srt");
    await dropAudio("film.mp3");
    playSpy.mockClear();

    // Save session via Ctrl+Shift+S (sync-save-session shortcut)
    target.dispatchEvent(new KeyboardEvent("keydown", {
      key: "S",
      ctrlKey: true,
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    }));
    await vi.waitFor(() => {
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_save_session", {
        sessionPath: "/out/session.json",
      });
    });

    // Load session via Load Session button
    vi.mocked(invokeCommand).mockClear();
    const loadSessionBtn = Array.from(target.querySelectorAll("button")).find((b) =>
      b.textContent?.includes("Load Session"),
    )!;
    expect(loadSessionBtn).toBeDefined();
    loadSessionBtn.click();

    await vi.waitFor(() => {
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_load_session", {
        sessionPath: "/out/session.json",
      });
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_get_subtitles_range", {
        startId: 1,
        count: 50,
      });
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_get_anchors");
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_get_subtitle", { id: 2 });
    });
    await flushAsyncAndTimers(60);
    expect(playSpy).not.toHaveBeenCalled();
  } finally {
    vi.mocked(guardedSave).mockReset();
    vi.mocked(guardedOpen).mockReset();
    await unmount(component);
  }
});

it("resets loaded subtitles, audio, and status after confirming New Sync", async () => {
  setupSyncInvokeMock({
    sync_reset: () => makeStatus({
      is_loaded: false,
      srt_path: null,
      video_path: null,
      total_subtitles: 0,
      anchor_count: 0,
      checked_count: 0,
      completion_percentage: 0,
      suggested_next_id: null,
      can_undo: false,
      can_redo: false,
    }),
  });

  const { target, component } = await mountActiveHarness();
  try {
    await dropSrt("film.srt");
    await dropAudio("film.mp3");

    const newSyncBtn = Array.from(target.querySelectorAll("button")).find((b) =>
      b.textContent?.includes("New Sync"),
    )!;
    newSyncBtn.click();
    await tick();

    const dialog = target.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    const confirmBtn = Array.from(dialog!.querySelectorAll("button")).at(-1)!;
    confirmBtn.click();

    await vi.waitFor(() => {
      expect(vi.mocked(invokeCommand)).toHaveBeenCalledWith("sync_reset");
    });
    await flushAsyncAndTimers(40);

    const saveBtn = Array.from(target.querySelectorAll("button")).find((b) =>
      b.textContent?.includes("Save File"),
    )!;
    expect(saveBtn.disabled).toBe(true);
    expect(target.querySelector("video")?.getAttribute("src")).toBeNull();
  } finally {
    await unmount(component);
  }
});
