// @vitest-environment happy-dom
import { expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import EpisodeEditorModal from "./EpisodeEditorModal.svelte";
import { predefinedNoteTypeForLanguage } from "$lib/types/noteTypes";
import EpisodeTable from "$lib/components/EpisodeTable.svelte";

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
