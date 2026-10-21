// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn() }));
import { invokeCommand } from "$lib/services/tauriClient";
import { AudioPreview } from "./audioPreview.svelte";

const invoke = vi.mocked(invokeCommand);
let players: FakeAudio[];
class FakeAudio {
  src = "";
  onended: (() => void) | null = null;
  onerror: (() => void) | null = null;
  pause = vi.fn();
  play = vi.fn().mockResolvedValue(undefined);
  constructor() { players.push(this); }
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
beforeEach(() => {
  players = [];
  vi.stubGlobal("Audio", FakeAudio);
  invoke.mockImplementation(async command => command === "get_media_server_info" ? [1234, "a&b"] : "clip name.mp3");
});
afterEach(() => { vi.unstubAllGlobals(); vi.resetAllMocks(); });

it("encodes the media URL and releases playback and handlers on stop", async () => {
  const error = vi.fn();
  const preview = new AudioPreview(error);
  await preview.play({ mediaPath: "film.mp4" });
  expect(preview.playing).toBe(true);
  expect(preview.loading).toBe(false);
  const url = new URL(players[0].src);
  expect(url.searchParams.get("path")).toBe("clip name.mp3");
  expect(url.searchParams.get("token")).toBe("a&b");
  players[0].onended!();
  expect(preview.playing).toBe(false);
  preview.stop();
  expect(players[0].pause).toHaveBeenCalled();
  expect(players[0].src).toBe("");
  expect(players[0].onerror).toBeNull();
  expect(error).not.toHaveBeenCalled();
});
it("does not start playback when closed during generation", async () => {
  const generated = deferred<string>();
  invoke.mockImplementation(async command => command === "get_media_server_info" ? [1234, "token"] : generated.promise);
  const preview = new AudioPreview(vi.fn());
  const pending = preview.play({});
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("flashcard_preview_audio", expect.any(Object)));
  preview.stop();
  generated.resolve("old.mp3");
  await pending;
  expect(players).toHaveLength(0);
  expect(preview.loading).toBe(false);
});
it("keeps the newest request when an older generation finishes late", async () => {
  const generated = deferred<string>();
  invoke.mockImplementation(async (command, args) => command === "get_media_server_info" ? [1234, "token"] : args?.old ? generated.promise : "new.mp3");
  const preview = new AudioPreview(vi.fn());
  const old = preview.play({ old: true });
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("flashcard_preview_audio", expect.any(Object)));
  await preview.play({});
  generated.resolve("old.mp3");
  await old;
  expect(players).toHaveLength(1);
  expect(players[0].src).toContain("new.mp3");
  expect(preview.playing).toBe(true);
});
it("ignores late playback completion after stop and reports a current rejection", async () => {
  const rejected = new Error("Playback refused");
  const onError = vi.fn();
  const preview = new AudioPreview(onError);
  // Replace the constructor to configure the playback promise before play starts.
  const playback = deferred<void>();
  vi.stubGlobal("Audio", class extends FakeAudio { play = vi.fn(() => playback.promise); });
  const pending = preview.play({});
  await vi.waitFor(() => expect(players).toHaveLength(1));
  preview.stop();
  playback.resolve();
  await pending;
  expect(preview.playing).toBe(false);
  expect(players[0].pause).toHaveBeenCalled();
  vi.stubGlobal("Audio", class extends FakeAudio { play = vi.fn().mockRejectedValue(rejected); });
  await preview.play({});
  expect(onError).toHaveBeenCalledWith(rejected);
  expect(preview.loading).toBe(false);
});
it("rejects a changed episode before creating a player", async () => {
  let valid = true;
  const generated = deferred<string>();
  invoke.mockImplementation(async command => command === "get_media_server_info" ? [1234, "token"] : generated.promise);
  const preview = new AudioPreview(vi.fn());
  const pending = preview.play({}, () => valid);
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("flashcard_preview_audio", expect.any(Object)));
  valid = false;
  generated.resolve("old.mp3");
  await pending;
  expect(players).toHaveLength(0);
  expect(preview.loading).toBe(false);
});

it("suppresses a playback rejection from a closed request", async () => {
  const playback = deferred<void>();
  const error = vi.fn();
  vi.stubGlobal("Audio", class extends FakeAudio { play = vi.fn(() => playback.promise); });
  const preview = new AudioPreview(error);
  const pending = preview.play({});
  await vi.waitFor(() => expect(players).toHaveLength(1));
  preview.stop();
  playback.reject(new Error("late refusal"));
  await pending;
  expect(error).not.toHaveBeenCalled();
  expect(preview.playing).toBe(false);
  expect(preview.loading).toBe(false);
});
it("reports current media errors and ignores callbacks saved before stop", async () => {
  const error = vi.fn();
  const preview = new AudioPreview(error);
  await preview.play({});
  const onError = players[0].onerror!;
  onError();
  expect(error).toHaveBeenCalledOnce();
  expect(preview.playing).toBe(false);
  preview.stop();
  onError();
  expect(error).toHaveBeenCalledOnce();
});
