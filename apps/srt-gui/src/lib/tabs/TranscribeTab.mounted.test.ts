// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  open: vi.fn(),
  save: vi.fn(),
  onDrop: undefined as ((paths: string[]) => void) | undefined,
  store: new Map<string, string>(),
}));

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: mocks.invoke,
}));
vi.mock("$lib/utils/dialogGuard", () => ({
  guardedOpen: mocks.open,
  guardedSave: mocks.save,
}));
vi.mock("$lib/utils/dragDrop", () => ({
  setupWebviewDragDrop: vi.fn((options: { onDrop: (paths: string[]) => void }) => {
    mocks.onDrop = options.onDrop;
    return () => {
      mocks.onDrop = undefined;
    };
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => mocks.store.get(key) ?? null,
  setItem: (key: string, value: string) => {
    mocks.store.set(key, value);
  },
  removeItem: (key: string) => {
    mocks.store.delete(key);
  },
  initConfig: vi.fn().mockResolvedValue(undefined),
  flush: vi.fn().mockResolvedValue(undefined),
}));

import TranscribeTab from "./TranscribeTab.svelte";
import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";

const mounted: ReturnType<typeof mount>[] = [];

async function settle(turns = 6) {
  for (let i = 0; i < turns; i++) {
    await Promise.resolve();
    await tick();
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  vi.clearAllMocks();
  localWhisperOperation.cancelRequested = false;
  mocks.store.clear();
  mocks.store.set(
    "vesta-transcribe-tiers",
    JSON.stringify([
      {
        id: "tier-1",
        entries: [{ id: "e1", provider: "local", apiKeyId: "local_whisper", model: "base" }],
      },
    ]),
  );
  mocks.store.set("vesta-default-whisper-model", "base");
});

afterEach(async () => {
  for (const component of mounted.splice(0)) {
    await unmount(component);
  }
  document.body.replaceChildren();
  mocks.onDrop = undefined;
});

describe("TranscribeTab", () => {
  it("shows FFmpeg banner when ffmpeg is unavailable", async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "transcribe_check_backends") {
        return { ffmpeg: false, vulkan_compiled: false, backends: [] };
      }
      if (cmd === "transcribe_list_models") {
        return [{ id: "base", name: "Base", size_mb: 142, downloaded: true }];
      }
      if (cmd === "transcribe_addons_status") {
        return { ffmpeg_ready: false, vad_ready: true, vad_models: [] };
      }
      return null;
    });

    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(TranscribeTab, { target, props: { active: true } }));
    await settle(8);

    expect(target.textContent).toContain("FFmpeg");
  });

  it("loads dropped media, starts transcription, handles errors and cancels cleanly", async () => {
    const startGate = deferred<{
      success: boolean;
      message: string;
      output_path?: string;
      subtitle_count?: number;
    }>();

    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "transcribe_check_backends") {
        return { ffmpeg: true, vulkan_compiled: false, backends: [] };
      }
      if (cmd === "transcribe_list_models") {
        return [{ id: "base", name: "Base", size_mb: 142, downloaded: true }];
      }
      if (cmd === "transcribe_addons_status") {
        return { ffmpeg_ready: true, vad_ready: true, vad_models: [] };
      }
      if (cmd === "transcribe_start") {
        return startGate.promise;
      }
      if (cmd === "transcribe_cancel") {
        return true;
      }
      return null;
    });

    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(TranscribeTab, { target, props: { active: true } }));
    await settle(8);

    mocks.onDrop?.(["/media/Episode01.mkv"]);
    await settle(6);

    const startBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Start Transcription"),
    )!;
    expect(startBtn).toBeDefined();
    expect(startBtn.disabled).toBe(false);

    startBtn.click();
    await settle(6);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "transcribe_start",
      expect.objectContaining({
        config: expect.objectContaining({
          input_path: "/media/Episode01.mkv",
          model: "base",
        }),
      }),
    );

    const cancelBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Cancel"),
    )!;
    expect(cancelBtn).toBeDefined();
    cancelBtn.click();
    await settle(6);
    expect(mocks.invoke).toHaveBeenCalledWith("transcribe_cancel");

    startGate.resolve({
      success: false,
      message: "Cancelled",
    });
    await settle(6);
  });
});
