// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn() }));
vi.mock("$lib/config/vestaConfig", () => ({ getItem: () => "false", setItem: vi.fn(), removeItem: vi.fn() }));
vi.mock("$lib/stores/uiModeStore.svelte", () => ({ uiMode: { expertMode: true } }));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { invokeCommand } from "$lib/services/tauriClient";
import { defaultMediaSettings } from "$lib/utils/mediaSettings";
import AudioClipsPanel from "./AudioClipsPanel.svelte";

const invoke = vi.mocked(invokeCommand);
const play = vi.fn().mockResolvedValue(undefined);
const instances: ReturnType<typeof mount>[] = [];
const episodes = [
  { id: 1, targetSubsPath: "first.srt", nativeSubsPath: "", mediaPath: "first.mp3" },
  { id: 2, targetSubsPath: "", nativeSubsPath: "second.it.srt", mediaPath: "second.mp3" },
];
const entries = [{ id: 1, start_ms: 3000, end_ms: 5000, text: "Second episode dialogue" }];

beforeEach(() => {
  play.mockClear();
  vi.stubGlobal("Audio", class { src = ""; pause = vi.fn(); play = play; });
  vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(() => {});
  invoke.mockImplementation(async (command) => {
    if (command === "flashcard_media_duration") return 90;
    if (command === "flashcard_parse_subtitles") return entries;
    if (command === "get_media_server_info") return [1234, "token"];
    if (command === "flashcard_preview_audio") return "preview.mp3";
  });
});
afterEach(async () => {
  for (const instance of instances.splice(0)) await unmount(instance);
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  invoke.mockReset();
  document.body.replaceChildren();
});
async function render() {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(AudioClipsPanel, { target, props: {
    settings: { ...defaultMediaSettings }, episodes,
    hasAudio: true, mediaType: "audio", audioTracks: [], audioTracksLoading: false,
    hintLoadMediaFirst: "Load media", onTrackPicked: vi.fn(),
  } });
  instances.push(component);
  await tick();
  return { target, component };
}

it("opens a collapsed preview, selects the requested episode and plays its reference subtitles", async () => {
  const { target, component } = await render();
  expect(target.querySelector(".preview-glow")).toBeNull();
  await component.previewEpisode(1);
  await tick();
  expect(target.querySelector(".preview-glow")).not.toBeNull();
  expect(Array.from(target.querySelectorAll("input")).some(input => input.value.includes("second.mp3"))).toBe(true);
  expect(target.textContent).toContain("Second episode dialogue");
  expect(invoke).toHaveBeenCalledWith("flashcard_preview_audio", expect.objectContaining({
    mediaPath: "second.mp3", subPath: "second.it.srt", startMs: 3000, endMs: 5000,
  }));
  expect(play).toHaveBeenCalledOnce();
  expect(target.querySelector('input[type="range"]')?.getAttribute("max")).toBe("90");
});

it("waits for subtitles and rejects an older episode request that finishes late", async () => {
  let resolve!: (value: typeof entries) => void;
  const delayed = new Promise<typeof entries>(yes => resolve = yes);
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation((command, args) => command === "flashcard_parse_subtitles" && args?.path === "second.it.srt"
    ? delayed : original(command, args));
  const { component } = await render();
  const old = component.previewEpisode(1);
  await tick();
  const latest = component.previewEpisode(0);
  await latest;
  resolve(entries);
  await old;
  expect(invoke.mock.calls.filter(([command]) => command === "flashcard_preview_audio")).toHaveLength(1);
  expect(invoke).toHaveBeenCalledWith("flashcard_preview_audio", expect.objectContaining({ mediaPath: "first.mp3" }));
});
