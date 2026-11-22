// @vitest-environment happy-dom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "flashcard_check_deps") return true;
    if (cmd === "flashcard_get_cpu_count") return 4;
    if (cmd === "flashcard_check_dir_exists") return true;
    if (cmd === "flashcard_list_audio_tracks") return [{ index: 0, language: "ja", title: "Japanese", codec: "aac" }];
    if (cmd === "flashcard_list_fonts" || cmd === "flashcard_parse_subtitles") return [];
    return null;
  }),
  notify: vi.fn(),
  drop: null as null | ((paths: string[], position?: { x: number; y: number }) => void),
}));

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: mocks.invoke,
}));
vi.mock("$lib/utils/defaultOutputDirectory", () => ({
  defaultOutputDirectory: vi.fn().mockResolvedValue("/out"),
}));
vi.mock("$lib/utils/dialogGuard", () => ({
  guardedOpen: vi.fn().mockResolvedValue(null),
}));
vi.mock("$lib/utils/dragDrop", () => ({
  setupWebviewDragDrop: vi.fn((options) => {
    mocks.drop = options.onDrop;
    return () => {};
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("$lib/stores/snackbarStore.svelte", async () => {
  const actual = await vi.importActual<any>("$lib/stores/snackbarStore.svelte");
  return {
    ...actual,
    snackbar: { show: mocks.notify },
  };
});

import FlashcardsTab from "./FlashcardsTab.svelte";
import { HandoffBox } from "./flashcardsHandoffState.svelte";
import { generationStore } from "$lib/stores/generationStore.svelte";

const mounted: ReturnType<typeof mount>[] = [];

async function settle(turns = 6) {
  for (let i = 0; i < turns; i++) {
    await Promise.resolve();
    await tick();
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.invoke.mockImplementation(async (cmd: string) => {
    if (cmd === "flashcard_check_deps") return true;
    if (cmd === "flashcard_get_cpu_count") return 4;
    if (cmd === "flashcard_check_dir_exists") return true;
    if (cmd === "flashcard_list_audio_tracks") return [{ index: 0, language: "ja", title: "Japanese", codec: "aac" }];
    if (cmd === "flashcard_list_fonts" || cmd === "flashcard_parse_subtitles") return [];
    return null;
  });
});

afterEach(async () => {
  generationStore.isProcessing = false;
  for (const component of mounted.splice(0)) await unmount(component);
  document.body.replaceChildren();
});

describe("FlashcardsTab plugin handoff", () => {
  it("prefills video and subtitle on a single episode slot, probes audio tracks, and handles audio + repeated handoffs cleanly", async () => {
    const target = document.createElement("div");
    document.body.append(target);

    const handoff = new HandoffBox();
    const component = mount(FlashcardsTab, {
      target,
      props: {
        active: true,
        get pluginHandoff() {
          return handoff.current;
        },
      },
      intro: false,
    });
    mounted.push(component);
    await settle();

    // 1. Video + subtitle handoff
    handoff.current = { nonce: 1, videoPath: "/tmp/Episode01.mp4", subtitlePath: "/tmp/Episode01.ja.srt" };
    await settle();

    expect(target.textContent).toContain("Episode01.ja.srt");
    expect(target.textContent).toContain("Episode01.mp4");
    expect(mocks.invoke).toHaveBeenCalledWith("flashcard_list_audio_tracks", { path: "/tmp/Episode01.mp4" });

    // 2. Audio + new subtitle handoff clears video probe and does not probe audio tracks
    mocks.invoke.mockClear();
    handoff.current = { nonce: 2, videoPath: "/tmp/Podcast02.mp3", subtitlePath: "/tmp/Podcast02.es.srt" };
    await settle();

    expect(target.textContent).toContain("Podcast02.es.srt");
    expect(target.textContent).toContain("Podcast02.mp3");
    expect(target.textContent).not.toContain("Episode01.mp4");
    expect(mocks.invoke).not.toHaveBeenCalledWith("flashcard_list_audio_tracks", expect.anything());

    // 3. Repeated handoff with a new video path updates cleanly
    handoff.current = { nonce: 3, videoPath: "/tmp/Film03.mkv", subtitlePath: "/tmp/Film03.fr.srt" };
    await settle();

    expect(target.textContent).toContain("Film03.fr.srt");
    expect(target.textContent).toContain("Film03.mkv");
    expect(mocks.invoke).toHaveBeenCalledWith("flashcard_list_audio_tracks", { path: "/tmp/Film03.mkv" });
  });

  it("preserves existing series episodes when updating episode 1 via handoff", async () => {
    const target = document.createElement("div");
    document.body.append(target);

    const handoff = new HandoffBox();
    const component = mount(FlashcardsTab, {
      target,
      props: {
        active: true,
        get pluginHandoff() {
          return handoff.current;
        },
      },
      intro: false,
    });
    mounted.push(component);
    await settle();

    mocks.drop!([
      "/series/S01E01.mkv",
      "/series/S01E01.ja.srt",
      "/series/S01E02.mkv",
      "/series/S01E02.ja.srt",
    ]);
    await settle();

    expect(target.textContent).toContain("S01E01.ja.srt");
    expect(target.textContent).toContain("S01E02.ja.srt");

    handoff.current = { nonce: 1, videoPath: "/series/S01E01_remux.mp4", subtitlePath: "/series/S01E01_alt.ja.srt" };
    await settle();

    expect(target.textContent).toContain("S01E01_alt.ja.srt");
    expect(target.textContent).toContain("S01E01_remux.mp4");
    expect(target.textContent).toContain("S01E02.ja.srt");
    expect(target.textContent).toContain("S01E02.mkv");
  });

  it("defers a plugin handoff arrived during busy generation until generation finishes", async () => {
    const target = document.createElement("div");
    document.body.append(target);

    const handoff = new HandoffBox();
    const component = mount(FlashcardsTab, {
      target,
      props: {
        active: true,
        get pluginHandoff() {
          return handoff.current;
        },
      },
      intro: false,
    });
    mounted.push(component);
    await settle();

    handoff.current = { nonce: 1, videoPath: "/tmp/First.mp4", subtitlePath: "/tmp/First.ja.srt" };
    await settle();
    expect(target.textContent).toContain("First.ja.srt");

    // Simulate active generation when a new handoff arrives
    generationStore.isProcessing = true;
    await settle();
    handoff.current = { nonce: 2, videoPath: "/tmp/Deferred.mkv", subtitlePath: "/tmp/Deferred.ja.srt" };
    await settle();

    // Must not overwrite during busy generation
    expect(target.textContent).toContain("First.ja.srt");
    expect(target.textContent).not.toContain("Deferred.ja.srt");

    // Once generation ends, the pending handoff is applied automatically
    generationStore.isProcessing = false;
    await settle();

    expect(target.textContent).toContain("Deferred.ja.srt");
    expect(target.textContent).toContain("Deferred.mkv");
    expect(target.textContent).not.toContain("First.mp4");
  });

  it("clears the previous episode video when a new subtitle arrives with absent or unrecognized media", async () => {
    const target = document.createElement("div");
    document.body.append(target);

    const handoff = new HandoffBox();
    const component = mount(FlashcardsTab, {
      target,
      props: {
        active: true,
        get pluginHandoff() {
          return handoff.current;
        },
      },
      intro: false,
    });
    mounted.push(component);
    await settle();

    // Initial paired episode
    handoff.current = { nonce: 1, videoPath: "/tmp/Ep1.mp4", subtitlePath: "/tmp/Ep1.ja.srt" };
    await settle();
    expect(target.textContent).toContain("Ep1.ja.srt");
    expect(target.textContent).toContain("Ep1.mp4");

    // New subtitle with no video -> old Ep1 video must not stick around
    handoff.current = { nonce: 2, videoPath: "", subtitlePath: "/tmp/Ep2.ja.srt" };
    await settle();
    expect(target.textContent).toContain("Ep2.ja.srt");
    expect(target.textContent).not.toContain("Ep1.mp4");

    // Re-pair video, then send new subtitle with unrecognized media extension -> old video must be cleared
    handoff.current = { nonce: 3, videoPath: "/tmp/Ep2.mkv", subtitlePath: "/tmp/Ep2.ja.srt" };
    await settle();
    expect(target.textContent).toContain("Ep2.mkv");

    handoff.current = { nonce: 4, videoPath: "/tmp/Ep3.unknown", subtitlePath: "/tmp/Ep3.ja.srt" };
    await settle();
    expect(target.textContent).toContain("Ep3.ja.srt");
    expect(target.textContent).not.toContain("Ep2.mkv");
  });
});
