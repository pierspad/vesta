// @vitest-environment happy-dom
import { expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import FlashcardsFooterBar from "./FlashcardsFooterBar.svelte";
import { listNoteTypes, predefinedNoteTypeForLanguage } from "$lib/types/noteTypes";
import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";

const noteTypes = listNoteTypes();
const defaultNoteType = predefinedNoteTypeForLanguage("en");
const noteTypeOptions = [
  { value: "default", label: `Automatic · ${defaultNoteType.name}`, displayLabel: defaultNoteType.name, icon: "🇬🇧" },
  ...noteTypes.filter((noteType) => noteType.id !== "default").map((noteType) => ({ value: noteType.id, label: noteType.name })),
];

function episode(id: number): EpisodeEntry {
  return { id, targetSubsPath: `episode-${id}.srt`, nativeSubsPath: "", mediaPath: "", mediaType: "none" };
}

function props(episodes: EpisodeEntry[]) {
  return {
    easyMode: true,
    activeNoteType: defaultNoteType,
    noteTypeOptions,
    automaticNoteType: true,
    selectedNoteTypeId: "default",
    selectNoteType: vi.fn(),
    episodes,
    isApkgSwitchEnabled: false,
    canRunFlashcards: false,
    generationPromptOpen: false,
    generationRequirements: [],
    generationRequirementsText: "",
    onOpenBottomContextMenu: vi.fn(),
    onPreview: vi.fn(),
    onStartGeneration: vi.fn(),
    onCancelGeneration: vi.fn(),
    onResetGeneration: vi.fn(),
    onPromptMissingRequirements: vi.fn(),
    onCloseGenerationPrompt: vi.fn(),
  };
}

it("shows the global note type selector for a single episode", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(FlashcardsFooterBar, { target, props: props([episode(1)]) });
  await tick();
  expect(target.querySelectorAll('input[aria-label="Note type"]')).toHaveLength(1);
  await unmount(component);
  target.remove();
});

it("replaces the global selector with per-film guidance for multiple episodes", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(FlashcardsFooterBar, { target, props: props([episode(1), episode(2)]) });
  await tick();
  expect(target.querySelectorAll('input[aria-label="Note type"]')).toHaveLength(0);
  expect(target.textContent).toContain("Each film uses its own note type");
  await unmount(component);
  target.remove();
});
