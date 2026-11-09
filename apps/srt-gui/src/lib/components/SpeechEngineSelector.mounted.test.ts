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

const instances: ReturnType<typeof mount>[] = [];
afterEach(async () => {
  for (const instance of instances.splice(0)) await unmount(instance);
  document.body.replaceChildren();
  vi.restoreAllMocks();
  state.selection = "external";
  state.keys = [{ id: "key", apiKey: "token", apiType: "openai", capabilities: ["stt"] }];
});

it("restores the explicit cloud choice without a local model and blocks when its key disappears", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const resolve = vi.fn();
  const component = mount(SpeechEngineSelector, { target, props: { purpose: "sync", onresolve: resolve } });
  instances.push(component);
  await tick();
  expect(resolve).toHaveBeenLastCalledWith(state.entry);
  expect(target.querySelector("input")?.value).toContain("whisper-1");
  state.keys = [];
  window.dispatchEvent(new Event("apikeys-updated"));
  await tick();
  expect(resolve).toHaveBeenLastCalledWith(undefined);
});

it("portals the transcription engine menu above sibling cards and cleans it up", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const onresolve = vi.fn();
  instances.push(mount(SpeechEngineSelector, { target, props: { purpose: "transcribe", onresolve } }));
  const anchor = target.querySelector(".relative")!;
  vi.spyOn(anchor, "getBoundingClientRect").mockReturnValue({ left: 20, right: 320, top: 80, bottom: 120, width: 300, height: 40 } as DOMRect);
  target.querySelector("input")!.focus();
  await tick();

  const menu = document.body.querySelector<HTMLDivElement>(".searchable-select-dropdown")!;
  expect(menu.parentElement).toBe(document.body);
  expect(menu.style.position).toBe("fixed");
  expect(menu.style.top).toBe("124px");
  expect(menu.style.width).toBe("300px");
  expect(menu.style.background).toContain("--control-menu-bg");

  const chain = [...menu.querySelectorAll("button")].find(button => button.textContent?.includes("Default priority chain"));
  expect(chain).toBeTruthy();
  chain!.click();
  await tick();
  expect(onresolve).toHaveBeenLastCalledWith(null);
  expect(menu.isConnected).toBe(false);
});
