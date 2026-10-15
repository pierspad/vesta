// @vitest-environment happy-dom

import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn() }));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { invokeCommand } from "$lib/services/tauriClient";
import { previewStore, type PreviewLine } from "$lib/stores/previewStore.svelte";
import { uiMode } from "$lib/stores/uiModeStore.svelte";
import FlashcardsPreviewModal from "./FlashcardsPreviewModal.svelte";
import FlashcardsPreviewModalLifecycleHarness from "$lib/test/fixtures/FlashcardsPreviewModalLifecycleHarness.svelte";

const mounted: Record<string, any>[] = [];
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function line(text: string): PreviewLine {
  return { index: 1, start_ms: 0, end_ms: 1000, subs1_text: text, subs2_text: null, active: true, leading_context: [], trailing_context: [] };
}
async function render(active = true, mediaPath: string | null = null) {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(FlashcardsPreviewModal, {
    target,
    intro: false,
    props: { mediaPath, mediaType: mediaPath ? "video" : "none", nativeSubsPath: "", active },
  });
  mounted.push(component);
  await tick();
  return { target, component };
}

async function renderLifecycleHarness() {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(FlashcardsPreviewModalLifecycleHarness, { target, intro: false });
  mounted.push(component);
  await tick();
  return { target, component };
}

afterEach(async () => {
  while (mounted.length) await unmount(mounted.pop()!);
  previewStore.close();
  previewStore.lines = [];
  uiMode.expertMode = false;
  vi.mocked(invokeCommand).mockReset();
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

it("Escape closes the modal from its focused search field", async () => {
  previewStore.visible = true;
  const { target } = await render();
  const search = target.querySelector<HTMLInputElement>('input[type="text"]')!;
  search.focus();
  search.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
  await tick();
  expect(previewStore.visible).toBe(false);
});

it("deactivating the owning tab closes its mounted preview", async () => {
  const staleLoad = deferred<PreviewLine[]>();
  vi.mocked(invokeCommand).mockReturnValueOnce(staleLoad.promise);
  previewStore.lines = [line("Edited line")];
  previewStore.toggleLineActive(previewStore.lines[0]);
  previewStore.visible = true;
  const { target } = await renderLifecycleHarness();
  const load = previewStore.load({ target_subs_path: "episode.srt" });
  await tick();
  target.querySelector<HTMLButtonElement>('[data-testid="deactivate-tab"]')!.click();
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(target.querySelector('[data-testid="owner-state"]')?.textContent).toBe("inactive");
  expect(previewStore.visible).toBe(false);
  expect(previewStore.loading).toBe(false);
  staleLoad.resolve([line("Late result")]);
  await load;
  expect(previewStore.lines[0].subs1_text).toBe("Edited line");
  expect(previewStore.lines[0].active).toBe(false);
});

it("close and reopen keeps the fresh preview when the older IPC completes late", async () => {
  const oldResult = deferred<PreviewLine[]>();
  const freshResult = deferred<PreviewLine[]>();
  vi.mocked(invokeCommand).mockReturnValueOnce(oldResult.promise).mockReturnValueOnce(freshResult.promise);

  const firstModal = await render();
  const oldLoad = previewStore.load({ target_subs_path: "episode.srt" });
  await tick();
  firstModal.target.querySelector<HTMLButtonElement>(".dialog-close-button")!.click();
  await tick();
  expect(previewStore.visible).toBe(false);
  mounted.splice(mounted.indexOf(firstModal.component), 1);
  await unmount(firstModal.component);

  const freshLoad = previewStore.load({ target_subs_path: "episode.srt" });
  const reopened = await render();
  freshResult.resolve([line("Fresh subtitle")]);
  await freshLoad;
  await tick();
  expect(reopened.target.textContent).toContain("Fresh subtitle");

  oldResult.reject(new Error("late stale failure"));
  await oldLoad;
  await tick();
  expect(previewStore.visible).toBe(true);
  expect(previewStore.lines[0].subs1_text).toBe("Fresh subtitle");
  expect(reopened.target.textContent).not.toContain("Stale subtitle");
});

it("removes the modal undo key listener on unmount", async () => {
  previewStore.visible = true;
  previewStore.lines = [line("Undo target")];
  const { component } = await render();
  previewStore.toggleLineActive(previewStore.lines[0]);
  expect(previewStore.lines[0].active).toBe(false);
  const closeButton = document.querySelector<HTMLButtonElement>(".dialog-close-button")!;
  closeButton.focus();
  closeButton.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true }));
  expect(previewStore.lines[0].active).toBe(true);
  previewStore.toggleLineActive(previewStore.lines[0]);
  expect(previewStore.lines[0].active).toBe(false);
  mounted.splice(mounted.indexOf(component), 1);
  await unmount(component);

  window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true }));
  expect(previewStore.lines[0].active).toBe(false);
});

