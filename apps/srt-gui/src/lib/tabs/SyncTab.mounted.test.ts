// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import SyncTab from "./SyncTab.svelte";
import SyncTabActiveHarness from "$lib/test/fixtures/SyncTabActiveHarness.svelte";
import { whisperModelsStore } from "$lib/stores/whisperModelsStore.svelte";
import { invokeCommand } from "$lib/services/tauriClient";

vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn().mockResolvedValue([]) }));
vi.mock("$lib/utils/dragDrop", () => ({ setupWebviewDragDrop: vi.fn(() => () => {}) }));
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn(), guardedSave: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

afterEach(() => { vi.restoreAllMocks(); document.body.replaceChildren(); });

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
    await unmount(component);
  }
});
