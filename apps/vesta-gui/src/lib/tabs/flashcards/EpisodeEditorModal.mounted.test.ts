// @vitest-environment happy-dom
import { expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import EpisodeEditorModal from "./EpisodeEditorModal.svelte";
import EpisodeTable from "$lib/components/EpisodeTable.svelte";
import { listNoteTypes, predefinedNoteTypeForLanguage } from "$lib/types/noteTypes";
import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";

it("limits a field-specific editor to the requested reference", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(EpisodeEditorModal, { target, props: {
    editingEpisode: { id: 1, targetSubsPath: "original.srt", nativeSubsPath: "reference.srt", mediaPath: "movie.mp4", mediaType: "video" },
    onlyField: "nativeSubsPath", activeNoteType: predefinedNoteTypeForLanguage("en"),
    dropMediaHint: "", dropSubtitleHint: "", onClose: vi.fn(), onRevert: vi.fn(),
    onSelectFile: vi.fn(), onClearField: vi.fn(), onExpandField: vi.fn(),
  } });
  await tick();
  expect([...target.querySelectorAll("[data-episode-file-field]")].map(el => el.getAttribute("data-episode-file-field"))).toEqual(["nativeSubsPath"]);
  expect(target.querySelector("h3")?.textContent).toContain("reference.srt");
  await unmount(component);
  target.remove();
});

it("passes the clicked file field to the context menu without also opening a row menu", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const context = vi.fn();
  const component = mount(EpisodeTable, { target, props: {
    episodes: [{ id: 1, targetSubsPath: "original.srt", nativeSubsPath: "reference.srt", mediaPath: "movie.mp4" }],
    showSnackbar: vi.fn(), onSwapAll: vi.fn(), onSwap: vi.fn(), onEdit: vi.fn(),
    onMediaSettings: vi.fn(), onRemove: vi.fn(), onContextMenu: context, onClearAll: vi.fn(),
  } });
  await tick();
  target.querySelector('[title="reference.srt"]')!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  expect(context).toHaveBeenCalledTimes(1);
  expect(context).toHaveBeenCalledWith(expect.any(MouseEvent), 0, "nativeSubsPath");
  await unmount(component);
  target.remove();
});

it("keeps the context target highlighted while the pointer moves away", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(EpisodeTable, { target, props: {
    episodes: [{ id: 1, targetSubsPath: "original.srt", nativeSubsPath: "reference.srt", mediaPath: "movie.mp4" }],
    contextTarget: { idx: 0, field: "nativeSubsPath" },
    showSnackbar: vi.fn(), onSwapAll: vi.fn(), onSwap: vi.fn(), onEdit: vi.fn(),
    onMediaSettings: vi.fn(), onRemove: vi.fn(), onContextMenu: vi.fn(), onClearAll: vi.fn(),
  } });
  await tick();
  const reference = target.querySelector('[title="reference.srt"]')!;
  reference.dispatchEvent(new MouseEvent("mouseleave", { bubbles: true }));
  await tick();
  expect(reference.classList.contains("context-selected")).toBe(true);
  expect(target.querySelectorAll(".context-selected")).toHaveLength(1);
  await unmount(component);
  target.remove();
});