it("undo works from a card control while Ctrl+Z in search stays native", async () => {
  uiMode.expertMode = true;
  previewStore.visible = true;
  previewStore.lines = [line("Undo target")];
  const { target } = await render();
  const row = target.querySelector<HTMLTableRowElement>("tbody tr")!;
  row.dispatchEvent(new MouseEvent("auxclick", { button: 1, bubbles: true, cancelable: true }));
  await tick();
  expect(previewStore.lines[0].active).toBe(false);

  const search = target.querySelector<HTMLInputElement>('input[type="text"]')!;
  search.focus();
  const nativeUndo = new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true, cancelable: true });
  search.dispatchEvent(nativeUndo);
  expect(nativeUndo.defaultPrevented).toBe(false);
  expect(previewStore.lines[0].active).toBe(false);

  previewStore.openContextMenu(new MouseEvent("contextmenu"), previewStore.lines[0]);
  await tick();
  const cardControl = target.querySelector<HTMLButtonElement>(".fixed.z-50 button:last-of-type")!;
  cardControl.focus();
  const cardUndo = new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true, cancelable: true });
  cardControl.dispatchEvent(cardUndo);
  expect(cardUndo.defaultPrevented).toBe(true);
  expect(previewStore.lines[0].active).toBe(true);
});

it("attaches the mounted player before media discovery completes", async () => {
  previewStore.visible = true;
  const discovery = deferred<[number, string]>();
  vi.mocked(invokeCommand).mockReturnValueOnce(discovery.promise);
  const play = vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue();
  vi.spyOn(HTMLMediaElement.prototype, "load").mockImplementation(() => {});
  previewStore.lines = [line("Playback target")];
  await render(true, "/tmp/clip.mp4");
  const playback = previewStore.playLine(previewStore.lines[0], "/tmp/clip.mp4");
  await tick();
  expect(previewStore.playerElement).toBeInstanceOf(HTMLVideoElement);
  discovery.resolve([8765, "token"]);
  await playback;
  expect(play).toHaveBeenCalledOnce();
  expect(previewStore.isPlaying).toBe(true);
});

it("unmounting a modal invalidates its pending load", async () => {
  const pending = deferred<PreviewLine[]>();
  vi.mocked(invokeCommand).mockReturnValueOnce(pending.promise);
  const load = previewStore.load({ target_subs_path: "episode.srt" });
  const { component } = await render();
  await unmount(component);
  mounted.splice(mounted.indexOf(component), 1);
  pending.resolve([line("Late subtitle")]);
  await load;
  expect(previewStore.visible).toBe(false);
  expect(previewStore.loading).toBe(false);
  expect(previewStore.lines).toEqual([]);
});

it("does not close a replacement dialog when its predecessor unmounts", async () => {
  previewStore.visible = true;
  const older = await render();
  const newer = await render();
  await unmount(older.component);
  mounted.splice(mounted.indexOf(older.component), 1);
  expect(previewStore.visible).toBe(true);
  expect(newer.target.querySelector('[role="dialog"]')).not.toBeNull();
});
