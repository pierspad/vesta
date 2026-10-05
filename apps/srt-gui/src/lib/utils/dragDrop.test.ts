import { beforeEach, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ register: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: mock.register }) }));
import { setupWebviewDragDrop } from "./dragDrop";
beforeEach(() => { mock.register.mockReset(); vi.stubGlobal("window", { devicePixelRatio: 2 }); });
it("filters inactive tabs and converts native drop coordinates", async () => {
  const unlisten = vi.fn(); mock.register.mockResolvedValue(unlisten);
  let active = false;
  const drop = vi.fn(); const hover = vi.fn();
  const cleanup = setupWebviewDragDrop({ isActive: () => active, onDrop: drop, setDraggingOver: hover });
  const event = mock.register.mock.calls[0][0];
  event({payload: {type: "drop", paths: ["movie.mkv"], position: {x: 200, y: 100}}});
  expect(drop).not.toHaveBeenCalled();
  active = true;
  event({payload: {type: "over"}}); expect(hover).toHaveBeenLastCalledWith(true);
  event({payload: {type: "drop", paths: ["movie.mkv"], position: {x: 200, y: 100}}});
  expect(drop).toHaveBeenCalledWith(["movie.mkv"], {x: 100, y: 50});
  expect(hover).toHaveBeenLastCalledWith(false);
  await Promise.resolve(); cleanup(); expect(unlisten).toHaveBeenCalledOnce();
});
it("unregisters even when the tab closes before registration completes", async () => {
  let resolve!: (cleanup: () => void) => void;
  mock.register.mockReturnValue(new Promise(yes => resolve = yes));
  const cleanup = setupWebviewDragDrop({onDrop: vi.fn(), setDraggingOver: vi.fn()});
  cleanup(); const unlisten = vi.fn(); resolve(unlisten);
  await Promise.resolve(); expect(unlisten).toHaveBeenCalledOnce();
});
