// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import SpeechEngineSelector from "./SpeechEngineSelector.svelte";

const state = vi.hoisted(() => ({
  selection: "external",
  keys: [{ id: "key", apiKey: "token", apiType: "openai", capabilities: ["stt"] }],
  entry: { id: "external", provider: "openai", apiKeyId: "key", model: "whisper-1" },
}));
vi.mock("$lib/config/vestaConfig", () => ({ getItem: () => state.selection, setItem: vi.fn() }));
vi.mock("$lib/config/transcribeTiers", () => ({
  loadTranscribeTiers: () => [{ id: "tier", entries: [state.entry] }],
  TRANSCRIBE_TIERS_UPDATED_EVENT: "tiers-updated",
}));
vi.mock("$lib/config/apiKeys", () => ({ loadAndValidateApiKeys: () => state.keys, supportsCapability: () => true }));
vi.mock("$lib/stores/whisperModelsStore.svelte", () => ({ whisperModelsStore: { whisperModels: [], defaultWhisperModel: "base" } }));
afterEach(() => { document.body.replaceChildren(); });

it("restores the explicit cloud choice without a local model and blocks when its key disappears", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const resolve = vi.fn();
  const component = mount(SpeechEngineSelector, { target, props: { purpose: "sync", onresolve: resolve } });
  await tick();
  expect(resolve).toHaveBeenLastCalledWith(state.entry);
  expect(target.querySelector("input")?.value).toContain("whisper-1");
  state.keys = [];
  window.dispatchEvent(new Event("apikeys-updated"));
  await tick();
  expect(resolve).toHaveBeenLastCalledWith(undefined);
  await unmount(component);
  resolve.mockClear();
  window.dispatchEvent(new Event("apikeys-updated"));
  await tick();
  expect(resolve).not.toHaveBeenCalled();
});
