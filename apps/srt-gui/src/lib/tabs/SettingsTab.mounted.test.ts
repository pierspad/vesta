// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock("$lib/services/tauriHttp", () => ({ fetch: vi.fn().mockRejectedValue(new Error("offline")) }));
import SettingsTab from "./SettingsTab.svelte";
import * as config from "$lib/config/vestaConfig";
import { uiMode } from "$lib/stores/uiModeStore.svelte";
import { ankiStore } from "$lib/stores/ankiStore.svelte";
vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: vi.fn(async (command: string) => {
    if (command === "transcribe_list_models" || command === "flashcard_list_fonts") return [];
    if (command === "transcribe_addons_status") return { vad_models: [] };
    if (command === "flashcard_get_cpu_count") return 4;
    if (command === "flashcard_get_total_memory_mb") return 4096;
    return undefined;
  }),
}));
const mounted: ReturnType<typeof mount>[] = [];
afterEach(async () => {
  for (const instance of mounted.splice(0)) await unmount(instance);
  document.body.replaceChildren(); uiMode.expertMode = false;
});
async function reset(section: "overview" | "language" | "anki") {
  const target = document.createElement("div"); document.body.append(target);
  mounted.push(mount(SettingsTab, {target, props:{requestedSection:section}}));
  await tick(); await tick();
  const button = [...target.querySelectorAll("button")].find(node => node.textContent?.includes("Reset to Defaults"));
  expect(button).toBeDefined(); button!.click(); await tick();
  target.querySelector<HTMLButtonElement>(".confirm-btn-action")!.click(); await tick();
}
it("resets languages and the flashcard alias in expert mode", async () => {
  uiMode.expertMode = true;
  for (const key of ["target", "transcribe", "flashcards", "native"]) config.setItem(`vesta-default-${key}-language`, "it");
  config.setItem("vesta-flashcards-note-type-language", "it");
  await reset("language");
  expect(config.getItem("vesta-default-native-language")).toBe("en");
  expect(config.getItem("vesta-default-flashcards-language")).toBe("en");
  expect(config.getItem("vesta-default-transcribe-language")).toBe("auto");
  expect(config.getItem("vesta-flashcards-note-type-language")).toBe("en");
});
it("resets languages from the easy-mode overview", async () => {
  config.setItem("vesta-default-native-language", "fr");
  await reset("overview");
  expect(config.getItem("vesta-default-native-language")).toBe("en");
});
