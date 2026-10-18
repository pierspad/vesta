// @vitest-environment happy-dom
import { expect, it } from "vitest";
import { mount, tick, unmount } from "svelte";
import FlashcardsWorkspaceHarness from "$lib/test/fixtures/FlashcardsWorkspaceHarness.svelte";
it("keeps files outside the shared scroll area and preserves preview DOM while resizing", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(FlashcardsWorkspaceHarness, { target });
  await tick();
  const files = target.querySelector('[data-testid="pinned-files"]')!;
  const scroller = target.querySelector('[data-testid="settings-scroll"]')!;
  expect(scroller.contains(files)).toBe(false);
  expect(scroller.querySelectorAll('[role="listitem"]')).toHaveLength(4);
  expect(scroller.querySelectorAll('[class*="overflow-y"]')).toHaveLength(0);
  expect(target.textContent).not.toContain("naming");
  const input = target.querySelector<HTMLInputElement>('input[aria-label="Preview position"]')!;
  input.value = "user position";
  target.querySelector<HTMLButtonElement>("button")!.click();
  await tick();
  expect(target.querySelector("input")).toBe(input);
  expect(input.value).toBe("user position");
  expect(target.querySelector("[data-columns]")?.getAttribute("data-columns")).toBe("1");
  await unmount(component);
  target.remove();
});
