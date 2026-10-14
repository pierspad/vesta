// @vitest-environment happy-dom

import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn() }));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { invokeCommand } from "$lib/services/tauriClient";
import { previewStore, type PreviewLine } from "$lib/stores/previewStore.svelte";
import FlashcardsPreviewModal from "./FlashcardsPreviewModal.svelte";

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
async function render() {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(FlashcardsPreviewModal, {
    target,
    intro: false,
    props: { mediaPath: null, mediaType: "none", nativeSubsPath: "" },
  });
  mounted.push(component);
  await tick();
  return { target, component };
}

afterEach(async () => {
  while (mounted.length) await unmount(mounted.pop()!);
  previewStore.close();
  previewStore.lines = [];
  vi.mocked(invokeCommand).mockReset();
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
  previewStore.lines = [line("Undo target")];
  const { component } = await render();
  previewStore.toggleLineActive(previewStore.lines[0]);
  expect(previewStore.lines[0].active).toBe(false);
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true }));
  expect(previewStore.lines[0].active).toBe(true);
  previewStore.toggleLineActive(previewStore.lines[0]);
  expect(previewStore.lines[0].active).toBe(false);
  mounted.splice(mounted.indexOf(component), 1);
  await unmount(component);

  window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true }));
  expect(previewStore.lines[0].active).toBe(false);
});
