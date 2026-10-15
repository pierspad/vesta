import { beforeEach, expect, it, vi } from "vitest";
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn() }));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { invokeCommand } from "$lib/services/tauriClient";
import { PreviewStore } from "./previewStore.svelte";
function deferred<T>() { let resolve!: (value: T) => void; let reject!: (error: Error) => void; const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; }
const line = { index: 1, start_ms: 0, end_ms: 1000, subs1_text: "New", subs2_text: null, active: true, leading_context: [], trailing_context: [] };
beforeEach(() => vi.mocked(invokeCommand).mockReset());
it("a stale same-file response cannot overwrite a newer preview or clear its loading flag", async () => {
 const store = new PreviewStore(); const first = deferred<typeof line[]>(); const second = deferred<typeof line[]>();
 vi.mocked(invokeCommand).mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
 const oldCallback = vi.fn(); const newCallback = vi.fn();
 const oldLoad = store.load({ target_subs_path: "film.srt" }, oldCallback);
 const newLoad = store.load({ target_subs_path: "film.srt" }, newCallback);
 first.resolve([{ ...line, subs1_text: "Old" }]); await oldLoad;
 expect(store.loading).toBe(true); expect(oldCallback).not.toHaveBeenCalled();
 second.resolve([line]); await newLoad; expect(store.lines[0].subs1_text).toBe("New"); expect(store.loading).toBe(false); expect(newCallback).toHaveBeenCalledOnce();
});
it("closing the dialog suppresses late loads and late errors", async () => {
 for (const fail of [false, true]) {
  const store = new PreviewStore(); const pending = deferred<typeof line[]>(); vi.mocked(invokeCommand).mockReturnValueOnce(pending.promise);
  const callback = vi.fn(); const load = store.load({ target_subs_path: "film.srt" }, callback); store.close();
  if (fail) pending.reject(new Error("old failure")); else pending.resolve([line]);
  await load; expect(store.visible).toBe(false); expect(store.loading).toBe(false); expect(callback).not.toHaveBeenCalled(); expect(store.lines).toEqual([]);
 }
});
it("failed current preview closes the dialog and returns the error to its caller", async () => {
 const store = new PreviewStore(); vi.mocked(invokeCommand).mockRejectedValueOnce(new Error("bad subtitles"));
 await expect(store.load({ target_subs_path: "film.srt" })).rejects.toThrow("bad subtitles"); expect(store.visible).toBe(false); expect(store.loading).toBe(false);
});
it("closing during transcoding cannot start playback later", async () => {
 const store = new PreviewStore(); const pending = deferred<string>(); const player = { pause: vi.fn(), play: vi.fn(), load: vi.fn(), removeAttribute: vi.fn(), src: "" };
 store.visible = true; store.playerElement = player as unknown as HTMLMediaElement;
 vi.mocked(invokeCommand).mockResolvedValueOnce([8765, "token"]).mockReturnValueOnce(pending.promise);
 const playing = store.playLine(line, "film.mkv"); await Promise.resolve(); store.close(); pending.resolve("prepared.mp4"); await playing;
 expect(player.play).not.toHaveBeenCalled(); expect(player.removeAttribute).toHaveBeenCalledWith("src"); expect(store.playerElement).toBeNull(); expect(store.playingLine).toBeNull();
});
it("closing while media-server discovery is pending prevents late playback", async () => {
 const store = new PreviewStore(); const pending = deferred<[number, string]>(); const player = { pause: vi.fn(), play: vi.fn(), load: vi.fn(), removeAttribute: vi.fn(), src: "" };
 store.visible = true; store.playerElement = player as unknown as HTMLMediaElement;
 vi.mocked(invokeCommand).mockReturnValueOnce(pending.promise);
 const playing = store.playLine(line, "film.mp4"); store.close(); pending.resolve([8765, "token"]); await playing;
 expect(player.play).not.toHaveBeenCalled(); expect(player.src).toBe(""); expect(store.playerElement).toBeNull();
});
it("undo reports whether it consumed a preview edit", () => {
 const store = new PreviewStore(); store.lines = [{ ...line }];
 expect(store.undo()).toBe(false);
 store.toggleLineActive(store.lines[0]);
 expect(store.undo()).toBe(true);
 expect(store.lines[0].active).toBe(true);
});
