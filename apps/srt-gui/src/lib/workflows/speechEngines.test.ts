import { describe, expect, it } from "vitest";
import { availableSpeechEngines } from "./speechEngines";
import type { ApiKeyConfig } from "$lib/config/apiKeys";

const keys = [{ id: "key", apiKey: "secret", apiType: "openai", capabilities: ["stt"] }] as ApiKeyConfig[];
const model = { id: "small", name: "Small", size: "", speed: "", downloaded: true };
const whisper = { id: "cloud", provider: "openai", apiKeyId: "key", model: "whisper-1" };
const textOnly = { ...whisper, id: "text", model: "gpt-4o-transcribe" };
const tiers = [{ id: "tier", entries: [whisper, textOnly] }];

describe("speech engine availability", () => {
  it("keeps external synchronization available without a local model", () => {
    expect(availableSpeechEngines(tiers, [], keys, true)).toEqual([whisper]);
    expect(availableSpeechEngines(tiers, [], keys, false)).toEqual([whisper, textOnly]);
  });
  it("removes unavailable endpoints without selecting a replacement", () => {
    expect(availableSpeechEngines(tiers, [{ ...model, downloaded: false }], [], true)).toEqual([]);
    expect(availableSpeechEngines(tiers, [model], [], true)).toEqual([
      { id: "local:small", provider: "local", model: "small", apiKeyId: "local_whisper" },
    ]);
  });
  it("requires STT capability and preserves custom timestamp endpoints", () => {
    const custom = { ...whisper, provider: "custom", id: "custom" };
    expect(availableSpeechEngines([{ id: "t", entries: [custom] }], [], [{ ...keys[0], apiUrl: "http://localhost:8000", apiKey: "" }], true)).toEqual([custom]);
    expect(availableSpeechEngines(tiers, [], [{ ...keys[0], capabilities: ["llm"] }], true)).toEqual([]);
  });
});
