// @vitest-environment happy-dom
import { beforeEach, expect, it, vi } from "vitest";
const { storage } = vi.hoisted(() => ({ storage: new Map<string, string>() }));
vi.mock("$lib/config/vestaConfig", () => ({getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key)}));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({snackbar: {show: vi.fn()}}));
vi.mock("$lib/services/tauriClient", () => ({invokeCommand: vi.fn().mockResolvedValue([])}));
beforeEach(() => { vi.resetModules(); storage.clear(); });
it("defaults word lookup on and respects an explicit opt-out", async () => {
 const { ankiStore } = await import("./ankiStore.svelte");
 expect(ankiStore.dictionaryLookup).toBe(true);
 storage.set("vesta-dictionary-lookup-enabled", "false"); ankiStore.reinit();
 expect(ankiStore.dictionaryLookup).toBe(false);
});
it("rejects blank names and duplicate built-in schemas without writing presets", async () => {
 const { ankiTemplateStore: store } = await import("./ankiTemplateStore.svelte");
 store.applyAnkiFieldPreset("predef:en");
 store.ankiFieldPresetName = "   "; expect(store.saveCurrentAnkiFieldPreset()).toBe(false);
 store.ankiFieldPresetName = "English copy"; expect(store.saveCurrentAnkiFieldPreset()).toBe(false);
 expect(storage.has("vesta-anki-field-presets")).toBe(false);
});
it("persists custom language for exports, rejects duplicate names and sorts custom templates first", async () => {
 const { ankiTemplateStore: store } = await import("./ankiTemplateStore.svelte");
 store.applyAnkiFieldPreset("predef:en"); store.ankiFieldPresetName = "Zulu";
 store.fieldNotes = "Extra"; store.noteTypeLanguage = "ja";
 expect(store.saveCurrentAnkiFieldPreset()).toBe(true);
 const { loadCustomNoteTypes } = await import("$lib/types/noteTypes");
 expect(loadCustomNoteTypes()[0]).toMatchObject({name: "Zulu", language: "ja"});
 store.applyAnkiFieldPreset("predef:fr"); store.ankiFieldPresetName = "Zulu"; store.fieldNotes = "Other";
 expect(store.saveCurrentAnkiFieldPreset()).toBe(false);
 store.ankiFieldPresetName = "Alpha"; expect(store.saveCurrentAnkiFieldPreset()).toBe(true);
 expect(store.allAnkiFieldPresets.slice(0, 2).map(p => p.name)).toEqual(["Alpha", "Zulu"]);
 store.applyAnkiFieldPreset(store.allAnkiFieldPresets[1].id); expect(store.noteTypeLanguage).toBe("ja");
});
