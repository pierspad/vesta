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

import TranslateTab from "./TranslateTab.svelte";
import { aiStore } from "$lib/stores/aiStore.svelte";

const mounted: ReturnType<typeof mount>[] = [];

async function settle(turns = 8) {
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
  mocks.store.clear();
  mocks.store.set(
    "vesta-api-keys",
    JSON.stringify([
      {
        id: "key-google",
        name: "Gemini",
        apiType: "google",
        apiKey: "AIzaTestKey1234567890",
        capabilities: ["llm"],
      },
    ]),
  );
  mocks.store.set(
    "vesta-translate-tiers",
    JSON.stringify([
      {
        id: "tier-1",
        entries: [{ id: "e1", provider: "google", apiKeyId: "key-google", model: "gemini-2.5-flash" }],
      },
    ]),
  );
});

afterEach(async () => {
  for (const component of mounted.splice(0)) {
    await unmount(component);
  }
  document.body.replaceChildren();
  mocks.onDrop = undefined;
});

describe("TranslateTab", () => {
  it("loads SRT metadata, runs translation, supports cancel, and formats provider rate-limit errors", async () => {
    const translateGate = deferred<{
      success: boolean;
      message: string;
      output_path?: string;
      translated_count: number;
    }>();

    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "load_srt_for_translate") {
        return {
          path: "/subs/ep01.en.srt",
          subtitle_count: 12,
          preview_subtitles: [{ id: 1, text: "Hello world" }],
        };
      }
      if (cmd === "start_translation") {
        return translateGate.promise;
      }
      if (cmd === "cancel_translation") {
        return null;
      }
      if (cmd === "get_latest_translated_subtitles") {
        return [];
      }
      return null;
    });

    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(TranslateTab, { target, props: { active: true } }));
    await settle();

    // Load an SRT file via drag-drop callback
    mocks.onDrop?.(["/subs/ep01.en.srt"]);
    await settle();

    expect(mocks.invoke).toHaveBeenCalledWith("load_srt_for_translate", {
      path: "/subs/ep01.en.srt",
    });
    expect(target.textContent).toContain("Hello world");

    // Start translation and test cancel while in flight
    const startBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Start Translation"),
    )!;
    expect(startBtn).toBeDefined();
    expect(startBtn.disabled).toBe(false);
    startBtn.click();
    await settle();

    expect(mocks.invoke).toHaveBeenCalledWith(
      "start_translation",
      expect.objectContaining({
        config: expect.objectContaining({
          input_path: "/subs/ep01.en.srt",
        }),
      }),
    );

    const cancelBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Cancel"),
    )!;
    expect(cancelBtn).toBeDefined();
    cancelBtn.click();
    await settle();
    expect(mocks.invoke).toHaveBeenCalledWith("cancel_translation");

    translateGate.resolve({
      success: false,
      message: "Cancelled by user",
      translated_count: 0,
    });
    await settle();

    // Verify rate-limit error banner when retry fails with 429
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "start_translation") {
        throw new Error("HTTP 429 quota exceeded");
      }
      if (cmd === "get_latest_translated_subtitles") {
        return [];
      }
      return null;
    });
    const retryBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Retry") || b.textContent?.includes("Start Translation"),
    )!;
    retryBtn.click();
    await settle();
    expect(target.textContent).toContain("429");
  });

  it("hides and restores the Suggest context AI button when AI Kill Switch is toggled", async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "load_srt_for_translate") {
        return {
          path: "/subs/Detour.1945.en.srt",
          subtitle_count: 10,
          coverage_end_ms: 3600000,
          preview_subtitles: [{ id: 1, text: "Give a guy a break." }],
        };
      }
      if (cmd === "suggest_translation_context") {
        return "Detour (1945): noir road movie set in post-war America. Use a colloquial, hard-boiled register.";
      }
      return null;
    });

    aiStore.killSwitchActive = false;
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(TranslateTab, { target, props: { active: true } }));
    await settle();

    mocks.onDrop?.(["/subs/Detour.1945.en.srt"]);
    await settle();

    const findSuggestBtn = () =>
      Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
        b.textContent?.includes("Suggest context"),
      );

    expect(findSuggestBtn()).toBeDefined();
    findSuggestBtn()!.click();
    await settle();

    const contextInput = target.querySelector<HTMLTextAreaElement>("#context-input");
    expect(contextInput?.value).toContain("Detour (1945): noir road movie");

    aiStore.killSwitchActive = true;
    await settle();
    expect(findSuggestBtn()).toBeUndefined();

    aiStore.killSwitchActive = false;
    await settle();
    expect(findSuggestBtn()).toBeDefined();
  });
});
