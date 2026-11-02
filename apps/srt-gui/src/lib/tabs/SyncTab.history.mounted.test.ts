// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import SyncTab from "./SyncTab.svelte";
import { autoSyncStore } from "$lib/stores/autoSyncStore.svelte";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  onDrop: undefined as ((paths: string[]) => void) | undefined,
}));

vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: mocks.invoke }));
vi.mock("$lib/utils/dragDrop", () => ({
  setupWebviewDragDrop: vi.fn((options: { onDrop: (paths: string[]) => void }) => {
    mocks.onDrop = options.onDrop;
    return () => { mocks.onDrop = undefined; };
  }),
}));
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn(), guardedSave: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

const status = (canUndo: boolean, canRedo: boolean) => ({
  is_loaded: true,
  srt_path: "film.srt",
  video_path: null,
  total_subtitles: 1,
  anchor_count: canUndo ? 1 : 0,
  checked_count: canUndo ? 1 : 0,
  completion_percentage: canUndo ? 100 : 0,
  average_offset_ms: 0,
  suggested_next_id: 1,
  can_undo: canUndo,
  can_redo: canRedo,
});
const subtitle = {
  id: 1, start_ms: 1000, end_ms: 2000, text: "Hello",
  synced_start_ms: 1000, synced_end_ms: 2000, offset_ms: 0, is_anchor: true,
};
const anchor = { subtitle_id: 1, original_time_ms: 1000, corrected_time_ms: 1000, offset_ms: 0, is_manual: true };
let component: object | undefined;
let target: HTMLDivElement | undefined;
let previousScrollIntoView: typeof HTMLElement.prototype.scrollIntoView;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  target?.remove();
  target = undefined;
  mocks.onDrop = undefined;
  mocks.invoke.mockReset();
  autoSyncStore.operation.busy = false;
  if (previousScrollIntoView) HTMLElement.prototype.scrollIntoView = previousScrollIntoView;
});

async function renderLoaded() {
  mocks.invoke.mockImplementation(async (command: string) => {
    switch (command) {
      case "transcribe_list_models": return [];
      case "sync_load_srt": return status(true, true);
      case "sync_get_subtitles_range": return [subtitle];
      case "sync_get_subtitle": return subtitle;
      case "sync_get_anchors": return [anchor];
      case "sync_suggest_media_for_srt": return null;
      case "sync_undo": return status(false, true);
      case "sync_redo": return status(true, false);
      default: throw new Error(`Unexpected command: ${command}`);
    }
  });
  previousScrollIntoView = HTMLElement.prototype.scrollIntoView;
  HTMLElement.prototype.scrollIntoView = vi.fn();
  target = document.createElement("div");
  document.body.append(target);
  component = mount(SyncTab, { target, props: { active: true }, intro: false });
  await tick();
  mocks.onDrop!("film.srt".split(" "));
  for (let i = 0; i < 8; i++) await tick();
}

const key = (key: string, extra: KeyboardEventInit = {}) => new KeyboardEvent("keydown", {
  key, ctrlKey: true, bubbles: true, cancelable: true, ...extra,
});

it("dispatches sync undo and redo commands instead of removing an anchor directly", async () => {
  await renderLoaded();
  target!.dispatchEvent(key("z"));
  for (let i = 0; i < 5; i++) await tick();
  expect(mocks.invoke.mock.calls.map(([command]) => command)).toContain("sync_undo");
  expect(mocks.invoke.mock.calls.map(([command]) => command)).not.toContain("sync_remove_anchor");
  await new Promise(resolve => setTimeout(resolve, 0));
  await tick();

  target!.dispatchEvent(key("Z", { shiftKey: true }));
  for (let i = 0; i < 10; i++) await tick();
  await new Promise(resolve => setTimeout(resolve, 0));
  expect(mocks.invoke.mock.calls.map(([command]) => command)).toContain("sync_redo");
  expect(mocks.invoke.mock.calls.map(([command]) => command)).not.toContain("sync_remove_anchor");
});

it("leaves textfield Ctrl+Z native and does not dispatch history while auto-sync is busy", async () => {
  await renderLoaded();
  const textfield = document.createElement("input");
  document.body.append(textfield);
  textfield.focus();
  const nativeUndo = key("z");
  textfield.dispatchEvent(nativeUndo);
  expect(nativeUndo.defaultPrevented).toBe(false);
  expect(mocks.invoke.mock.calls.map(([command]) => command)).not.toContain("sync_undo");

  textfield.blur();
  autoSyncStore.operation.busy = true;
  await tick();
  target!.dispatchEvent(key("z"));
  await tick();
  expect(mocks.invoke.mock.calls.map(([command]) => command)).not.toContain("sync_undo");
});
