// @vitest-environment happy-dom
import { afterEach, expect, it } from "vitest";
import { mount, tick, unmount } from "svelte";
import SearchableSelectDisabledHost from "$lib/test/fixtures/SearchableSelectDisabledHost.svelte";

it("closes and removes a portalled menu when the select becomes disabled", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(SearchableSelectDisabledHost, { target });
  try {
    target.querySelector("input")!.focus();
    await tick();
    expect(document.body.querySelector(".searchable-select-dropdown")).not.toBeNull();
    target.querySelector("button")!.click();
    await tick();
    expect(document.body.querySelector(".searchable-select-dropdown")).toBeNull();
  } finally {
    await unmount(component);
    document.body.replaceChildren();
  }
});
