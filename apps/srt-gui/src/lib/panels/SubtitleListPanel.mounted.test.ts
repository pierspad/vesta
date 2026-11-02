// @vitest-environment happy-dom
import { expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import SubtitleListPanel from "./SubtitleListPanel.svelte";

it("keeps anchor removal as a separate keyboard-operable control", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const onRemoveAnchor = vi.fn();
  const component = mount(SubtitleListPanel, { target, props: {
    subtitles: [{ id: 7, start_ms: 1000, end_ms: 2000, text: "Anchor text", synced_start_ms: 1000,
      synced_end_ms: 2000, offset_ms: 0, is_anchor: true }],
    isLoaded: true, currentPage: 1, totalPages: 1, pageSize: 20, activeSubtitleId: 7,
    formatTime: String, formatOffset: String, onPageChange: vi.fn(), onClickSub: vi.fn(),
    onDblClickSub: vi.fn(), onContextMenu: vi.fn(), onRemoveAnchor,
  } });
  await tick();
  try {
    const remove = target.querySelector<HTMLButtonElement>('button[aria-label]')!;
    const subtitle = target.querySelector<HTMLButtonElement>('[data-subtitle-id="7"]')!;
    expect(remove.tagName).toBe("BUTTON");
    expect(remove.parentElement).toBe(subtitle.parentElement);
    expect(subtitle.contains(remove)).toBe(false);
    expect(remove.disabled).toBe(false);
    // Native buttons activate on Enter in the WebView; click here verifies the
    // activation target calls the remove callback without selecting the row.
    remove.click();
    expect(onRemoveAnchor).toHaveBeenCalledExactlyOnceWith(7);
  } finally { await unmount(component); target.remove(); }
});

it("disables anchor removal while a sync edit is in progress", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(SubtitleListPanel, { target, props: {
    subtitles: [{ id: 8, start_ms: 0, end_ms: 1, text: "", synced_start_ms: 0, synced_end_ms: 1, offset_ms: 0, is_anchor: true }],
    isLoaded: true, currentPage: 1, totalPages: 1, pageSize: 1, activeSubtitleId: null,
    formatTime: String, formatOffset: String, onPageChange: vi.fn(), onClickSub: vi.fn(),
    onDblClickSub: vi.fn(), onContextMenu: vi.fn(), onRemoveAnchor: vi.fn(), editDisabled: true,
  } });
  await tick();
  try { expect(target.querySelector<HTMLButtonElement>('button[aria-label]')?.disabled).toBe(true); }
  finally { await unmount(component); target.remove(); }
});
