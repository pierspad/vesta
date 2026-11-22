// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import OutputFileActions from "./OutputFileActions.svelte";
const mocks = vi.hoisted(() => ({ invoke: vi.fn(), copy: vi.fn() }));
vi.mock("$lib/config/vestaConfig", () => ({ getItem: () => null, setItem: vi.fn() }));
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: mocks.invoke }));
vi.mock("$lib/utils/clipboard", () => ({ writeClipboard: mocks.copy }));
const instances: ReturnType<typeof mount>[] = [];
afterEach(async () => { for (const instance of instances.splice(0)) await unmount(instance); document.body.replaceChildren(); vi.clearAllMocks(); });
async function settle() { for (let i = 0; i < 3; i++) { await Promise.resolve(); await tick(); } }
async function render(path = "C:\\Film Folder\\cards.apkg") {
  const target = document.createElement("div"); document.body.append(target);
  instances.push(mount(OutputFileActions, { target, props: { path, packageFile: true } })); await tick();
  return { target, buttons: [...target.querySelectorAll<HTMLButtonElement>("button")] };
}
it("separates path copy, opening the containing folder, and opening the APKG", async () => {
  const path = "C:\\Film Folder\\cards.apkg";
  const { buttons } = await render(path);
  expect(buttons[0].textContent).toContain("cards.apkg");
  buttons[0].click(); expect(mocks.copy).toHaveBeenCalledWith(path, expect.any(String));
  expect(mocks.invoke).not.toHaveBeenCalled();
  buttons[1].click(); await settle();
  expect(buttons[2].disabled).toBe(false);
  expect(mocks.invoke).toHaveBeenLastCalledWith("open_output_path", { path, folder: true });
  buttons[2].click(); await settle();
  expect(buttons[1].disabled).toBe(false);
  expect(mocks.invoke).toHaveBeenLastCalledWith("open_output_path", { path, folder: false });
  expect(buttons.every(button => button.type === "button")).toBe(true);
});
it("shows a readable open failure and releases the action buttons", async () => {
  mocks.invoke.mockRejectedValueOnce({ message: "Output no longer exists" });
  const { target, buttons } = await render(); buttons[2].click();
  await Promise.resolve(); await tick();
  expect(target.querySelector('[role="alert"]')?.textContent).toBe("Output no longer exists");
  expect(buttons[1].disabled).toBe(false);
});