it("selects and clears a per-episode note type override", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const noteTypes = listNoteTypes();
  const customNoteType = noteTypes.find((noteType) => noteType.id !== "default")!;
  const editingEpisode: EpisodeEntry = { id: 3, targetSubsPath: "ep3.srt", nativeSubsPath: "", mediaPath: "", mediaType: "none" };
  const onNoteTypeChange = vi.fn();
  const component = mount(EpisodeEditorModal, { target, props: {
    editingEpisode,
    activeNoteType: predefinedNoteTypeForLanguage("en"),
    noteTypeOptions: [
      { value: "default", label: "Automatic · English_Vesta", displayLabel: "English_Vesta", icon: "🇬🇧" },
      ...noteTypes.filter((noteType) => noteType.id !== "default").map((noteType) => ({ value: noteType.id, label: noteType.name })),
    ],
    getEpisodeNoteType: (episode) => episode.noteTypeId ? noteTypes.find((noteType) => noteType.id === episode.noteTypeId)! : predefinedNoteTypeForLanguage("en"),
    episodeIndex: 2,
    onNoteTypeChange,
    dropMediaHint: "", dropSubtitleHint: "", onClose: vi.fn(), onRevert: vi.fn(),
    onSelectFile: vi.fn(), onClearField: vi.fn(), onExpandField: vi.fn(),
  } });
  await tick();
  const select = target.querySelector('input[aria-label="Note type"]') as HTMLInputElement;
  select.dispatchEvent(new FocusEvent("focus"));
  await tick();
  target.querySelectorAll(".searchable-select-option")[1].dispatchEvent(new MouseEvent("click", { bubbles: true }));
  await tick();
  expect(editingEpisode.noteTypeId).toBe(customNoteType.id);
  expect(onNoteTypeChange).toHaveBeenCalledWith(2, customNoteType.id);
  target.querySelector('input[aria-label="Note type"]')!.dispatchEvent(new FocusEvent("focus"));
  await tick();
  target.querySelectorAll(".searchable-select-option")[0].dispatchEvent(new MouseEvent("click", { bubbles: true }));
  await tick();
  expect(editingEpisode.noteTypeId).toBeUndefined();
  expect(onNoteTypeChange).toHaveBeenLastCalledWith(2, "default");
  await unmount(component);
  target.remove();
});

it("shows the resolved automatic note type in the episode row and opens its editor", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const noteType = predefinedNoteTypeForLanguage("en");
  const onEdit = vi.fn();
  const component = mount(EpisodeTable, { target, props: {
    episodes: [{ id: 1, targetSubsPath: "original.srt", nativeSubsPath: "", mediaPath: "", noteTypeId: undefined }],
    noteTypeOptions: [{ value: "default", label: `Automatic · ${noteType.name}`, displayLabel: noteType.name, icon: "🇬🇧" }],
    getEpisodeNoteType: () => noteType,
    showSnackbar: vi.fn(), onSwapAll: vi.fn(), onSwap: vi.fn(), onEdit,
    onMediaSettings: vi.fn(), onRemove: vi.fn(), onContextMenu: vi.fn(), onClearAll: vi.fn(),
  } });
  await tick();
  const noteTypeButton = target.querySelector(`button[aria-label="Note type: ${noteType.name}"]`) as HTMLButtonElement;
  expect(noteTypeButton.textContent).toContain(noteType.name);
  expect(noteTypeButton.title).toContain(`Automatic · ${noteType.name}`);
  noteTypeButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  expect(onEdit).toHaveBeenCalledWith(0);
  await unmount(component);
  target.remove();
});

it("keeps the automatic option resolved per episode when a manual override is selected", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const noteTypes = listNoteTypes();
  const manualType = noteTypes.find((noteType) => noteType.id !== "default" && noteType.id !== "predef:en")!;
  const automaticType = predefinedNoteTypeForLanguage("en");
  const editingEpisode: EpisodeEntry = {
    id: 4, targetSubsPath: "episode-en.srt", nativeSubsPath: "", mediaPath: "", mediaType: "none", noteTypeId: manualType.id,
  };
  const component = mount(EpisodeEditorModal, { target, props: {
    editingEpisode,
    activeNoteType: manualType,
    noteTypeOptions: [
      { value: "default", label: `Automatic · ${manualType.name}`, displayLabel: manualType.name },
      ...noteTypes.filter((noteType) => noteType.id !== "default").map((noteType) => ({ value: noteType.id, label: noteType.name })),
    ],
    getEpisodeNoteType: (episode) => episode.noteTypeId ? noteTypes.find((noteType) => noteType.id === episode.noteTypeId)! : automaticType,
    dropMediaHint: "", dropSubtitleHint: "", onClose: vi.fn(), onRevert: vi.fn(),
    onSelectFile: vi.fn(), onClearField: vi.fn(), onExpandField: vi.fn(),
  } });
  await tick();
  const select = target.querySelector('input[aria-label="Note type"]') as HTMLInputElement;
  select.dispatchEvent(new FocusEvent("focus"));
  await tick();
  expect(target.querySelectorAll(".searchable-select-option")[0].textContent).toContain(`Automatic · ${automaticType.name}`);
  expect(target.querySelectorAll(".searchable-select-option")[0].textContent).not.toContain(manualType.name);
  await unmount(component);
  target.remove();
});
