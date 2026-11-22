import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { storage, mocks } = vi.hoisted(() => ({
  storage: new Map<string, string>(),
  mocks: {
    invokeCommand: vi.fn().mockResolvedValue(true),
  },
}));

vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => storage.set(key, val),
  removeItem: (key: string) => storage.delete(key),
}));

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: mocks.invokeCommand,
}));

import { aiStore } from "./aiStore.svelte";
import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";

describe("aiStore", () => {
  let activeRun: ReturnType<typeof localWhisperOperation.begin> = null;

  beforeEach(() => {
    activeRun?.finish();
    activeRun = null;
    storage.clear();
    mocks.invokeCommand.mockClear();
    aiStore.killSwitchActive = false;
    aiStore.isTranslating = false;
    aiStore.isTranscribing = false;
    aiStore.autoRefining = false;
    aiStore.isSingleRefining = false;
    aiStore.isOcring = false;
  });

  afterEach(() => {
    activeRun?.finish();
    activeRun = null;
  });

  it("toggles kill switch and persists setting when idle", () => {
    expect(aiStore.hasActiveAiProcess).toBe(false);
    aiStore.toggleKillSwitch();
    expect(aiStore.killSwitchActive).toBe(true);
    expect(storage.get("vesta-ai-kill-switch")).toBe("true");
    expect(mocks.invokeCommand).not.toHaveBeenCalled();

    aiStore.toggleKillSwitch();
    expect(aiStore.killSwitchActive).toBe(false);
    expect(storage.get("vesta-ai-kill-switch")).toBe("false");
  });

  it("allows activation while AI jobs are running and cancels active backend operations", async () => {
    aiStore.isTranslating = true;
    aiStore.isOcring = true;
    aiStore.autoRefining = true;
    activeRun = localWhisperOperation.begin("autosync");
    expect(aiStore.hasActiveAiProcess).toBe(true);

    aiStore.toggleKillSwitch();
    await Promise.resolve();
    await Promise.resolve();

    expect(aiStore.killSwitchActive).toBe(true);
    expect(storage.get("vesta-ai-kill-switch")).toBe("true");
    expect(mocks.invokeCommand).toHaveBeenCalledWith("cancel_translation");
    expect(mocks.invokeCommand).toHaveBeenCalledWith("ocr_cancel");
    expect(mocks.invokeCommand).toHaveBeenCalledWith("refine_cancel");
    expect(mocks.invokeCommand).toHaveBeenCalledWith("sync_cancel_auto_sync");
  });

  it("deduplicates transcribe_cancel when both isTranscribing and localWhisperOperation are active", async () => {
    aiStore.isTranscribing = true;
    activeRun = localWhisperOperation.begin("transcription");

    aiStore.toggleKillSwitch();
    await Promise.resolve();

    expect(
      mocks.invokeCommand.mock.calls.filter(([cmd]) => cmd === "transcribe_cancel"),
    ).toHaveLength(1);
  });
});
