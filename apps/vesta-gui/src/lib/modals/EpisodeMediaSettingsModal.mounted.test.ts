// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import EpisodeMediaSettingsModal from "./EpisodeMediaSettingsModal.svelte";
import { episodeMediaEditorStore as editor } from "$lib/stores/episodeMediaEditorStore.svelte";
import { defaultMediaSettings } from "$lib/utils/mediaSettings";
const instances: ReturnType<typeof mount>[] = [];
afterEach(async () => { for (const instance of instances.splice(0)) await unmount(instance); editor.close(); document.body.replaceChildren(); });
async function render(mediaType: "video" | "audio" = "video") {
  editor.begin(0, { mediaPath: "/film.mkv", mediaType, targetSubsPath: "/film.srt" }, { ...defaultMediaSettings });
  editor.captureBaseline();
  const target = document.createElement("div"); document.body.append(target);
  instances.push(mount(EpisodeMediaSettingsModal, { target, props: { mediaOverrideClass: () => "", onSave: vi.fn(), onReset: vi.fn() } }));
  await tick(); return target;
}
it("exposes switch state and disables video-only settings for audio media", async () => {
  const target = await render("audio");
  const switches = [...target.querySelectorAll<HTMLButtonElement>('[role="switch"]')];
  expect(switches).toHaveLength(3);
  expect(switches.map(button => button.getAttribute("aria-checked"))).toEqual(["true", "false", "false"]);
  expect(switches.map(button => button.disabled)).toEqual([false, true, true]);
  switches[0].click(); await tick();
  expect(editor.overrides?.generateAudio).toBe(false);
  expect(switches[0].getAttribute("aria-checked")).toBe("false");
});
it("closes on Escape from inside the dialog while ignoring composing input", async () => {
  const target = await render();
  expect(target.querySelector('[role="dialog"]')?.getAttribute("aria-label")).toBeTruthy();
  const button = target.querySelector<HTMLButtonElement>("button")!;
  button.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", isComposing: true, bubbles: true, cancelable: true }));
  await tick(); expect(editor.episode).not.toBeNull();
  button.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await tick(); expect(editor.episode).toBeNull();
});
