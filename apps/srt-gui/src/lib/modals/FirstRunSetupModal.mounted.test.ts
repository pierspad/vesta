// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import FirstRunSetupModal from "./FirstRunSetupModal.svelte";
import { currentLanguage } from "$lib/i18n";

vi.mock("$lib/stores/whisperModelsStore.svelte", () => ({ whisperModelsStore: {
  whisperModels: [{ id: "small", name: "Small", size: "500 MB", downloaded: false }],
  vadModels: [], vadSelection: { customPath: null },
} }));
const mounted: ReturnType<typeof mount>[] = [];
afterEach(async () => {
  for (const component of mounted.splice(0)) await unmount(component);
  document.body.replaceChildren();
});
async function settle() { for (let i = 0; i < 10; i++) { await Promise.resolve(); await tick(); } }
async function render(onClose = vi.fn()) {
  currentLanguage.set("en");
  const target = document.createElement("div");
  document.body.append(target);
  mounted.push(mount(FirstRunSetupModal, { target, props: { preview: true, onComplete: vi.fn(), onClose } }));
  await settle();
  return target;
}
async function click(target: HTMLElement, text: string) {
  const button = [...target.querySelectorAll("button")].find(button => button.textContent?.includes(text));
  expect(button).toBeDefined();
  button!.click();
  await settle();
}
it("installs directly from quick language selection without a review step", async () => {
  const onClose = vi.fn();
  const target = await render(onClose);
  await click(target, "Quick setup");
  expect(target.textContent).not.toContain("Two essential languages");
  expect(target.textContent).not.toContain("Continue");
  await click(target, "Install everything");
  expect(target.textContent).not.toContain("Review what will be installed");
  expect(target.querySelectorAll("progress")).toHaveLength(0);
  expect(target.querySelector("h2")?.textContent).toBe("Vesta is ready.");
  expect(target.textContent).not.toContain("What would you like");
  expect(target.textContent).not.toContain("I have subtitles");
  await click(target, "Start");
  expect(onClose).toHaveBeenCalledOnce();
});
it("keeps the custom configuration and a concise component review", async () => {
  const target = await render();
  await click(target, "Custom setup");
  await click(target, "Continue");
  await click(target, "Continue");
  await click(target, "Continue");
  expect(target.textContent).toContain("Review what will be installed");
  expect(target.textContent).not.toContain("One click downloads");
  expect(target.textContent).not.toContain("Checking installed components");
  expect(target.querySelectorAll(".language-badge")).toHaveLength(2);
  expect(target.querySelectorAll(".component-icon")).toHaveLength(3);
  expect(target.textContent).toContain("Sizes are approximate");
  expect(target.querySelectorAll(".review-preferences dt svg")).toHaveLength(3);
  expect(target.querySelectorAll(".review-preferences dd")[0].textContent).toContain("🇬🇧");
});
