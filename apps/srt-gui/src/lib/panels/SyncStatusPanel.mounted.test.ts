// @vitest-environment happy-dom
import { expect, it } from "vitest";
import { mount, tick, unmount } from "svelte";
import SyncStatusPanel from "./SyncStatusPanel.svelte";

it("shows offset spread instead of a confidence score for exact anchors", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(SyncStatusPanel, { target, props: {
    status: { is_loaded: true, srt_path: null, video_path: null, total_subtitles: 1755,
      anchor_count: 4, checked_count: 4, completion_percentage: 3, average_offset_ms: 0,
      suggested_next_id: null },
    offsetSpread: 0,
    formatOffset: (ms: number) => `${ms >= 0 ? "+" : ""}${(ms / 1000).toFixed(2)}s`,
  } });
  await tick();
  try {
    expect(target.textContent).toContain("4");
    expect(target.textContent).toContain("0.00s");
    expect(target.textContent).not.toContain("3%");
    expect(target.querySelector('[title]')?.textContent).toContain("0.00s");
  } finally {
    await unmount(component);
    target.remove();
  }
});

it("renders an unavailable spread as a dash", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(SyncStatusPanel, { target, props: { status: null, offsetSpread: null, formatOffset: String } });
  await tick();
  try { expect(target.textContent).toContain("—"); }
  finally { await unmount(component); target.remove(); }
});
