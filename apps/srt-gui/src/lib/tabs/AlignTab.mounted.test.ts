// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import AlignLifecycleHarness from "$lib/test/fixtures/AlignLifecycleHarness.svelte";
import { guardedOpen } from "$lib/utils/dialogGuard";

vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn().mockResolvedValue(null), guardedSave: vi.fn().mockResolvedValue(null) }));
vi.mock("$lib/utils/dragDrop", () => ({ setupWebviewDragDrop: vi.fn(() => () => {}) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock("@tauri-apps/plugin-fs", () => ({ writeTextFile: vi.fn() }));

type HarnessInstance = { setActive: (value: boolean) => void };
const mounted: HarnessInstance[] = [];
afterEach(async () => {
  for (const component of mounted.splice(0)) await unmount(component);
  document.body.replaceChildren();
  vi.clearAllMocks();
});

async function render() {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(AlignLifecycleHarness, { target, intro: false });
  mounted.push(component);
  await tick();
  return { target, component };
}
function openKey(extra: KeyboardEventInit = {}) {
  return new KeyboardEvent("keydown", { key: "o", ctrlKey: true, bubbles: true, cancelable: true, ...extra });
}

it("handles a bubbling alignment shortcut once and releases the listener on unmount", async () => {
  const { target, component } = await render();
  target.querySelector('[role="region"]')!.dispatchEvent(openKey());
  expect(guardedOpen).toHaveBeenCalledOnce();
  await unmount(component);
  mounted.splice(mounted.indexOf(component), 1);
  window.dispatchEvent(openKey());
  expect(guardedOpen).toHaveBeenCalledOnce();
});

it("ignores shortcuts while hidden and resumes after reactivation", async () => {
  const { component } = await render();
  component.setActive(false);
  await tick();
  window.dispatchEvent(openKey());
  expect(guardedOpen).not.toHaveBeenCalled();
  component.setActive(true);
  await tick();
  window.dispatchEvent(openKey());
  expect(guardedOpen).toHaveBeenCalledOnce();
});

it("leaves consumed, composing and editable events to their owner", async () => {
  await render();
  const consumed = openKey();
  consumed.preventDefault();
  window.dispatchEvent(consumed);
  window.dispatchEvent(openKey({ isComposing: true }));
  for (const tag of ["input", "textarea", "select", "div"]) {
    const field = document.createElement(tag);
    if (tag === "div") { field.setAttribute("contenteditable", "true"); field.tabIndex = 0; }
    document.body.append(field);
    field.focus();
    field.dispatchEvent(openKey());
    field.remove();
  }
  expect(guardedOpen).not.toHaveBeenCalled();
});
