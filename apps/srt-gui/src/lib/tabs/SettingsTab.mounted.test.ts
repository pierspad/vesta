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
it("restores Anki connection and card options in expert mode", async () => {
  uiMode.expertMode = true;
  ankiStore.setUrl("http://localhost:9000"); ankiStore.setDictionaryLookup(false);
  ankiStore.setAutoCardFont(false); ankiStore.setEmbedCardFont(false);
  await reset("anki");
  expect(ankiStore.url).toBe("http://127.0.0.1:8765");
  expect(ankiStore.dictionaryLookup && ankiStore.autoCardFont && ankiStore.embedCardFont).toBe(true);
});
it("confirms deletion from the provider editor and closes the deleted configuration", async () => {
  const {apiKeyEditorStore}=await import("$lib/stores/apiKeyEditorStore.svelte");
  const key={id:"delete-test",apiType:"openai" as const,name:"Delete me",apiKey:"example"};
  config.setItem("srt-tools-api-keys",JSON.stringify([key]));
  const target=document.createElement("div");document.body.append(target);
  mounted.push(mount(SettingsTab,{target,props:{requestedSection:"llm"}}));await tick();await tick();
  apiKeyEditorStore.openEdit(key,"llm");await tick();
  target.querySelector<HTMLButtonElement>('[data-provider-action="delete"]')!.click();await tick();
  expect(JSON.parse(config.getItem("srt-tools-api-keys")!)).toContainEqual(key);
  target.querySelector<HTMLButtonElement>(".confirm-btn-cancel")!.click();await tick();
  expect(apiKeyEditorStore.showAddKey).toBe(true);
  expect(apiKeyEditorStore.newKeyName).toBe("Delete me");
  target.querySelector<HTMLButtonElement>('[data-provider-action="delete"]')!.click();await tick();
  target.querySelector<HTMLButtonElement>(".confirm-btn-action")!.click();await tick();
  expect(JSON.parse(config.getItem("srt-tools-api-keys")!)).not.toContainEqual(key);
  expect(apiKeyEditorStore.showAddKey).toBe(false);
});

it("selects an existing sole provider by default and transfers selection to another configuration", async () => {
  const first={id:"first-openai",apiType:"openai",name:"First API",apiKey:"example"};
  config.setItem("srt-tools-api-keys",JSON.stringify([first]));
  const target=document.createElement("div");document.body.append(target);
  mounted.push(mount(SettingsTab,{target,props:{requestedSection:"llm"}}));await tick();await tick();
  expect(target.querySelector('[data-key-default="first-openai"]')?.getAttribute("aria-pressed")).toBe("true");
  // The selected configuration stays selected; choosing another changes it.
  target.querySelector<HTMLButtonElement>('[data-key-default="first-openai"]')!.click();await tick();
  expect(target.querySelector('[data-key-default="first-openai"]')?.getAttribute("aria-pressed")).toBe("true");
  const {apiKeyEditorStore}=await import("$lib/stores/apiKeyEditorStore.svelte");
  apiKeyEditorStore.openAdd("llm","openai");
  apiKeyEditorStore.newKeyName="Second API";apiKeyEditorStore.newKeyValue="example";await tick();
  target.querySelector<HTMLButtonElement>('[aria-labelledby="provider-dialog-title"] button.bg-indigo-500')!.click();await tick();
  const second=JSON.parse(config.getItem("srt-tools-api-keys")!).find((key:{name:string})=>key.name==="Second API");
  expect(target.querySelector(`[data-key-default="${second.id}"]`)?.getAttribute("aria-pressed")).toBe("false");
  target.querySelector<HTMLButtonElement>(`[data-key-default="${second.id}"]`)!.click();await tick();
  expect(target.querySelector('[data-key-default="first-openai"]')?.getAttribute("aria-pressed")).toBe("false");
  expect(JSON.parse(config.getItem("srt-tools-api-keys")!).find((key:{id:string})=>key.id===second.id).isDefault).toBe(true);
});
