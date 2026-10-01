import { describe, expect, it } from "vitest";
import { defaultMediaSettings } from "./mediaSettings";
import { episodeMediaDiff } from "./episodeMediaSettings";
describe("episode media overrides", () => {
  it("does not persist an automatically selected track as a manual override", () => {
    expect(episodeMediaDiff({ ...defaultMediaSettings, audioTrackIndex: 2 }, defaultMediaSettings, 2)).toEqual({});
  });
  it("preserves explicit different tracks and changed media settings", () => {
    expect(episodeMediaDiff({ ...defaultMediaSettings, audioTrackIndex: 3, audioGainDb: 4, cropBottom: 12 }, defaultMediaSettings, 2)).toEqual({ audioTrackIndex: 3, audioGainDb: 4, cropBottom: 12 });
  });
  it("compares against an explicit deck track and omits unchanged settings", () => {
    const defaults = { ...defaultMediaSettings, audioTrackIndex: 4 };
    expect(episodeMediaDiff(defaults, defaults, 3)).toEqual({});
    expect(episodeMediaDiff({ ...defaults, audioTrackIndex: null }, defaults, 3)).toEqual({ audioTrackIndex: null });
  });
});
