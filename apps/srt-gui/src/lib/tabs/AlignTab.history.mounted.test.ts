// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import AlignLifecycleHarness from "$lib/test/fixtures/AlignLifecycleHarness.svelte";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  open: vi.fn(),
  onDrop: undefined as ((paths: string[]) => void) | undefined,
}));

vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: mocks.invoke }));
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: mocks.open, guardedSave: vi.fn() }));
vi.mock("$lib/utils/dragDrop", () => ({
  setupWebviewDragDrop: vi.fn((options: { onDrop: (paths: string[]) => void }) => {
    mocks.onDrop = options.onDrop;
    return () => { mocks.onDrop = undefined; };
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock("@tauri-apps/plugin-fs", () => ({ writeTextFile: vi.fn() }));

const srt = (text: string) => `1\n00:00:01,000 --> 00:00:02,000\n${text}\n`;
const contents: Record<string, string> = {
  "old-target.srt": srt("Original line"),
  "old-source.srt": srt("Translated line"),
  "new-target.srt": srt("New document"),
};

const mounted: object[] = [];
afterEach(async () => {
  for (const component of mounted.splice(0)) await unmount(component);
  document.body.replaceChildren();
  mocks.invoke.mockReset();
  mocks.open.mockReset();
  mocks.onDrop = undefined;
});

async function renderWithPair() {
  mocks.invoke.mockImplementation(async (command: string, args?: { path?: string; srtPath?: string }) => {
    if (command === "read_subtitle_file") return contents[args?.path ?? ""];
    if (command === "sync_suggest_companion_subtitle_for_srt") return null;
    throw new Error(`Unexpected command: ${command}`);
  });
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(AlignLifecycleHarness, { target, intro: false });
  mounted.push(component);
  await tick();
  mocks.onDrop!("old-target.srt old-source.srt".split(" "));
  for (let i = 0; i < 5; i++) await tick();
  return { target, component };
}

function paths(target: HTMLElement) {
  return [...target.querySelectorAll<HTMLButtonElement>("button[title]")]
    .map(button => button.title)
    .filter(title => title.endsWith(".srt"));
}

function key(key: string, extra: KeyboardEventInit = {}) {
  return new KeyboardEvent("keydown", { key, ctrlKey: true, bubbles: true, cancelable: true, ...extra });
}

it("undoes and redoes a file swap with both paths and subtitle text intact", async () => {
  const { target } = await renderWithPair();
  expect(paths(target)).toEqual(["old-target.srt", "old-source.srt"]);
  expect([...target.querySelectorAll("textarea")].map(field => field.value)).toEqual(["Original line", "Translated line"]);

  target.querySelector<HTMLButtonElement>('button[aria-label*="Swap"]')!.click();
  await tick();
  expect(paths(target)).toEqual(["old-source.srt", "old-target.srt"]);
  expect([...target.querySelectorAll("textarea")].map(field => field.value)).toEqual(["Translated line", "Original line"]);

  window.dispatchEvent(key("z"));
  await tick();
  expect(paths(target)).toEqual(["old-target.srt", "old-source.srt"]);
  expect([...target.querySelectorAll("textarea")].map(field => field.value)).toEqual(["Original line", "Translated line"]);

  window.dispatchEvent(key("Z", { shiftKey: true }));
  await tick();
  expect(paths(target)).toEqual(["old-source.srt", "old-target.srt"]);
  expect([...target.querySelectorAll("textarea")].map(field => field.value)).toEqual(["Translated line", "Original line"]);
});

it("clears old document history on a new load and leaves textarea Ctrl+Z native", async () => {
  const { target } = await renderWithPair();
  const originalEditor = [...target.querySelectorAll<HTMLTextAreaElement>("textarea")].find(field => !field.readOnly && !field.disabled)!;
  originalEditor.focus();
  const nativeUndo = key("z");
  originalEditor.dispatchEvent(nativeUndo);
  expect(nativeUndo.defaultPrevented).toBe(false);
  expect(originalEditor.value).toBe("Translated line");
  originalEditor.blur();

  target.querySelector<HTMLButtonElement>('button[aria-label*="Swap"]')!.click();
  await tick();
  window.dispatchEvent(key("z"));
  await tick();

  mocks.open.mockResolvedValueOnce("new-target.srt");
  const currentTargetField = [...target.querySelectorAll<HTMLButtonElement>("button[title]")]
    .find(button => button.title === "old-target.srt")!;
  currentTargetField.parentElement!.querySelector<HTMLButtonElement>("button[aria-label]")!.click();
  for (let i = 0; i < 5; i++) await tick();
  expect(mocks.open).toHaveBeenCalledOnce();
  expect(paths(target)).toEqual(["new-target.srt"]);
  expect([...target.querySelectorAll("textarea")].map(field => field.value)).toEqual(["New document", ""]);
  window.dispatchEvent(key("z"));
  await tick();
  expect(paths(target)).toEqual(["new-target.srt"]);
});
