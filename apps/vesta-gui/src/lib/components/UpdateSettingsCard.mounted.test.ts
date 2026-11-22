// @vitest-environment happy-dom
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import UpdateSettingsCard from "./UpdateSettingsCard.svelte";

const updates = vi.hoisted(() => ({
  updateStatus: "current", automaticUpdateChecks: true, latestVersion: "v9.0.0", managerHint: "", canInstall: true,
  installing: false, downloadPercent: 0, checkForUpdates: vi.fn(), openRelease: vi.fn(), installUpdate: vi.fn(), onAutomaticUpdateChecksChange: vi.fn(),
}));
vi.mock("$lib/stores/updateCheckerStore.svelte", () => ({ updateCheckerStore: updates }));
let component: ReturnType<typeof mount> | undefined;
beforeEach(() => {
  Object.assign(updates, { updateStatus: "current", managerHint: "", canInstall: true, installing: false });
  vi.clearAllMocks();
});
afterEach(async () => { if (component) await unmount(component); component = undefined; document.body.replaceChildren(); });
async function render() {
  component = mount(UpdateSettingsCard, { target: document.body });
  await tick();
  const buttons = document.querySelectorAll<HTMLButtonElement>('button:not([role="switch"])');
  expect(buttons).toHaveLength(1);
  return buttons[0];
}
it("offers only a manual check when already up to date", async () => {
  const action = await render(); action.click();
  expect(updates.checkForUpdates).toHaveBeenCalledWith("manual");
  expect(document.querySelector('[role="status"]')?.textContent).toContain("Up to date");
});
it("replaces the check action with installation when an installer is available", async () => {
  updates.updateStatus = "available";
  const action = await render(); action.click();
  expect(updates.installUpdate).toHaveBeenCalledOnce();
  expect(updates.checkForUpdates).not.toHaveBeenCalled();
});
it("opens release details and shows manager instructions for managed installations", async () => {
  updates.updateStatus = "available"; updates.managerHint = "Use the package manager";
  const action = await render(); action.click();
  expect(updates.openRelease).toHaveBeenCalledOnce();
  expect(updates.installUpdate).not.toHaveBeenCalled();
  expect(document.body.textContent).toContain(updates.managerHint);
});
it("disables the action while checking or downloading", async () => {
  updates.updateStatus = "checking";
  expect((await render()).disabled).toBe(true);
});
