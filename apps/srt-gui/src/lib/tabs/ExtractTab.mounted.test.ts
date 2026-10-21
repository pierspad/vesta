// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import ExtractTab from "./ExtractTab.svelte";
const mocks = vi.hoisted(() => ({ invoke: vi.fn(), drop: null as null | ((paths: string[]) => void) }));
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: mocks.invoke }));
vi.mock("$lib/utils/defaultOutputDirectory", () => ({ defaultOutputDirectory: vi.fn().mockResolvedValue("/out") }));
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn() }));
vi.mock("$lib/utils/dragDrop", () => ({ setupWebviewDragDrop: vi.fn(options => { mocks.drop = options.onDrop; return () => {}; }) }));
vi.mock("@tauri-apps/api/path", () => ({ join: vi.fn(async (...parts) => parts.join("/")) }));
const mounted: ReturnType<typeof mount>[] = [];
afterEach(async () => { for (const component of mounted.splice(0)) await unmount(component); document.body.replaceChildren(); vi.clearAllMocks(); });
async function settle() { for (let i = 0; i < 6; i++) { await Promise.resolve(); await tick(); } }
async function render() {
  mocks.invoke.mockResolvedValue([]);
  const target = document.createElement("div"); document.body.append(target);
  mounted.push(mount(ExtractTab, { target, props: { active: true }, intro: false }));
  await settle(); mocks.drop!(["/film.mkv"]); await settle();
  return target;
}
it("ignores an obsolete output lookup failure after a newer lookup succeeds", async () => {
  const target = await render();
  let reject!: (error: unknown) => void;
  mocks.invoke.mockImplementationOnce(() => new Promise((_, fail) => { reject = fail; })).mockResolvedValue([]);
  window.dispatchEvent(new Event("focus")); await settle();
  window.dispatchEvent(new Event("focus")); await settle();
  reject(new Error("obsolete lookup")); await settle();
  expect(target.textContent).not.toContain("obsolete lookup");
});
it("ignores a pending lookup from the previous media file", async () => {
  const target = await render();
  let reject!: (error: unknown) => void;
  mocks.invoke.mockImplementationOnce(() => new Promise((_, fail) => { reject = fail; })).mockResolvedValue([]);
  window.dispatchEvent(new Event("focus")); await settle();
  mocks.drop!(["/new-film.mkv"]); await settle();
  reject(new Error("previous media failure")); await settle();
  expect(target.textContent).not.toContain("previous media failure");
});
it("still displays current lookup errors", async () => {
  const target = await render();
  mocks.invoke.mockRejectedValueOnce(new Error("current lookup failure"));
  window.dispatchEvent(new Event("focus")); await settle();
  expect(target.textContent).toContain("current lookup failure");
});
