// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import RefineTab from "./RefineTab.svelte";
import { invokeCommand } from "$lib/services/tauriClient";

vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn(async (command: string) => {
  if (command === "refine_load_file") return [{ id: "note-1", expression: "hello", meaning: "ciao", notes: "original" }];
  if (command === "refine_save_file") return true;
  return undefined;
}) }));
vi.mock("$lib/utils/dragDrop", () => ({ setupWebviewDragDrop: vi.fn(() => () => {}) }));
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn(), guardedSave: vi.fn() }));

const instances: ReturnType<typeof mount>[] = [];
afterEach(async () => {
  for (const instance of instances.splice(0)) await unmount(instance);
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

async function render(active = true) {
  const target = document.createElement("div");
  document.body.append(target);
  instances.push(mount(RefineTab, { target, props: { active } }));
  await tick();
  await (window as any).loadRefineFile("deck.apkg");
  await tick();
  return target;
}

function editNotes(target: HTMLElement, value: string) {
  const textarea = target.querySelector<HTMLTextAreaElement>("#card-notes")!;
  textarea.value = value;
  textarea.dispatchEvent(new Event("input", { bubbles: true }));
  return textarea;
}

it("undoes and redoes notes outside the editor while preserving the save baseline", async () => {
  const target = await render();
  const textarea = editNotes(target, "changed");
  await tick();
  expect(textarea.value).toBe("changed");

  const undo = target.querySelector<HTMLButtonElement>('button[aria-label="Undo"]')!;
  const redo = target.querySelector<HTMLButtonElement>('button[aria-label="Redo"]')!;
  expect(undo.disabled).toBe(false);
  undo.click();
  await tick();
  expect(target.querySelector<HTMLTextAreaElement>("#card-notes")!.value).toBe("original");
  expect(redo.disabled).toBe(false);

  redo.click();
  await tick();
  expect(target.querySelector<HTMLTextAreaElement>("#card-notes")!.value).toBe("changed");
  const overwrite = [...target.querySelectorAll("button")].find(button => button.textContent?.toLowerCase().includes("overwrite"));
  expect(overwrite).toBeTruthy();
  overwrite!.click();
  await tick();
  const save = vi.mocked(invokeCommand).mock.calls.find(([command]) => command === "refine_save_file");
  expect(save?.[1]).toMatchObject({ updates: [{ id: "note-1", notes: "changed" }] });
});

it("leaves editor Ctrl+Z to CodeEditor and keeps document undo separate", async () => {
  const target = await render();
  const textarea = editNotes(target, "changed");
  await tick();
  const event = new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true, cancelable: true });
  textarea.dispatchEvent(event);
  await tick();
  expect(event.defaultPrevented).toBe(true);
  // CodeEditor owns text-field Ctrl+Z; its mounted initial history starts empty
  // before the deck is loaded, so its native undo returns to that local baseline.
  expect(target.querySelector<HTMLTextAreaElement>("#card-notes")!.value).toBe("");
  expect(target.querySelector<HTMLButtonElement>('button[aria-label="Undo"]')!.disabled).toBe(false);
});

it("does not apply document shortcuts when the tab is hidden", async () => {
  const target = await render(false);
  const textarea = editNotes(target, "changed");
  textarea.blur();
  await tick();
  const event = new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true, cancelable: true });
  document.body.dispatchEvent(event);
  await tick();
  expect(event.defaultPrevented).toBe(false);
  expect(target.querySelector<HTMLTextAreaElement>("#card-notes")!.value).toBe("changed");
});
