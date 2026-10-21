import { expect, it, vi } from "vitest";
import { FlashcardAudioSelection } from "./flashcardAudioSelection.svelte";
import type { AudioTrackInfo } from "$lib/types/flashcardMediaTypes";

const tracks: AudioTrackInfo[] = ["en", "it"].map((language, index) => ({
  index, stream_index: index + 1, language, codec: "aac", title: null, channels: 2,
}));
function setup(probe = vi.fn< (path: string) => Promise<AudioTrackInfo[]> >().mockResolvedValue(tracks)) {
  let language = "it";
  const select = vi.fn();
  const error = vi.fn();
  const state = new FlashcardAudioSelection(probe, () => language, select, error);
  return { state, probe, select, error, language: (value: string) => { language = value; } };
}
it("selects by language, follows changes only until a manual choice", async () => {
  const s = setup();
  await s.state.load("/film.mkv");
  expect(s.select).toHaveBeenLastCalledWith(1);
  s.language("en"); s.state.updateLanguage();
  expect(s.select).toHaveBeenLastCalledWith(0);
  s.state.automatic = false;
  s.language("it"); s.state.updateLanguage();
  expect(s.select).toHaveBeenLastCalledWith(0);
});
it("ignores superseded probes, including errors and loading cleanup", async () => {
  let fail!: (e: unknown) => void;
  let resolve!: (v: AudioTrackInfo[]) => void;
  const probe = vi.fn<(path: string) => Promise<AudioTrackInfo[]>>()
    .mockImplementationOnce(() => new Promise((_, reject) => { fail = reject; }))
    .mockImplementationOnce(() => new Promise(done => { resolve = done; }));
  const s = setup(probe);
  const old = s.state.load("old.mkv");
  const current = s.state.load("new.mp4");
  fail(new Error("obsolete")); await old;
  expect(s.error).not.toHaveBeenCalled();
  expect(s.state.loading).toBe(true);
  resolve(tracks); await current;
  expect(s.state.tracks).toEqual(tracks);
  expect(s.state.loading).toBe(false);
});
it("reset and disposal prevent a pending probe from changing selection", async () => {
  for (const dispose of [false, true]) {
    let resolve!: (v: AudioTrackInfo[]) => void;
    const s = setup(vi.fn().mockImplementation(() => new Promise(done => { resolve = done; })));
    const pending = s.state.load("film.mkv");
    if (dispose) s.state.dispose(); else s.state.reset();
    s.select.mockClear();
    resolve(tracks); await pending;
    expect(s.select).not.toHaveBeenCalled();
    expect(s.state.tracks).toEqual([]);
  }
});
it("does not probe non-video files, keeps automatic choice for a single track", async () => {
  const s = setup();
  await s.state.load("speech.mp3");
  expect(s.probe).not.toHaveBeenCalled();
  s.probe.mockResolvedValueOnce([tracks[0]]);
  await s.state.load("film.mkv");
  expect(s.select).toHaveBeenLastCalledWith(null);
});
it("reports current failures and permits another load", async () => {
  const s = setup();
  s.probe.mockRejectedValueOnce(new Error("probe failed"));
  await s.state.load("bad.mkv");
  expect(s.error).toHaveBeenCalledOnce();
  expect(s.state.loading).toBe(false);
  await s.state.load("good.mkv");
  expect(s.state.tracks).toEqual(tracks);
});
