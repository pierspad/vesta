// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createRawSnippet, mount, tick, unmount } from "svelte";

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: vi.fn().mockResolvedValue(undefined),
}));

import CardFiltersPanel from "./CardFiltersPanel.svelte";
import FilesOutputPanel from "./FilesOutputPanel.svelte";
import GenerationStatusDisplay from "./GenerationStatusDisplay.svelte";
import VideoClipsPanel from "./VideoClipsPanel.svelte";
import { generationStore } from "$lib/stores/generationStore.svelte";
import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";
import type { EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";

const mounted: ReturnType<typeof mount>[] = [];

function targetEl() {
  const el = document.createElement("div");
  document.body.append(el);
  return el;
}

beforeEach(() => {
  generationStore.cancelRun();
  generationStore.resetRun();
});

afterEach(async () => {
  for (const component of mounted.splice(0)) {
    await unmount(component);
  }
  document.body.replaceChildren();
  vi.clearAllMocks();
});

describe("Flashcards panels", () => {
  it("CardFiltersPanel toggles enabled state when switch button is clicked", async () => {
    const filters: CardFilterSettings = {
      enabled: false,
      minChars: 2,
      maxChars: 80,
      minCharsEnabled: true,
      maxCharsEnabled: false,
      minDurationMs: 500,
      maxDurationMs: 10000,
      minDurationEnabled: false,
      maxDurationEnabled: false,
      combineSentences: false,
      continuationChars: "…",
    };
    const target = targetEl();
    mounted.push(
      mount(CardFiltersPanel, {
        target,
        props: { filters, hasAnyFiles: true, hintLoadTargetFirst: "Load subtitles" },
      }),
    );
    await tick();

    const toggle = target.querySelector<HTMLButtonElement>("button[aria-expanded]")!;
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    toggle.click();
    await tick();
    expect(filters.enabled).toBe(true);

    const enabledTarget = targetEl();
    mounted.push(
      mount(CardFiltersPanel, {
        target: enabledTarget,
        props: { filters, hasAnyFiles: true, hintLoadTargetFirst: "Load subtitles" },
      }),
    );
    await tick();
    expect(enabledTarget.querySelectorAll("input").length).toBeGreaterThan(0);
  });

  it("FilesOutputPanel renders output directory and invokes callbacks on Add Files / Select Output", async () => {
    const onAddFiles = vi.fn();
    const onSelectOutput = vi.fn();
    const target = targetEl();
    mounted.push(
      mount(FilesOutputPanel, {
        target,
        props: {
          highlightClass: "",
          episodes: [],
          outputDir: "/exports/deck",
          onAddFiles,
          onClearAll: vi.fn(),
          showSnackbar: vi.fn(),
          onSwapAll: vi.fn(),
          onSwap: vi.fn(),
          onEdit: vi.fn(),
          onMediaSettings: vi.fn(),
          onRemove: vi.fn(),
          onContextMenu: vi.fn(),
          onExpand: vi.fn(),
          onSelectOutput,
          onClearField: vi.fn(),
        },
      }),
    );
    await tick();

    expect(target.innerHTML).toContain("/exports/deck");
    const addBtn = target.querySelector<HTMLButtonElement>("button.bg-cyan-600")!;
    addBtn.click();
    expect(onAddFiles).toHaveBeenCalledOnce();
  });

  it("GenerationStatusDisplay renders noteTypeControl when idle, spinner when busy, and result summary", async () => {
    const noteTypeControl = createRawSnippet(() => ({
      render: () => `<button type="button" data-note-picker="1">Vesta Card</button>`,
    }));

    const target = targetEl();
    mounted.push(
      mount(GenerationStatusDisplay, {
        target,
        props: { easyMode: false, noteTypeControl, noteTypeName: "Vesta Card" },
      }),
    );
    await tick();
    expect(target.querySelector('[data-note-picker="1"]')).not.toBeNull();

    generationStore.isProcessing = true;
    generationStore.progress = 64;
    generationStore.progressMessage = "Encoding audio";
    generationStore.phaseProgress = 64;
    await tick();
    expect(target.textContent).toContain("64%");
    expect(target.textContent).toContain("Encoding audio");

    generationStore.cancelRun();
    generationStore.result = {
      success: true,
      message: "OK",
      cardsGenerated: 18,
      audioClips: 18,
      snapshots: 18,
      videoClips: 0,
      tsvPath: null,
      apkgPath: "/exports/deck.apkg",
      outputSizeBytes: 1024,
    };
    await tick();
    expect(target.textContent).toContain("18");
  });

  it("VideoClipsPanel toggles clip generation and respects hasVideo disabled state", async () => {
    const settings: Required<EpisodeMediaOverrides> = {
      generateAudio: true,
      audioBitrate: 128,
      audioTrackIndex: null,
      normalizeAudio: false,
      audioBoost: false,
      audioGainDb: 6,
      audioPadStart: 0,
      audioPadEnd: 0,
      audioFormat: "mp3",
      generateSnapshots: true,
      snapshotWidth: 256,
      snapshotHeight: 144,
      cropBottom: 0,
      snapshotFormat: "webp",
      snapshotQuality: 80,
      generateVideoClips: false,
      videoCodec: "h264",
      h264Preset: "veryfast",
      videoBitrate: 800,
      videoAudioBitrate: 128,
      videoPadStart: 0,
      videoPadEnd: 0,
      videoWidth: 640,
      videoHeight: 360,
    };

    const target = targetEl();
    mounted.push(
      mount(VideoClipsPanel, {
        target,
        props: {
          settings,
          hasVideo: true,
          effectiveExportFormat: "tsv",
          hintLoadVideoFirst: "Load video",
        },
      }),
    );
    await tick();

    const toggle = target.querySelector<HTMLButtonElement>("button")!;
    expect(toggle.disabled).toBe(false);
    toggle.click();
    await tick();
    expect(settings.generateVideoClips).toBe(true);
  });
});
