// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import CodeEditor from "./CodeEditor.svelte";
const instances: ReturnType<typeof mount>[] = [];
afterEach(async () => {
  for (const instance of instances.splice(0)) await unmount(instance);
  document.body.replaceChildren(); vi.unstubAllGlobals(); vi.restoreAllMocks();
});
async function render(wrap = false) {
  let nextId = 0;
  const frames = new Map<number, FrameRequestCallback>();
  vi.stubGlobal("requestAnimationFrame", vi.fn((callback: FrameRequestCallback) => {
    frames.set(++nextId, callback); return nextId;
  }));
  vi.stubGlobal("cancelAnimationFrame", vi.fn((id: number) => frames.delete(id)));
  const flushFrames = async () => {
    const pending = [...frames.values()]; frames.clear();
    for (const callback of pending) callback(0);
    await tick();
  };
  const target = document.createElement("div"); document.body.append(target);
  instances.push(mount(CodeEditor, { target, props: {
    value: Array.from({ length: 200 }, (_, i) => `"key${i}": ${i},`).join("\n"),
    language: "jsonc", wrap,
  } }));
  await tick(); await flushFrames();
  return { target, frames, flushFrames, textarea: target.querySelector("textarea")! };
}
it("coalesces scrolling without replacing highlighted text or measuring line layout", async () => {
  const { target, textarea, frames, flushFrames } = await render();
  const highlighted = target.querySelector(".code-editor-content > div")!;
  const gutter = target.querySelector(".code-editor-lines > div")!;
  const token = highlighted.querySelector("span")!;
  const layout = vi.spyOn(HTMLElement.prototype, "offsetHeight", "get");
  textarea.scrollTop = 120; textarea.scrollLeft = 16;
  textarea.dispatchEvent(new Event("scroll"));
  textarea.scrollTop = 240; textarea.scrollLeft = 32;
  textarea.dispatchEvent(new Event("scroll"));
  expect(frames.size).toBe(1); await flushFrames();
  expect(highlighted.parentElement!.scrollTop).toBe(240);
  expect(highlighted.parentElement!.scrollLeft).toBe(32);
  expect(gutter.parentElement!.scrollTop).toBe(240);
  expect((highlighted as HTMLElement).style.transform).toBe("");
  expect(highlighted.querySelector("span")).toBe(token);
  expect(layout).not.toHaveBeenCalled();
  textarea.value = '"updated": true';
  textarea.dispatchEvent(new Event("input", { bubbles: true })); await tick();
  expect(highlighted.textContent).toContain('"updated": true');
});
it("keeps wrapped text aligned and cancels pending scroll work on teardown", async () => {
  const { target, textarea, frames, flushFrames } = await render(true);
  textarea.scrollTop = 75; textarea.scrollLeft = 20;
  textarea.dispatchEvent(new Event("scroll")); await flushFrames();
  expect(target.querySelector<HTMLElement>(".code-editor-content")!.scrollTop).toBe(75);
  expect(target.querySelector<HTMLElement>(".code-editor-content")!.scrollLeft).toBe(0);
  textarea.dispatchEvent(new Event("scroll")); expect(frames.size).toBe(1);
  await unmount(instances.pop()!); expect(frames.size).toBe(0);
  textarea.dispatchEvent(new Event("scroll")); expect(frames.size).toBe(0);
});
