// @vitest-environment happy-dom

import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";

const nativeWindow = vi.hoisted(() => ({
  scaleFactor: vi.fn().mockResolvedValue(1),
  setMinSize: vi.fn().mockResolvedValue(undefined),
  setSize: vi.fn().mockResolvedValue(undefined),
  onResized: vi.fn().mockResolvedValue(() => {}),
}));

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => nativeWindow }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock("$lib/utils/preloadTabs", () => ({ preloadTabs: () => vi.fn() }));
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: invokeMock }));
vi.mock("$lib/utils/defaultOutputDirectory", () => ({
  defaultOutputDirectory: vi.fn().mockResolvedValue("/out"),
}));
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn().mockResolvedValue(null) }));
vi.mock("$lib/utils/dragDrop", () => ({ setupWebviewDragDrop: vi.fn(() => () => {}) }));
vi.mock("$lib/config/vestaConfig", () => ({
  getItem: () => null,
  removeItem: vi.fn(),
  setItem: vi.fn(),
  flush: vi.fn(),
  isNewInstallation: () => false,
}));
vi.mock("$lib/stores/ankiStore.svelte", () => ({
  ankiStore: {
    checkConnection: vi.fn(),
    autoCardFont: false,
    embedCardFont: false,
    dictionaryLookup: false,
  },
}));
vi.mock("$lib/stores/aiStore.svelte", () => ({ aiStore: { killSwitchActive: false } }));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({
  snackbar: { message: null, show: vi.fn() },
  createSnackbarNotifier: () => vi.fn(),
}));
vi.mock("$lib/components/Sidebar.svelte", () => import("./fixtures/AppTestSidebar.svelte"));
vi.mock("$lib/components/ShortcutOverlay.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/components/AppContextMenu.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/components/Snackbar.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/tabs/ExtractTab.svelte", () => import("./fixtures/AppTestStub.svelte"));
vi.mock("$lib/tabs/TranslateTab.svelte", () => import("./fixtures/AppTabProbe.svelte"));
vi.mock("$lib/tabs/SettingsTab.svelte", () => import("./fixtures/AppTabProbe.svelte"));

import App from "../../App.svelte";

const mounted: any[] = [];

async function settle(turns = 6) {
  for (let i = 0; i < turns; i++) {
    await Promise.resolve();
    await tick();
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "flashcard_check_deps") return true;
    if (cmd === "flashcard_get_cpu_count") return 4;
    if (cmd === "flashcard_check_dir_exists") return true;
    if (cmd === "flashcard_list_audio_tracks") return [];
    if (cmd === "flashcard_parse_subtitles") return [];
    return null;
  });
});

afterEach(async () => {
  while (mounted.length) await unmount(mounted.pop()!);
  document.body.replaceChildren();
  delete (window as any).__appTabLifecycle;
  delete (window as any).changeTab;
});

it("switches to Flashcards and prefills videoPath & subtitlePath on vesta-plugin-open-flashcards", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(App, { target, intro: false });
  mounted.push(component);
  await settle();

  // Navigate away from Flashcards to Translate
  target.querySelector<HTMLButtonElement>('[data-testid="switch-translate"]')!.click();
  await vi.waitFor(() =>
    expect(target.querySelector('[data-testid="tab-probe"][data-active="true"]')).not.toBeNull()
  );

  // Dispatch plugin handoff event
  window.dispatchEvent(
    new CustomEvent("vesta-plugin-open-flashcards", {
      detail: {
        videoPath: "/media/Inception.mkv",
        subtitlePath: "/media/Inception.ja.srt",
      },
    })
  );
  await settle();

  // Translate probe is no longer active; Flashcards tab container is visible (no .hidden)
  expect(target.querySelector('[data-testid="tab-probe"][data-active="true"]')).toBeNull();
  const flashcardsWrapper = [...target.querySelectorAll<HTMLElement>(".flex-1 > div")].find((el) =>
    el.textContent?.includes("Inception.mkv")
  );
  expect(flashcardsWrapper).toBeDefined();
  expect(flashcardsWrapper?.classList.contains("hidden")).toBe(false);
  expect(flashcardsWrapper?.textContent).toContain("Inception.ja.srt");
  expect(invokeMock).toHaveBeenCalledWith("flashcard_list_audio_tracks", {
    path: "/media/Inception.mkv",
  });
});
