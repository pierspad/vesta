import { describe, expect, it, vi } from "vitest";
import { buildSetupPlan, installSetupPlan, resolveSetupChoices, setupTierPreferences, type SetupChoices, type SetupResources } from "./firstRunSetup";
const choices: SetupChoices = { mode: "quick", wantsTranscription: false, whisperModel: "large", useVad: false, vadChoice: "custom", nativeLanguage: "ja", studyLanguage: "ja" };
const resources: SetupResources = {
 models: ["small", "medium", "large"].map(id => ({ id, name: id, size: "~500 MB", speed: "", downloaded: false })),
 vadModels: [{ id: "v6.2.0", size: "~0.9 MB", downloaded: false }],
 fonts: [{ id: "japanese", name: "Japanese font", language_name: "Japanese", target_languages: ["ja"], filename: "jp.otf", approx_size: "~12 MB", downloaded: false }],
 mediaReady: false, mediaDownloadBytes: 80_000_000,
};
describe("first run setup", () => {
 it("quick setup always includes Small, Silero, media tools and deduplicated language fonts", () => {
  const plan = buildSetupPlan(choices, resources);
  expect(plan.map(task => [task.kind, task.resourceId])).toEqual([["media", "ffmpeg"], ["whisper", "small"], ["vad", "v6.2.0"], ["font", "japanese"]]);
  expect(plan[0].size).toBe("~80 MB");
  expect(choices.wantsTranscription).toBe(false);
 });
 it("custom choices can select Medium, custom VAD or no transcription", () => {
  const custom = { ...choices, mode: "custom" as const, wantsTranscription: true, whisperModel: "medium", useVad: true };
  expect(resolveSetupChoices(custom)).toEqual(custom);
  expect(buildSetupPlan(custom, resources).map(task => task.resourceId)).toEqual(["ffmpeg", "medium", "japanese"]);
  expect(buildSetupPlan({ ...custom, wantsTranscription: false }, resources).map(task => task.kind)).toEqual(["media", "font"]);
 });
 it("keeps unknown archive size honest and excludes fonts not needed by the chosen languages", () => {
  const plan = buildSetupPlan({ ...choices, nativeLanguage: "it", studyLanguage: "en" }, { ...resources, mediaDownloadBytes: null });
  expect(plan[0].size).toBe("");
  expect(plan.some(task => task.kind === "font")).toBe(false);
 });
 it("fails discovery for a missing selected model", () => {
  expect(() => buildSetupPlan(choices, { ...resources, models: [] })).toThrow("Whisper model unavailable");
 });
 it("stops on failed downloads, retries only missing components, and never marks a failure ready", async () => {
  const plan = buildSetupPlan(choices, resources); const statuses = vi.fn();
  const install = vi.fn(async (task) => { if (task.kind === "vad") throw new Error("offline"); });
  await expect(installSetupPlan(plan, install, statuses)).rejects.toThrow("offline");
  expect(install.mock.calls.map(([task]) => task.kind)).toEqual(["media", "whisper", "vad"]);
  expect(statuses).toHaveBeenLastCalledWith(plan[2], "failed");
  const retry = vi.fn(async () => {});
  await installSetupPlan(plan.map((task, i) => ({ ...task, installed: i < 2 })), retry, statuses);
  expect(retry.mock.calls).toHaveLength(2);
  expect(statuses).toHaveBeenLastCalledWith(expect.objectContaining({ kind: "font" }), "ready");
 });
 it("persists the chosen model in the local tier so the first actual request uses it", () => {
  expect(JSON.parse(setupTierPreferences("small", true))[0].entries[0]).toMatchObject({ provider: "local", model: "small" });
  expect(JSON.parse(setupTierPreferences("medium", true))[0].entries[0].model).toBe("medium");
  expect(JSON.parse(setupTierPreferences("small", false))).toEqual([]);
 });
});
